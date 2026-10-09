// SPDX-License-Identifier: Apache-2.0

//! Two-step release orchestration; explicit publication is the human approval boundary.

use crate::quality::gate::QualityGate;
use crate::{
    Command, Path, QualityProfile, approve_quality_release, constants, env, evaluate_quality, fs,
    output, package, project_license, project_slug, quality, quality_evidence, read_workspace_text,
    release_plan, require_main_head_checkout, validate_semver, verify_release_approval, versioning,
    workspace_package_version, workspace_root,
};

/// Tests and packages clean main, committing only optional generated version metadata.
/// No approval, tag, remote write, or GitHub publication occurs in this step.
pub(crate) fn prepare(requested: Option<&str>) -> Result<(), String> {
    let root = workspace_root()?;
    if let Some(paths) = super::version_update::recover(&root)? {
        if git(&root, &["branch", "--show-current"])? != "main" {
            return Err("finish recovered release metadata on main".to_owned());
        }
        let version = workspace_package_version(&read_workspace_text(
            &root,
            constants::WORKSPACE_MANIFEST_FILE,
        )?)?;
        if requested.is_some_and(|requested| requested != version) {
            return Err(format!(
                "finish prepared version {version} before selecting another release"
            ));
        }
        require_only_metadata_changes(&root, &paths)?;
        commit_paths(
            &root,
            &paths.iter().map(String::as_str).collect::<Vec<_>>(),
            &format!("[REL] v{version}"),
        )?;
        super::version_update::acknowledge(&root)?;
    }
    require_main_head_checkout()?;
    let current = workspace_package_version(&read_workspace_text(
        &root,
        constants::WORKSPACE_MANIFEST_FILE,
    )?)?;
    let version = requested.unwrap_or(&current);
    validate_semver(version)?;
    require_local_tag_available(&root, &format!("v{version}"))?;
    let evidence = evidence_directory(version);
    if version != current {
        versioning::prepare_version(version)?;
        commit_paths(
            &root,
            &[
                constants::WORKSPACE_MANIFEST_FILE,
                constants::CARGO_LOCK_FILE,
                &format!("{evidence}/README.md"),
            ],
            &format!("[REL] v{version}"),
        )?;
        super::version_update::acknowledge(&root)?;
    } else if !root.join(&evidence).join("README.md").is_file() {
        fs::create_dir_all(root.join(&evidence)).map_err(|error| error.to_string())?;
        fs::write(
            root.join(&evidence).join("README.md"),
            versioning::release_evidence_readme(version, &project_license(&root)?),
        )
        .map_err(|error| error.to_string())?;
        commit_paths(
            &root,
            &[&format!("{evidence}/README.md")],
            &format!("[REL] v{version}"),
        )?;
    }
    release_plan()?;
    // Fast failures precede costly analysis. Evaluation repeats this same policy
    // with all expensive receipts present and binds success to the exact commit.
    quality(QualityProfile::Pr)?;
    ensure_measurements()?;
    evaluate_quality(QualityProfile::Release)?;
    package()?;
    output::pass(format!(
        "v{version} prepared locally; review artifacts, then run `cargo xtask release publish`"
    ));
    Ok(())
}

/// Checks exact paths, expanding new directories so a recovered scaffold remains committable.
fn require_only_metadata_changes(root: &Path, paths: &[String]) -> Result<(), String> {
    let status = git(root, &["status", "--porcelain", "--untracked-files=all"])?;
    if status.lines().any(|line| {
        !paths
            .iter()
            .any(|path| line.get(3..) == Some(path.as_str()))
    }) {
        return Err(
            "unrelated worktree changes prevent recovered release commit; commit/review them first"
                .to_owned(),
        );
    }
    Ok(())
}

