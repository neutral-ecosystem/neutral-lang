// SPDX-License-Identifier: Apache-2.0

//! Unit tests for fail-closed release-selection parsing.

use super::{DistributionChannel, ReleasePlan, verify_approval_lineage};
use std::{fs, path::Path, process::Command};

#[test]
/// Approval accepts later fixes in the same history and rejects unrelated or missing commits.
fn release_approval_follows_the_approved_lineage() {
    let root = temporary_plan_path("approval-lineage");
    fs::create_dir(&root).expect("temporary repository must be new");
    repository_git(&root, &["init", "--initial-branch=main"]);
    repository_git(
        &root,
        &["commit", "--allow-empty", "-m", "evaluated candidate"],
    );
    let approved = repository_git(&root, &["rev-parse", "HEAD"]);
    assert!(verify_approval_lineage(&root, &approved, &approved).is_ok());

    repository_git(
        &root,
        &["commit", "--allow-empty", "-m", "approval evidence"],
    );
    let evidence = repository_git(&root, &["rev-parse", "HEAD"]);
    assert!(verify_approval_lineage(&root, &approved, &evidence).is_ok());
    repository_git(
        &root,
        &["commit", "--allow-empty", "-m", "subsequent lint fix"],
    );
    let corrected = repository_git(&root, &["rev-parse", "HEAD"]);
    assert!(verify_approval_lineage(&root, &approved, &corrected).is_ok());
    assert!(verify_approval_lineage(&root, &corrected, &approved).is_err());

    repository_git(&root, &["checkout", "--orphan", "unrelated"]);
    repository_git(
        &root,
        &["commit", "--allow-empty", "-m", "unrelated candidate"],
    );
    let unrelated = repository_git(&root, &["rev-parse", "HEAD"]);
    let error = verify_approval_lineage(&root, &approved, &unrelated)
        .expect_err("an unrelated candidate must not reuse approval");
    assert!(error.contains("does not descend from approved candidate"));
    assert!(verify_approval_lineage(&root, "missing-commit", &corrected).is_err());
    assert!(verify_approval_lineage(&root, &approved, "missing-commit").is_err());
    fs::remove_dir_all(root).expect("owned temporary repository must be removable");
}

/// Executes Git with repository-local test identity and signing disabled.
fn repository_git(root: &Path, arguments: &[&str]) -> String {
    let output = Command::new(super::constants::GIT_COMMAND)
        .current_dir(root)
        .args([
            "-c",
            "user.name=Neutral release test",
            "-c",
            "user.email=release-test@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(arguments)
        .output()
        .expect("Git must execute in the temporary repository");
    assert!(
        output.status.success(),
        "Git {arguments:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("Git test output must be UTF-8")
        .trim()
        .to_owned()
}

#[test]
/// A complete explicit distribution selection is accepted.
fn release_plan_accepts_an_explicit_scope() {
    let path = temporary_plan_path("accepted");
    fs::write(
        &path,
        concat!(
            "schema_version = 1\n[distribution]\n",
            "source_tag = true\n",
            "github_binaries = true\n",
            "crates_io = false\n",
            "binaries = [\"neutral-cli\", \"neutral-probe\"]\n",
        ),
    )
    .expect("temporary release plan should be writable");
    let version = env!("CARGO_PKG_VERSION");
    let plan = ReleasePlan::read(&path, version).expect("explicit release plan should parse");
    assert_eq!(plan.release_tag, format!("v{version}"));
    assert!(plan.channels.contains(&DistributionChannel::SourceTag));
    assert!(plan.channels.contains(&DistributionChannel::GithubBinaries));
    assert!(!plan.channels.contains(&DistributionChannel::CratesIo));
    fs::remove_file(path).expect("temporary release plan should be removable");
}

#[test]
/// An empty distribution selection fails closed.
fn release_plan_rejects_empty_scope() {
    let path = temporary_plan_path("rejected");
    fs::write(
        &path,
        concat!(
            "schema_version = 1\n[distribution]\n",
            "source_tag = false\n",
            "github_binaries = false\n",
            "crates_io = false\n",
            "binaries = []\n",
        ),
    )
    .expect("temporary release plan should be writable");
    assert!(ReleasePlan::read(&path, "0.1.0").is_err());
    fs::remove_file(path).expect("temporary release plan should be removable");
}

#[test]
/// Binary publication cannot bypass the selected immutable source-tag channel.
fn release_plan_rejects_binaries_without_a_source_tag() {
    let path = temporary_plan_path("binary-without-source");
    fs::write(
        &path,
        concat!(
            "schema_version = 1\n[distribution]\n",
            "source_tag = false\n",
            "github_binaries = true\n",
            "crates_io = false\n",
            "binaries = [\"neutral-cli\", \"neutral-probe\"]\n",
        ),
    )
    .expect("temporary release plan should be writable");
    assert!(ReleasePlan::read(&path, "0.1.0").is_err());
    fs::remove_file(path).expect("temporary release plan should be removable");
}

#[test]
/// Duplicate or path-like binary selections cannot become release filenames.
fn release_plan_rejects_unsafe_binary_names() {
    let path = temporary_plan_path("unsafe-binary");
    for binaries in ["[\"neutral-cli\", \"neutral-cli\"]", "[\"../escape\"]"] {
        fs::write(
            &path,
            format!(
                "schema_version = 1\n[distribution]\nsource_tag = true\ngithub_binaries = true\ncrates_io = false\nbinaries = {binaries}\n"
            ),
        )
        .expect("temporary release plan should be writable");
        assert!(ReleasePlan::read(&path, env!("CARGO_PKG_VERSION")).is_err());
    }
    fs::remove_file(path).expect("temporary release plan should be removable");
}

#[test]
/// A distribution block cannot silently inherit values from another TOML section.
fn release_plan_requires_the_selected_schema_and_section() {
    let path = temporary_plan_path("wrong-section");
    fs::write(
        &path,
        "schema_version = 1\n[unrelated]\nsource_tag = true\ngithub_binaries = true\ncrates_io = false\nbinaries = [\"neutral-cli\"]\n",
    )
    .expect("temporary release plan should be writable");
    assert!(ReleasePlan::read(&path, env!("CARGO_PKG_VERSION")).is_err());
    fs::remove_file(path).expect("temporary release plan should be removable");
}

/// Returns a process-unique temporary release-plan path.
fn temporary_plan_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "neutral-release-plan-{label}-{}.toml",
        std::process::id()
    ))
}