/// Runs only missing/stale measurements, selecting nightly per child without changing global Rust.
pub(crate) fn ensure_measurements() -> Result<(), String> {
    for gate in QualityGate::RELEASE_REQUIRED {
        if quality_evidence::verify_gate(gate).is_ok() {
            output::pass(format!(
                "release measurement {gate}: reusing verified evidence"
            ));
            continue;
        }
        if gate == QualityGate::Advisories {
            quality_evidence::advisory_scan()?;
            continue;
        }
        let arguments = measurement_arguments(gate);
        let executable = env::current_exe().map_err(|error| error.to_string())?;
        let mut child = Command::new(executable);
        child.current_dir(workspace_root()?).args(arguments);
        if gate.requires_nightly() {
            child.env("RUSTUP_TOOLCHAIN", "nightly");
        }
        output::info(format!(
            "release measurement {gate}: running required tools"
        ));
        if !child
            .status()
            .map_err(|error| format!("could not measure {gate}: {error}"))?
            .success()
        {
            return Err(format!(
                "release measurement {gate} failed; fix the reported cause and rerun `cargo xtask release prepare`"
            ));
        }
        quality_evidence::verify_gate(gate)?;
    }
    Ok(())
}

/// Maps each gate to its existing command; no second measurement implementation is maintained.
fn measurement_arguments(gate: QualityGate) -> &'static [&'static str] {
    match gate {
        QualityGate::Coverage => &["coverage"],
        QualityGate::Mutation => &["mutate"],
        QualityGate::Fuzz => &["fuzz", "campaign"],
        QualityGate::PerformanceRelease => &["test", "performance", "--profile", "release"],
        QualityGate::PerformanceSoak => &["test", "performance", "--profile", "soak"],
        QualityGate::Advisories => &[],
    }
}

/// Approves a prepared candidate, signs its tag, and sends main plus exactly that tag atomically.
/// Failures leave local commits/tags available for retry; existing remote tags are never moved.
pub(crate) fn publish() -> Result<(), String> {
    require_main_head_checkout()?;
    let root = workspace_root()?;
    let plan = release_plan()?;
    let version = plan
        .release_tag
        .strip_prefix('v')
        .ok_or("invalid release tag")?;
    require_remote_tag_available(&root, &plan.release_tag)?;
    git(&root, &["fetch", "origin", "main"])?;
    let head = git(&root, &["rev-parse", "HEAD"])?;
    // This rejects a moved local tag before approval creates a new commit.
    if git_exists(
        &root,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/tags/{}", plan.release_tag),
        ],
    )? {
        require_tag_commit(&root, &plan.release_tag, &head)?;
        git(&root, &["verify-tag", &plan.release_tag])?;
    }
    let ancestry = Command::new(constants::GIT_COMMAND)
        .current_dir(&root)
        .args(["merge-base", "--is-ancestor", "FETCH_HEAD", &head])
        .status()
        .map_err(|error| error.to_string())?;
    if !ancestry.success() {
        return Err("origin/main is not an ancestor of local main; integrate remote changes and rerun release prepare".to_owned());
    }
    // Approval itself requires the exact current candidate's passing evaluation.
    let evidence = evidence_directory(version);
    if root.join(&evidence).join("record.toml").is_file() {
        verify_release_approval()?;
        quality_evidence::verify_all()?;
    } else {
        approve_quality_release(version)?;
    }
    commit_paths(
        &root,
        &[
            &format!("{evidence}/record.toml"),
            constants::QUALITY_STATUS_FILE,
        ],
        &format!("[REL] {}", plan.release_tag),
    )?;
    let release_head = require_main_head_checkout()?;
    crate::release_qualify()?;
    if require_main_head_checkout()? != release_head {
        return Err("main changed during release qualification; rerun release prepare".to_owned());
    }
    let tag_ref = format!("refs/tags/{}", plan.release_tag);
    if !git_exists(&root, &["show-ref", "--verify", "--quiet", &tag_ref])? {
        // Signing can require a pinentry prompt, so inherit streams rather than hiding it.
        let status = Command::new(constants::GIT_COMMAND)
            .current_dir(&root)
            .args([
                "tag",
                "-s",
                &plan.release_tag,
                "-m",
                &format!("{} {}", project_slug(&root)?, plan.release_tag),
                &release_head,
            ])
            .status()
            .map_err(|error| error.to_string())?;
        if !status.success() {
            return Err(
                "signed tag creation failed; configure Git signing and retry release publish"
                    .to_owned(),
            );
        }
    }
    require_tag_commit(&root, &plan.release_tag, &release_head)?;
    git(&root, &["verify-tag", &plan.release_tag])?;
    require_remote_tag_available(&root, &plan.release_tag)?;
    let arguments = atomic_push_arguments(&plan.release_tag);
    let arguments = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    git(&root, &arguments)?;
    output::pass(format!(
        "pushed main + {}; GitHub will create a draft release",
        plan.release_tag
    ));
    Ok(())
}

/// Builds explicit non-force refspecs, avoiding unrelated local tags and partial remote updates.
fn atomic_push_arguments(tag: &str) -> Vec<String> {
    vec![
        "push".to_owned(),
        "--atomic".to_owned(),
        "origin".to_owned(),
        "refs/heads/main:refs/heads/main".to_owned(),
        format!("refs/tags/{tag}:refs/tags/{tag}"),
    ]
}

/// Derives the compact evidence location from the authoritative package version.
fn evidence_directory(version: &str) -> String {
    format!("{}/v{version}", constants::QUALITY_EVIDENCE_DIRECTORY)
}

/// Stages only owned metadata paths and makes no commit when they are already unchanged.
fn commit_paths(root: &Path, paths: &[&str], message: &str) -> Result<(), String> {
    let mut arguments = vec!["add", "--"];
    arguments.extend(paths);
    git(root, &arguments)?;
    if !git(root, &["diff", "--cached", "--name-only"])?.is_empty() {
        let mut commit = vec!["commit", "--only", "-m", message, "--"];
        commit.extend(paths);
        git(root, &commit)?;
    }
    Ok(())
}

/// Captures Git output in an explicit repository, preserving useful failure diagnostics.
fn git(root: &Path, arguments: &[&str]) -> Result<String, String> {
    output::invocation(constants::GIT_COMMAND, arguments);
    let result = Command::new(constants::GIT_COMMAND)
        .current_dir(root)
        .args(arguments)
        .output()
        .map_err(|error| format!("Git could not run: {error}"))?;
    if !result.status.success() {
        return Err(format!(
            "git {} failed: {}",
            arguments.join(" "),
            String::from_utf8_lossy(&result.stderr).trim()
        ));
    }
    String::from_utf8(result.stdout)
        .map(|text| text.trim_end().to_owned())
        .map_err(|error| error.to_string())
}

/// Distinguishes an absent local ref from a Git invocation error.
fn git_exists(root: &Path, arguments: &[&str]) -> Result<bool, String> {
    let status = Command::new(constants::GIT_COMMAND)
        .current_dir(root)
        .args(arguments)
        .status()
        .map_err(|error| error.to_string())?;
    match status.code() {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        _ => Err(format!("Git ref lookup failed: {status}")),
    }
}

/// Prevents version preparation from changing metadata for an already tagged release.
fn require_local_tag_available(root: &Path, tag: &str) -> Result<(), String> {
    if git_exists(
        root,
        &[
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/tags/{tag}"),
        ],
    )? {
        return Err(format!(
            "{tag} already exists locally; choose a new version, or retry release publish for that exact tag"
        ));
    }
    Ok(())
}

/// Rejects existing remote tags and transport errors instead of treating either as absence.
fn require_remote_tag_available(root: &Path, tag: &str) -> Result<(), String> {
    let result = Command::new(constants::GIT_COMMAND)
        .current_dir(root)
        .args([
            "ls-remote",
            "--exit-code",
            "--refs",
            "--tags",
            "origin",
            &format!("refs/tags/{tag}"),
        ])
        .output()
        .map_err(|error| error.to_string())?;
    match result.status.code() {
        Some(2) => Ok(()),
        Some(0) => Err(format!(
            "{tag} already exists on origin; release tags are immutable; use a new version"
        )),
        _ => Err(format!(
            "could not check origin tag: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        )),
    }
}

/// Requires any reusable signed local tag to identify the exact qualified source commit.
fn require_tag_commit(root: &Path, tag: &str, expected: &str) -> Result<(), String> {
    if git(root, &["rev-parse", &format!("refs/tags/{tag}^{{commit}}")])? != expected {
        return Err(format!(
            "local {tag} points to another commit; immutable tags cannot be moved"
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../tests/unit/release_lifecycle.rs"]
mod tests;
