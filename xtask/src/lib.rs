// SPDX-License-Identifier: Apache-2.0

//! Dependency-boundary checks for the Neutral workspace.
//!
//! This automation-only crate checks the resolved Cargo graph rather than
//! trusting package documentation. It is intentionally outside production
//! dependency graphs and does not implement Neutral language behavior.

use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env,
    fmt::Write as _,
    fs::{self, OpenOptions},
    io::Write as _,
    path::{Component, Path, PathBuf},
    process::Command,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

mod configuration;
pub mod constants;
mod environment;
mod fixtures;
mod interface;
mod portable_stage;
mod release;
mod released_conformance;
mod results;
mod versioning;

use configuration::{
    active_test_profile, automation_value, cargo_command, cargo_target_directory,
    configuration_array_from, configuration_section, configuration_value, is_safe_relative_path,
    quality_array, quality_value, repository_directories, rustc_command, test_minimums,
};
use environment::{
    bootstrap, json_string, print_environment_manifest, verify_complete_environment,
    verify_environment,
};
use interface::{
    BuildProfile, CiProfile, FuzzMode, PerformanceProfile, PortableAction, QualityAction,
    QualityProfile, Task, TestLevel, ValidationTarget, VersionAction,
};
use released_conformance::ReleasedBundle;
use results::{clean_results, quality_output_path, result_root};
use versioning::{
    check_versions, collect_named_files, html_spdx_marker, line_spdx_marker, project_license,
    project_slug, sha256_hex, validate_semver, version, workspace_package_directory,
    workspace_package_license, workspace_package_manifests, workspace_package_version,
};

/// One named fallible step in an aggregate automation workflow.
type WorkflowStep<'a> = (&'static str, Box<dyn FnOnce() -> Result<(), String> + 'a>);

/// Source template for the generated workspace rustdoc landing page.
const RUSTDOC_INDEX_TEMPLATE: &str = include_str!("rustdoc-index.html");
/// Shared HTML fragment injected into every generated rustdoc page.
const RUSTDOC_HEADER_TEMPLATE: &str = include_str!("rustdoc-header.html");

/// Runs an `xtask` subcommand.
///
/// # Errors
///
/// Returns an error when the command is invalid or the resolved dependency
/// graph violates the boundary policy.
pub fn run(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    let arguments = arguments.into_iter().collect::<Vec<_>>();
    execute(interface::parse(&arguments)?)
}

/// Executes one parsed stable automation task.
fn execute(task: Task) -> Result<(), String> {
    match task {
        Task::Help => {
            for line in interface::HELP.lines() {
                println!("{} {line}", constants::INFO);
            }
            Ok(())
        }
        Task::Bootstrap => bootstrap(),
        Task::Dev => develop(),
        Task::EnvironmentVerify => verify_complete_environment(),
        Task::EnvironmentManifest => print_environment_manifest(),
        Task::Format { write } => format_workspace(write),
        Task::Lint => lint(),
        Task::Check => check(),
        Task::Build(profile) => build(profile),
        Task::Docs => documentation(),
        Task::Test(level) => test_suite(level),
        Task::Performance(profile) => performance(profile),
        Task::Fuzz(mode) => fuzz(mode),
        Task::Coverage => coverage(),
        Task::Mutate => mutate(),
        Task::Quality(action) => quality_action(action),
        Task::Validate(target) => validate(target),
        Task::Package => package(),
        Task::ReleasePrepare => release_prepare(),
        Task::ReleaseTag => {
            println!("{}", release_plan()?.release_tag);
            Ok(())
        }
        Task::Version(action) => version(action),
        Task::Portable(action) => portable(action),
        Task::Clean => clean_results(),
        Task::Ci(profile) => ci(profile),
        Task::Fixtures { check } => fixtures::sync_fixtures(&workspace_root()?, check),
    }
}

/// Runs the complete auto-formatting workflow intended before ordinary commits.
fn develop() -> Result<(), String> {
    run_recorded_workflow(
        "dev",
        "default",
        vec![
            ("environment", Box::new(verify_environment)),
            ("format", Box::new(|| format_workspace(true))),
            ("check", Box::new(check)),
            ("lint", Box::new(lint)),
            ("tests", Box::new(|| test_suite(TestLevel::All))),
            ("smoke", Box::new(|| test_suite(TestLevel::Smoke))),
            ("docs", Box::new(documentation)),
        ],
    )
}

/// Checks or applies Rust formatting across the complete workspace.
fn format_workspace(write: bool) -> Result<(), String> {
    if write {
        run_cargo(&["fmt", "--all"])
    } else {
        run_cargo(&["fmt", "--all", "--", "--check"])
    }
}

/// Runs warning-free Clippy across every workspace target and feature.
fn lint() -> Result<(), String> {
    run_cargo(&[
        "clippy",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ])
}

/// Runs locked compilation plus repository boundary and coherence checks.
fn check() -> Result<(), String> {
    run_cargo(&["check", "--workspace", "--all-targets", "--locked"])?;
    check_boundaries()?;
    check_test_layout()?;
    check_traceability()?;
    verify_repository_markdown_links()?;
    check_versions()?;
    verify_optional_portable()?;
    check_generated_outputs()?;
    verify_quality_ledger()?;
    check_repository_structure()?;
    check_workflow_contract()
}

/// Verifies every tracked repository-local Markdown link stays within the workspace.
fn verify_repository_markdown_links() -> Result<(), String> {
    let root = workspace_root()?;
    let canonical_root = fs::canonicalize(&root)
        .map_err(|error| format!("could not canonicalize workspace root: {error}"))?;
    let mut files = vec![root.join(constants::ROOT_README_FILE)];
    for directory in repository_directories(&root)? {
        collect_regular_files(&root.join(directory.path), &mut files)?;
    }
    let mut failures = Vec::new();
    for file in files
        .iter()
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md"))
    {
        let content = fs::read_to_string(file)
            .map_err(|error| format!("could not read {}: {error}", file.display()))?;
        for target in markdown_link_targets(&content) {
            let candidate = file.parent().unwrap_or(&root).join(&target);
            let valid = fs::canonicalize(&candidate)
                .is_ok_and(|resolved| resolved.starts_with(&canonical_root));
            if !valid {
                failures.push(format!(
                    "{} -> {target}",
                    file.strip_prefix(&root).unwrap_or(file).display()
                ));
            }
        }
    }
    if failures.is_empty() {
        println!("{} repository Markdown links: pass", constants::INFO);
        Ok(())
    } else {
        failures.sort();
        failures.dedup();
        Err(format!(
            "repository Markdown links are missing or escape the workspace: {}",
            failures.join(", ")
        ))
    }
}

/// Verifies command documentation, platform adapters, and CI stay synchronized.
fn check_workflow_contract() -> Result<(), String> {
    let root = workspace_root()?;
    for relative in ["README.md"] {
        let content = read_workspace_text(&root, relative)?;
        for command in interface::DOCUMENTED_COMMANDS {
            if !content.contains(command) {
                return Err(format!("{relative} does not document `{command}`"));
            }
        }
    }

    let ci = read_workspace_text(&root, ".github/workflows/ci.yml")?;
    if !ci.contains("cargo xtask ci pr") {
        return Err("ci.yml does not delegate to `cargo xtask ci pr`".to_owned());
    }
    let retained_workflows = format!(
        "path: {}/workflows",
        automation_value("output", "results_root")?
    );
    for retention_requirement in [
        "if: always()",
        "actions/upload-artifact@",
        &retained_workflows,
    ] {
        if !ci.contains(retention_requirement) {
            return Err(format!(
                "ci.yml does not retain workflow evidence with `{retention_requirement}`"
            ));
        }
    }
    let release = read_workspace_text(&root, ".github/workflows/release.yml")?;
    if !release.contains("cargo xtask release prepare") {
        return Err("release.yml does not delegate to `cargo xtask release prepare`".to_owned());
    }
    for requirement in [
        "tags: ['v*']",
        "github.event_name == 'push' && github.ref || 'main'",
        "Verify tag identifies main HEAD",
        "git switch -C main",
        "cargo xtask release tag",
        "sha256sum --check SHA256SUMS",
        "contents: write",
        "if: github.event_name == 'push'",
        "gh release create",
        "--verify-tag",
        "--draft",
    ] {
        if !release.contains(requirement) {
            return Err(format!(
                "release.yml does not enforce publication requirement `{requirement}`"
            ));
        }
    }
    if ci.contains("pull_request:") || ci.contains("contents: write") {
        return Err("push CI must not expose release credentials or write permission".to_owned());
    }
    if ci.contains("cargo xtask docs") || ci.contains("cargo xtask quality") {
        return Err("ci.yml must use the single logged CI composition".to_owned());
    }

    for (relative, command) in [
        ("scripts/linux/bootstrap.sh", "xtask bootstrap"),
        ("scripts/linux/environment.sh", "xtask environment"),
        ("scripts/linux/release.sh", "xtask release prepare"),
        ("scripts/win/bootstrap.ps1", "xtask bootstrap"),
        ("scripts/win/environment.ps1", "xtask environment"),
        ("scripts/win/release.ps1", "xtask release prepare"),
    ] {
        if !read_workspace_text(&root, relative)?.contains(command) {
            return Err(format!("{relative} does not delegate to `{command}`"));
        }
    }
    println!("{} stable workflow contract: pass", constants::INFO);
    Ok(())
}

/// Runs the requested durable Cargo build profile.
fn build(profile: BuildProfile) -> Result<(), String> {
    match profile {
        BuildProfile::Dev => run_cargo(&["build", "--workspace", "--locked"]),
        BuildProfile::Release => run_cargo(&["build", "--workspace", "--release", "--locked"]),
    }
}

/// Builds every workspace crate's local API documentation.
fn documentation() -> Result<(), String> {
    run_rustdoc()?;
    let metadata = command_output(
        &cargo_command()?,
        &["metadata", "--format-version", "1", "--no-deps"],
    )?;
    let index = render_rustdoc_index(&metadata)?;
    let output_directory = cargo_target_directory()?.join("doc");
    fs::create_dir_all(&output_directory).map_err(|error| {
        format!(
            "could not create rustdoc directory {}: {error}",
            output_directory.display()
        )
    })?;
    let output_path = output_directory.join(constants::RUSTDOC_INDEX_FILE);
    fs::write(&output_path, index)
        .map_err(|error| format!("could not write {}: {error}", output_path.display()))?;
    copy_documentation_assets(&workspace_root()?, &output_directory)?;
    println!(
        "{} workspace documentation: {}",
        constants::INFO,
        output_path.display()
    );
    Ok(())
}

/// Copies repository-owned brand assets into generated documentation output.
///
/// # Errors
///
/// Returns an error when a required source asset is missing or cannot be copied.
fn copy_documentation_assets(root: &Path, output_directory: &Path) -> Result<(), String> {
    let source_directory = root.join(constants::ASSET_DIRECTORY);
    let destination_directory = output_directory.join(constants::DOCUMENTATION_ASSET_DIRECTORY);
    fs::create_dir_all(&destination_directory).map_err(|error| {
        format!(
            "could not create documentation asset directory {}: {error}",
            destination_directory.display()
        )
    })?;
    for filename in constants::DOCUMENTATION_ASSET_FILES {
        let source = source_directory.join(filename);
        let destination = destination_directory.join(filename);
        if !source.is_file() {
            return Err(format!(
                "required documentation asset is missing: {}",
                source.display()
            ));
        }
        fs::copy(&source, &destination).map_err(|error| {
            format!(
                "could not copy documentation asset {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })?;
    }
    Ok(())
}

/// Runs workspace rustdoc with the shared navigation header on every HTML page.
fn run_rustdoc() -> Result<(), String> {
    let header_path = workspace_root()?.join(constants::RUSTDOC_HEADER_FILE);
    if !header_path.is_file() {
        return Err(format!(
            "rustdoc navigation header is missing: {}",
            header_path.display()
        ));
    }
    let header_path = header_path
        .to_str()
        .ok_or_else(|| "rustdoc navigation header path is not valid UTF-8".to_owned())?;
    let mut rustdoc_flags = env::var(constants::CARGO_ENCODED_RUSTDOCFLAGS).unwrap_or_default();
    if !rustdoc_flags.is_empty() {
        rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    }
    rustdoc_flags.push_str(constants::RUSTDOC_HTML_HEADER_FLAG);
    rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    rustdoc_flags.push_str(header_path);
    rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    rustdoc_flags.push_str("-D");
    rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    rustdoc_flags.push_str("warnings");
    rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    rustdoc_flags.push_str("--cfg");
    rustdoc_flags.push(constants::RUSTDOC_FLAG_SEPARATOR);
    rustdoc_flags.push_str(&rustdoc_header_configuration());

    let arguments = ["doc", "--workspace", "--no-deps"];
    let cargo = cargo_command()?;
    let status = Command::new(&cargo)
        .current_dir(workspace_root()?)
        .env(constants::CARGO_ENCODED_RUSTDOCFLAGS, rustdoc_flags)
        .args(arguments)
        .status()
        .map_err(|error| format!("could not run {} {}: {error}", cargo, arguments.join(" ")))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{} {} failed with {status}", cargo, arguments.join(" ")))
}

/// Derives a stable non-security cache token from the shared rustdoc header.
fn rustdoc_header_configuration() -> String {
    let mut hash = constants::RUSTDOC_HEADER_HASH_OFFSET;
    for byte in RUSTDOC_HEADER_TEMPLATE.bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(constants::RUSTDOC_HEADER_HASH_PRIME);
    }
    format!("{}_{hash:016x}", constants::RUSTDOC_HEADER_CFG_PREFIX)
}

/// Embeds Cargo metadata safely into the workspace rustdoc index template.
fn render_rustdoc_index(metadata: &str) -> Result<String, String> {
    if !RUSTDOC_INDEX_TEMPLATE.contains(constants::CARGO_METADATA_PLACEHOLDER) {
        return Err("rustdoc index template has no Cargo metadata placeholder".to_owned());
    }
    let safe_metadata = metadata.replace('<', "\\u003c");
    Ok(RUSTDOC_INDEX_TEMPLATE.replacen(constants::CARGO_METADATA_PLACEHOLDER, &safe_metadata, 1))
}

/// Runs one independently selectable, stage-free test level.
fn test_suite(level: TestLevel) -> Result<(), String> {
    match level {
        TestLevel::All => {
            run_cargo(&["test", "--workspace", "--lib", "--bins", "--tests"])?;
            verify_test_counts(active_test_profile())
        }
        TestLevel::Unit => run_cargo(&["test", "--workspace", "--lib", "--bins"]),
        TestLevel::Smoke => run_shell_smoke(),
        TestLevel::Integration => run_active_test_filter("integration"),
        TestLevel::System => run_active_test_filter("system"),
        TestLevel::Conformance => run_active_test_filter("conformance"),
        TestLevel::Property => run_active_test_filter("property"),
        TestLevel::Security => run_active_test_filter("security"),
    }
}

/// Runs one active cross-package suite by stable test-name prefix.
fn run_active_test_filter(suite: &str) -> Result<(), String> {
    run_cargo(&[
        "test",
        "--workspace",
        "--lib",
        "--bins",
        "--tests",
        "--",
        &format!("{suite}_"),
    ])
}

/// Runs the selected durable fuzz mode.
fn fuzz(mode: FuzzMode) -> Result<(), String> {
    match mode {
        FuzzMode::Smoke => run_active_test_filter("fuzz_smoke"),
        FuzzMode::Campaign => coverage_guided_fuzz_campaign(),
    }
}

/// Runs every configured coverage-guided fuzz target for its approved budget.
fn coverage_guided_fuzz_campaign() -> Result<(), String> {
    let targets = quality_array("fuzz", "targets")?;
    let seconds = quality_value("fuzz", "minimum_seconds_per_target")?;
    for target in targets {
        run_cargo(&[
            "fuzz",
            "run",
            &target,
            "--",
            &format!("-max_total_time={seconds}"),
        ])?;
    }
    Ok(())
}

/// Runs workspace coverage and enforces every configured percentage threshold.
fn coverage() -> Result<(), String> {
    let lines = quality_value("coverage", "minimum_line_percent")?;
    let functions = quality_value("coverage", "minimum_function_percent")?;
    let regions = quality_value("coverage", "minimum_region_percent")?;
    let exclusions = quality_value("coverage", "exclusion_regex")?;
    let html_index = quality_output_path("coverage", "html_output")?;
    let html = html_index
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "coverage HTML output must be below a report directory".to_owned())?;
    let json = quality_output_path("coverage", "json_output")?;
    fs::create_dir_all(
        json.parent()
            .ok_or_else(|| "coverage JSON output has no parent".to_owned())?,
    )
    .map_err(|error| format!("could not create coverage result directory: {error}"))?;
    let html = html
        .to_str()
        .ok_or_else(|| "coverage HTML path is not valid UTF-8".to_owned())?;
    let json = json
        .to_str()
        .ok_or_else(|| "coverage JSON path is not valid UTF-8".to_owned())?;
    run_cargo(&["llvm-cov", "--workspace", "--all-targets", "--no-report"])?;
    run_cargo(&[
        "llvm-cov",
        "report",
        "--html",
        "--output-dir",
        html,
        "--ignore-filename-regex",
        &exclusions,
        "--fail-under-lines",
        &lines,
        "--fail-under-functions",
        &functions,
        "--fail-under-regions",
        &regions,
    ])?;
    run_cargo(&[
        "llvm-cov",
        "report",
        "--json",
        "--summary-only",
        "--output-path",
        json,
        "--ignore-filename-regex",
        &exclusions,
        "--fail-under-lines",
        &lines,
        "--fail-under-functions",
        &functions,
        "--fail-under-regions",
        &regions,
    ])?;
    println!(
        "{} coverage HTML: {}",
        constants::INFO,
        html_index.display()
    );
    println!("{} coverage JSON: {json}", constants::INFO);
    Ok(())
}

/// Runs mutation analysis for the configured critical production target.
fn mutate() -> Result<(), String> {
    let target = quality_value("mutation", "critical_target")?;
    run_cargo(&["mutants", "--file", &target])
}

/// Dispatches one managed quality workflow action.
fn quality_action(action: QualityAction) -> Result<(), String> {
    match action {
        QualityAction::Run(profile) => quality(profile),
        QualityAction::Status => quality_status(),
        QualityAction::Evaluate(profile) => evaluate_quality(profile),
        QualityAction::Approve(release) => approve_quality_release(&release),
        QualityAction::Render => render_quality_status(),
        QualityAction::Verify => verify_quality_ledger(),
    }
}

/// Runs the documented aggregate quality composition for one durable profile.
fn quality(profile: QualityProfile) -> Result<(), String> {
    let profile_name = quality_profile_name(profile);
    let mut steps: Vec<WorkflowStep<'_>> = vec![
        ("format", Box::new(|| format_workspace(false))),
        ("check", Box::new(check)),
        ("lint", Box::new(lint)),
        ("tests", Box::new(|| test_suite(TestLevel::All))),
        ("smoke", Box::new(|| test_suite(TestLevel::Smoke))),
        (
            "probe-build",
            Box::new(|| run_cargo(&["build", "--locked", "--package", constants::NEUTRAL_PROBE])),
        ),
        ("docs", Box::new(documentation)),
    ];
    if profile == QualityProfile::Release {
        steps.extend([
            (
                "recorded-quality-gates",
                Box::new(verify_recorded_quality_gates) as Box<dyn FnOnce() -> Result<(), String>>,
            ),
            ("release-build", Box::new(|| build(BuildProfile::Release))),
            (
                "binary-validation",
                Box::new(|| validate(ValidationTarget::Binaries)),
            ),
        ]);
    }
    run_recorded_workflow("quality", profile_name, steps)
}

/// One immutable release-quality record.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd)]
struct QualityApproval {
    /// Release identifier without a `v` prefix.
    release: String,
    /// Exact approved Git commit.
    commit: String,
    /// Approval state.
    status: String,
    /// UTC-independent Unix timestamp or retained historical date.
    approved_at: String,
    /// Evaluation identifier that supported the approval.
    evaluation: String,
    /// SHA-256 of the quality-gate configuration used for evaluation.
    quality_gates_sha256: String,
}

/// Runs a quality profile and writes a deterministic commit-bound evaluation.
fn evaluate_quality(profile: QualityProfile) -> Result<(), String> {
    let commit = require_clean_checkout()?;
    quality(profile)?;
    let root = workspace_root()?;
    let profile_name = quality_profile_name(profile);
    let output = result_root()?
        .join(constants::QUALITY_EVALUATION_DIRECTORY)
        .join(&commit)
        .join(format!("{profile_name}.toml"));
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    let manifest_sha256 = sha256_file(&root.join(constants::QUALITY_MANIFEST_FILE))?;
    let quality_gates_sha256 = sha256_file(&root.join(constants::QUALITY_GATES_FILE))?;
    let toolchain = command_output(&rustc_command()?, &["--version"])?;
    let license_marker = line_spdx_marker(&project_license(&root)?);
    fs::write(
        &output,
        format!(
            "{license_marker}\n\nschema_version = 1\ncommit = \"{}\"\nprofile = \"{profile_name}\"\nstatus = \"pass\"\ntoolchain = \"{}\"\nquality_manifest_sha256 = \"{manifest_sha256}\"\nquality_gates_sha256 = \"{quality_gates_sha256}\"\n",
            json_string(&commit),
            json_string(&toolchain)
        ),
    )
    .map_err(|error| format!("could not write {}: {error}", output.display()))?;
    println!(
        "{} quality evaluation: {}",
        constants::INFO,
        output.display()
    );
    Ok(())
}

/// Approves a passing release evaluation for the clean checked-out `main` HEAD.
fn approve_quality_release(release: &str) -> Result<(), String> {
    let version = release.strip_prefix('v').unwrap_or(release);
    validate_semver(version)?;
    let root = workspace_root()?;
    let workspace_version = workspace_package_version(&read_workspace_text(
        &root,
        constants::WORKSPACE_MANIFEST_FILE,
    )?)?;
    if version != workspace_version {
        return Err(format!(
            "quality approval release {version} does not match workspace version {workspace_version}"
        ));
    }
    let commit = require_main_head_checkout()?;
    let evaluation_relative = PathBuf::from(constants::QUALITY_EVALUATION_DIRECTORY)
        .join(&commit)
        .join("release.toml");
    let evaluation = result_root()?.join(&evaluation_relative);
    let evaluation_content = fs::read_to_string(&evaluation).map_err(|error| {
        format!(
            "passing release evaluation is missing at {}: {error}; run `cargo xtask quality evaluate --profile release`",
            evaluation.display()
        )
    })?;
    if configuration_value(&evaluation_content, "commit").as_deref() != Some(commit.as_str())
        || configuration_value(&evaluation_content, "profile").as_deref() != Some("release")
        || configuration_value(&evaluation_content, "status").as_deref() != Some("pass")
    {
        return Err("release evaluation does not identify a passing current HEAD".to_owned());
    }
    let evaluated_manifest = configuration_value(&evaluation_content, "quality_manifest_sha256")
        .ok_or_else(|| "release evaluation has no quality-manifest digest".to_owned())?;
    let evaluated_gates = configuration_value(&evaluation_content, "quality_gates_sha256")
        .ok_or_else(|| "release evaluation has no quality-gate digest".to_owned())?;
    if evaluated_manifest != sha256_file(&root.join(constants::QUALITY_MANIFEST_FILE))?
        || evaluated_gates != sha256_file(&root.join(constants::QUALITY_GATES_FILE))?
    {
        return Err(
            "quality policy changed after evaluation; rerun the release evaluation".to_owned(),
        );
    }
    let evidence_directory = root
        .join(constants::QUALITY_EVIDENCE_DIRECTORY)
        .join(format!("v{version}"));
    if !evidence_directory.join("README.md").is_file() {
        return Err(format!(
            "release evidence directory is not prepared: {}",
            evidence_directory.display()
        ));
    }
    let approved_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system clock precedes Unix epoch: {error}"))?
        .as_secs();
    let record = evidence_directory.join("record.toml");
    if record.exists() {
        return Err(format!(
            "quality approval already exists and is immutable: {}",
            record.display()
        ));
    }
    let quality_gates_sha256 = evaluated_gates;
    let license_marker = line_spdx_marker(&project_license(&root)?);
    fs::write(
        &record,
        format!(
            "{license_marker}\n\nschema_version = 1\nrelease = \"v{version}\"\ncommit = \"{commit}\"\nstatus = \"approved\"\napproved_at = \"{approved_at}\"\nevaluation = \"{}\"\nquality_gates_sha256 = \"{quality_gates_sha256}\"\n",
            json_string(&evaluation_relative.to_string_lossy())
        ),
    )
    .map_err(|error| format!("could not write {}: {error}", record.display()))?;
    render_quality_status()?;
    println!("{} quality release v{version}: approved", constants::INFO);
    Ok(())
}

/// Prints the current approved-release quality ledger.
fn quality_status() -> Result<(), String> {
    let approvals = read_quality_approvals()?;
    if approvals.is_empty() {
        println!("{} no approved quality releases", constants::INFO);
    }
    for approval in approvals {
        println!(
            "{} {} {}",
            constants::INFO,
            approval.release,
            approval.status
        );
    }
    Ok(())
}

/// Regenerates the checked-in human-readable quality status document.
fn render_quality_status() -> Result<(), String> {
    let approvals = read_quality_approvals()?;
    let root = workspace_root()?;
    let rendered = quality_status_markdown(&approvals, &project_license(&root)?);
    let path = root.join(constants::QUALITY_STATUS_FILE);
    fs::write(&path, rendered)
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    println!(
        "{} quality status rendered: {}",
        constants::INFO,
        path.display()
    );
    Ok(())
}

/// Verifies release records and generated status documentation.
fn verify_quality_ledger() -> Result<(), String> {
    check_quality_inventory()?;
    let root = workspace_root()?;
    let approvals = read_quality_approvals()?;
    for approval in &approvals {
        if approval.status != "approved"
            || approval.approved_at.is_empty()
            || approval.evaluation.is_empty()
            || !is_sha256(&approval.quality_gates_sha256)
            || approval.commit.len() != 40
            || !approval
                .commit
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            return Err(format!(
                "quality approval v{} is incomplete or invalid",
                approval.release
            ));
        }
        let version = approval.release.strip_prefix('v').ok_or_else(|| {
            format!(
                "quality approval release must start with v: {}",
                approval.release
            )
        })?;
        validate_semver(version)?;
    }
    let expected = quality_status_markdown(&approvals, &project_license(&root)?);
    let status = read_workspace_text(&root, constants::QUALITY_STATUS_FILE)?;
    if status != expected {
        return Err(
            "quality status documentation is stale; run `cargo xtask quality render`".to_owned(),
        );
    }
    println!("{} quality approval ledger: pass", constants::INFO);
    Ok(())
}

/// Returns the stable command spelling for one quality profile.
fn quality_profile_name(profile: QualityProfile) -> &'static str {
    match profile {
        QualityProfile::Pr => "pr",
        QualityProfile::Release => "release",
    }
}

/// Requires a clean checkout and returns its exact current Git commit.
fn require_clean_checkout() -> Result<String, String> {
    let commit = command_output(constants::GIT_COMMAND, &["rev-parse", "HEAD"])?;
    let status = command_output(constants::GIT_COMMAND, &["status", "--porcelain"])?;
    if !status.is_empty() {
        return Err("quality evaluation requires a clean worktree".to_owned());
    }
    Ok(commit)
}

/// Returns the SHA-256 digest of one exact file.
fn sha256_file(path: &Path) -> Result<String, String> {
    fs::read(path)
        .map(|bytes| sha256_hex(&bytes))
        .map_err(|error| format!("could not read {}: {error}", path.display()))
}

/// Returns whether text is one lowercase SHA-256 hexadecimal digest.
fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase())
}

/// Reads every immutable release-quality record in release order.
fn read_quality_approvals() -> Result<Vec<QualityApproval>, String> {
    let root = workspace_root()?;
    let mut records = Vec::new();
    collect_named_files(
        &root.join(constants::QUALITY_EVIDENCE_DIRECTORY),
        "record.toml",
        &mut records,
    )?;
    let mut approvals = Vec::new();
    for path in records {
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let field = |name: &str| {
            configuration_value(&content, name)
                .ok_or_else(|| format!("quality approval {} has no {name}", path.display()))
        };
        let approval = QualityApproval {
            release: field("release")?,
            commit: field("commit")?,
            status: field("status")?,
            approved_at: field("approved_at")?,
            evaluation: field("evaluation")?,
            quality_gates_sha256: field("quality_gates_sha256")?,
        };
        let directory_release = path
            .parent()
            .and_then(Path::file_name)
            .and_then(std::ffi::OsStr::to_str)
            .ok_or_else(|| {
                format!(
                    "quality approval has no release directory: {}",
                    path.display()
                )
            })?;
        if approval.release != directory_release {
            return Err(format!(
                "quality approval release {} does not match directory {directory_release}",
                approval.release
            ));
        }
        approvals.push(approval);
    }
    approvals.sort();
    Ok(approvals)
}

/// Renders the deterministic human-readable release-quality table.
fn quality_status_markdown(approvals: &[QualityApproval], license: &str) -> String {
    let mut output = format!(
        "{}\n<!-- Generated by `cargo xtask quality render`; do not edit. -->\n\n# Release quality status\n\n| Release | Status | Recorded at |\n| --- | --- | --- |\n",
        html_spdx_marker(license)
    );
    for approval in approvals {
        writeln!(
            &mut output,
            "| `v{}` | {} | `{}` |",
            approval
                .release
                .strip_prefix('v')
                .unwrap_or(&approval.release),
            approval.status,
            approval.approved_at
        )
        .expect("writing to a String cannot fail");
    }
    output.push_str("\nRelease records are verified by `cargo xtask quality verify`.\n");
    output
}

/// Verifies that every configured expensive quality gate has retained pass evidence.
fn verify_recorded_quality_gates() -> Result<(), String> {
    for (section, accepted) in [
        ("coverage", "pass"),
        ("mutation", "pass"),
        ("fuzz", "full-campaign-pass"),
        ("performance", "pass-local-profiled-runner"),
    ] {
        let actual = quality_value(section, "status")?;
        if actual != accepted {
            return Err(format!(
                "quality [{section}] status must be {accepted}; found {actual}"
            ));
        }
    }
    Ok(())
}

/// Validates either built release binaries or one encoded artifact.
fn validate(target: ValidationTarget) -> Result<(), String> {
    match target {
        ValidationTarget::Binaries => validate_release_binaries(
            &cargo_target_directory()?.join("release"),
            &release_plan()?.binaries,
        ),
        ValidationTarget::Artifact(path) => {
            if !path.is_file() {
                return Err(format!("artifact does not exist: {}", path.display()));
            }
            let probe = release_binary_path(
                &cargo_target_directory()?.join("release"),
                constants::NEUTRAL_PROBE,
            );
            if !probe.is_file() {
                return Err(format!(
                    "release probe is missing: {}; run `cargo xtask build --profile release`",
                    probe.display()
                ));
            }
            run_program(&probe, &[path.as_os_str()])
        }
    }
}

/// Validates release-mode CLI and probe entry points plus the probe boundary.
fn validate_release_binaries(directory: &Path, binaries: &[String]) -> Result<(), String> {
    for binary in binaries {
        let path = release_binary_path(directory, binary);
        if !path.is_file() {
            return Err(format!(
                "release binary is missing: {}; run `cargo xtask build --profile release`",
                path.display()
            ));
        }
        run_program(&path, &[std::ffi::OsStr::new("--help")])?;
    }
    check_boundaries()?;
    println!("{} release binaries: valid", constants::INFO);
    Ok(())
}

/// Returns a platform-correct release binary path.
fn release_binary_path(directory: &Path, binary: &str) -> PathBuf {
    let extension = if cfg!(windows) { ".exe" } else { "" };
    directory.join(format!("{binary}{extension}"))
}

/// Runs one already-resolved executable with inherited standard streams.
fn run_program(program: &Path, arguments: &[&std::ffi::OsStr]) -> Result<(), String> {
    let status = Command::new(program)
        .current_dir(workspace_root()?)
        .args(arguments)
        .status()
        .map_err(|error| format!("could not run {}: {error}", program.display()))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{} failed with {status}", program.display()))
}

/// Reads and verifies the explicit release-authority selection for a `main`-head candidate.
fn release_plan() -> Result<release::ReleasePlan, String> {
    let root = workspace_root()?;
    let package_version = workspace_package_version(&read_workspace_text(
        &root,
        constants::WORKSPACE_MANIFEST_FILE,
    )?)?;
    let plan =
        release::ReleasePlan::read(&root.join(constants::RELEASE_CONFIG_FILE), &package_version)?;
    let residual_risk_status = quality_document_status(&root, constants::RESIDUAL_RISKS_FILE)?;
    if !matches!(
        residual_risk_status.as_str(),
        "approved" | "approved-with-limitation"
    ) {
        return Err(format!(
            "residual-risk review is not approved in the quality manifest; found {residual_risk_status}"
        ));
    }
    if plan
        .channels
        .contains(&release::DistributionChannel::GithubBinaries)
    {
        let expected = BTreeSet::from([
            constants::NEUTRAL_CLI.to_owned(),
            constants::NEUTRAL_PROBE.to_owned(),
        ]);
        let selected = plan.binaries.iter().cloned().collect::<BTreeSet<_>>();
        if selected != expected {
            return Err(format!(
                "GitHub binary scope must select exactly {expected:?}; found {selected:?}"
            ));
        }
    }
    Ok(plan)
}

/// Reads the lifecycle status of one durable document from the quality manifest.
fn quality_document_status(root: &Path, document: &str) -> Result<String, String> {
    let manifest = read_workspace_text(root, constants::QUALITY_MANIFEST_FILE)?;
    for entry in manifest.split("[[document]]").skip(1) {
        if configuration_value(entry, "path").as_deref() == Some(document) {
            return configuration_value(entry, "status")
                .ok_or_else(|| format!("quality document {document} has no status"));
        }
    }
    Err(format!(
        "quality document is not registered in the manifest: {document}"
    ))
}

/// Requires a clean checked-out `main` and returns its exact current candidate commit.
fn require_main_head_checkout() -> Result<String, String> {
    let branch = command_output("git", &["branch", "--show-current"])?;
    if branch != "main" {
        return Err(format!(
            "release qualification requires checked-out main; found {branch:?}"
        ));
    }
    let head = command_output("git", &["rev-parse", "HEAD"])?;
    let status = command_output("git", &["status", "--porcelain"])?;
    if !status.is_empty() {
        return Err("release qualification requires a clean main worktree".to_owned());
    }
    Ok(head)
}

/// Requires an approved release candidate followed only by its evidence commit.
fn verify_release_approval() -> Result<(), String> {
    let plan = release_plan()?;
    let approvals = read_quality_approvals()?;
    let approval = approvals
        .iter()
        .find(|record| record.release == plan.release_tag)
        .ok_or_else(|| {
            format!(
                "release {} has no quality approval record",
                plan.release_tag
            )
        })?;
    if approval.status != "approved" {
        return Err(format!(
            "release {} quality approval is not approved",
            plan.release_tag
        ));
    }
    let root = workspace_root()?;
    if approval.quality_gates_sha256 != sha256_file(&root.join(constants::QUALITY_GATES_FILE))? {
        return Err("quality gates changed after release approval".to_owned());
    }
    let parent = command_output("git", &["rev-parse", "HEAD^"])?;
    if parent != approval.commit {
        return Err(format!(
            "release {} must use the approval-evidence commit immediately after its evaluated candidate {}",
            plan.release_tag, approval.commit
        ));
    }
    let changed = command_output("git", &["diff", "--name-only", &approval.commit, "HEAD"])?;
    let expected = BTreeSet::from([
        format!(
            "{}/{}/record.toml",
            constants::QUALITY_EVIDENCE_DIRECTORY,
            plan.release_tag
        ),
        constants::QUALITY_STATUS_FILE.to_owned(),
    ]);
    let actual = changed.lines().map(str::to_owned).collect::<BTreeSet<_>>();
    if actual != expected {
        return Err(format!(
            "approval commit must change only {expected:?}; found {actual:?}"
        ));
    }
    Ok(())
}

/// One immutable generated distribution file.
struct DistributionAsset {
    /// Plain filename beneath the release package directory.
    filename: String,
    /// Exact file bytes.
    bytes: Vec<u8>,
}

/// Shared provenance fields applied to every release-manifest artifact entry.
struct ReleaseEntryContext<'a> {
    /// Workspace package version that produced the artifact.
    version: &'a str,
    /// Exact source commit from which the artifact was produced.
    candidate_commit: &'a str,
    /// Project license expression inherited from the workspace manifest.
    license: &'a str,
}

/// Assembles the selected binary distribution into the ignored release root.
fn package() -> Result<(), String> {
    let plan = release_plan()?;
    let candidate_commit = require_main_head_checkout()?;
    if !plan
        .channels
        .contains(&release::DistributionChannel::GithubBinaries)
    {
        return Err("GitHub binary distribution is not selected".to_owned());
    }
    for binary in &plan.binaries {
        run_cargo(&["build", "--release", "--locked", "--package", binary])?;
    }
    let root = workspace_root()?;
    let host = rust_host()?;
    let source_directory = cargo_target_directory()?.join("release");
    validate_release_binaries(&source_directory, &plan.binaries)?;
    let output_directory = result_root()?
        .join(constants::RELEASE_RESULT_DIRECTORY)
        .join("package")
        .join(&plan.release_tag)
        .join(&candidate_commit)
        .join(&host);
    let summary = format!(
        "{{\"release_tag\":\"{}\",\"candidate_ref\":\"main\",\"candidate_commit\":\"{}\",\"host\":\"{}\",\"channel\":\"github-binaries\",\"status\":\"assembled\"}}\n",
        json_string(&plan.release_tag),
        json_string(&candidate_commit),
        json_string(&host)
    );
    let assets = release_distribution_assets(
        &root,
        &source_directory,
        &plan,
        &candidate_commit,
        &host,
        &summary,
    )?;
    stage_binary_package(
        &root,
        &source_directory,
        &output_directory,
        &plan.binaries,
        &assets,
        &summary,
    )?;
    println!(
        "{} package assembled: {}",
        constants::INFO,
        output_directory.display()
    );
    Ok(())
}

/// Generates the source archive, SBOM, provenance, installation, manifest, and checksums.
fn release_distribution_assets(
    root: &Path,
    source_directory: &Path,
    plan: &release::ReleasePlan,
    candidate_commit: &str,
    host: &str,
    package_summary: &str,
) -> Result<Vec<DistributionAsset>, String> {
    let version = plan.release_tag.trim_start_matches('v');
    let license = project_license(root)?;
    let artifact_name = project_slug(root)?;
    let license_marker = html_spdx_marker(&license);
    let entry_context = ReleaseEntryContext {
        version,
        candidate_commit,
        license: &license,
    };
    let source_name = format!("{artifact_name}-{}-source.tar", plan.release_tag);
    let lock_bytes = fs::read(root.join(constants::CARGO_LOCK_FILE))
        .map_err(|error| format!("could not read release dependency lock: {error}"))?;
    let mut assets = vec![
        DistributionAsset {
            filename: source_name,
            bytes: source_archive(root, &artifact_name, version, candidate_commit)?,
        },
        DistributionAsset {
            filename: constants::RELEASE_SBOM_FILE.to_owned(),
            bytes: lock_bytes.clone(),
        },
        DistributionAsset {
            filename: constants::RELEASE_INSTALL_FILE.to_owned(),
            bytes: release_install_guide(
                &license_marker,
                &artifact_name,
                version,
                host,
                &plan.binaries,
            ),
        },
        DistributionAsset {
            filename: constants::RELEASE_PROVENANCE_FILE.to_owned(),
            bytes: format!("{{\"schema_version\":1,\"builder\":\"cargo xtask package\",\"candidate_ref\":\"main\",\"candidate_commit\":\"{}\",\"release_tag\":\"{}\",\"target\":\"{}\",\"rustc\":\"{}\",\"cargo_lock_sha256\":\"{}\",\"reproducible_command\":\"cargo xtask package\"}}\n", json_string(candidate_commit), json_string(&plan.release_tag), json_string(host), json_string(&command_output(&rustc_command()?, &["--version"])?), sha256_hex(&lock_bytes)).into_bytes(),
        },
    ];
    let mut entries = Vec::new();
    let mut checksums = Vec::new();
    append_binary_release_entries(
        &mut entries,
        &mut checksums,
        source_directory,
        &plan.binaries,
        &entry_context,
    )?;
    for asset in &assets {
        let channel = if asset.filename.ends_with("-source.tar") {
            "source-tag"
        } else {
            "github-release-metadata"
        };
        append_release_entry(
            &mut entries,
            &mut checksums,
            &asset.filename,
            &asset.bytes,
            channel,
            &entry_context,
        );
    }
    for (filename, bytes) in [
        (
            constants::LICENSE_FILE,
            fs::read(root.join(constants::LICENSE_FILE))
                .map_err(|error| format!("could not hash LICENSE: {error}"))?,
        ),
        (
            constants::ROOT_README_FILE,
            fs::read(root.join(constants::ROOT_README_FILE))
                .map_err(|error| format!("could not hash README: {error}"))?,
        ),
        ("package-summary.json", package_summary.as_bytes().to_vec()),
    ] {
        append_release_entry(
            &mut entries,
            &mut checksums,
            filename,
            &bytes,
            "github-release-metadata",
            &entry_context,
        );
    }
    let manifest = release_manifest_bytes(plan, &entry_context, host, &entries);
    checksums.push(format!(
        "{}  {}\n",
        sha256_hex(&manifest),
        constants::RELEASE_MANIFEST_FILE
    ));
    assets.push(DistributionAsset {
        filename: constants::RELEASE_MANIFEST_FILE.to_owned(),
        bytes: manifest,
    });
    checksums.sort();
    assets.push(DistributionAsset {
        filename: constants::RELEASE_CHECKSUM_FILE.to_owned(),
        bytes: checksums.concat().into_bytes(),
    });
    Ok(assets)
}

/// Hashes the selected built binaries into one release manifest and checksum set.
fn append_binary_release_entries(
    entries: &mut Vec<String>,
    checksums: &mut Vec<String>,
    source_directory: &Path,
    binaries: &[String],
    context: &ReleaseEntryContext<'_>,
) -> Result<(), String> {
    for binary in binaries {
        let filename = release_binary_path(Path::new(""), binary)
            .to_string_lossy()
            .into_owned();
        let bytes = fs::read(release_binary_path(source_directory, binary))
            .map_err(|error| format!("could not hash release binary {binary}: {error}"))?;
        append_release_entry(
            entries,
            checksums,
            &filename,
            &bytes,
            "github-binaries",
            context,
        );
    }
    Ok(())
}

/// Renders the installation instructions for the exact selected binary set.
fn release_install_guide(
    license_marker: &str,
    artifact_name: &str,
    version: &str,
    host: &str,
    binaries: &[String],
) -> Vec<u8> {
    let programs = binaries
        .iter()
        .map(|binary| format!("`{binary}`"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{license_marker}\n\n# Install {artifact_name} {version}\n\nSupported target: `{host}`. Verify the downloaded files with `sha256sum --check SHA256SUMS`, install {programs} into a directory on `PATH`, then run each installed binary with `--help`.\n"
    )
    .into_bytes()
}

/// Renders metadata from the selected channels and assembled artifact records.
fn release_manifest_bytes(
    plan: &release::ReleasePlan,
    context: &ReleaseEntryContext<'_>,
    host: &str,
    entries: &[String],
) -> Vec<u8> {
    let crates_io_selected = plan
        .channels
        .contains(&release::DistributionChannel::CratesIo);
    let deferred = if crates_io_selected {
        "[\"additional host targets\"]"
    } else {
        "[\"additional host targets\",\"crates.io publication\"]"
    };
    format!("{{\"schema_version\":1,\"release_tag\":\"{}\",\"candidate_ref\":\"main\",\"candidate_commit\":\"{}\",\"license\":\"{}\",\"supported_targets\":[\"{}\"],\"crates_io_selected\":{crates_io_selected},\"known_limitations\":[\"single-host binary package\",\"no runtime or application semantics\"],\"deferred\":{deferred},\"artifacts\":[{}]}}\n", json_string(&plan.release_tag), json_string(context.candidate_commit), json_string(context.license), json_string(host), entries.join(",")).into_bytes()
}

/// Produces deterministic tracked source bytes for one exact candidate commit.
fn source_archive(
    root: &Path,
    artifact_name: &str,
    version: &str,
    candidate_commit: &str,
) -> Result<Vec<u8>, String> {
    let archive = Command::new("git")
        .current_dir(root)
        .args([
            "archive",
            "--format=tar",
            &format!("--prefix={artifact_name}-{version}/"),
            candidate_commit,
        ])
        .output()
        .map_err(|error| format!("could not create source archive: {error}"))?;
    if archive.status.success() {
        Ok(archive.stdout)
    } else {
        Err(format!(
            "git archive failed: {}",
            String::from_utf8_lossy(&archive.stderr).trim()
        ))
    }
}

/// Adds one selected file to the release manifest and checksum list.
fn append_release_entry(
    entries: &mut Vec<String>,
    checksums: &mut Vec<String>,
    filename: &str,
    bytes: &[u8],
    channel: &str,
    context: &ReleaseEntryContext<'_>,
) {
    let digest = sha256_hex(bytes);
    checksums.push(format!("{digest}  {filename}\n"));
    entries.push(format!("{{\"filename\":\"{}\",\"sha256\":\"{}\",\"license\":\"{}\",\"producer_version\":\"{}\",\"source_commit\":\"{}\",\"channel\":\"{}\"}}", json_string(filename), digest, json_string(context.license), json_string(context.version), json_string(context.candidate_commit), json_string(channel)));
}

/// Atomically stages selected binaries, license material, and package metadata.
fn stage_binary_package(
    root: &Path,
    source_directory: &Path,
    output_directory: &Path,
    binaries: &[String],
    assets: &[DistributionAsset],
    summary: &str,
) -> Result<(), String> {
    if output_directory.exists() {
        return verify_existing_binary_package(
            root,
            source_directory,
            output_directory,
            binaries,
            assets,
            summary,
        );
    }
    let parent = output_directory
        .parent()
        .ok_or_else(|| "package output has no parent directory".to_owned())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    let partial_directory = parent.join(format!(".partial-{}", std::process::id()));
    if partial_directory.exists() {
        return Err(format!(
            "partial package output already exists: {}; run `cargo xtask clean`",
            partial_directory.display()
        ));
    }
    fs::create_dir(&partial_directory)
        .map_err(|error| format!("could not create {}: {error}", partial_directory.display()))?;
    for binary in binaries {
        let source = release_binary_path(source_directory, binary);
        let destination = release_binary_path(&partial_directory, binary);
        fs::copy(&source, &destination).map_err(|error| {
            format!(
                "could not copy {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })?;
    }
    for file in [constants::LICENSE_FILE, constants::ROOT_README_FILE] {
        fs::copy(root.join(file), partial_directory.join(file))
            .map_err(|error| format!("could not stage {file}: {error}"))?;
    }
    for asset in assets {
        fs::write(partial_directory.join(&asset.filename), &asset.bytes)
            .map_err(|error| format!("could not stage {}: {error}", asset.filename))?;
    }
    fs::write(partial_directory.join("package-summary.json"), summary)
        .map_err(|error| format!("could not write package summary: {error}"))?;
    fs::rename(&partial_directory, output_directory).map_err(|error| {
        format!(
            "could not publish staged package {} as {}: {error}",
            partial_directory.display(),
            output_directory.display()
        )
    })?;
    Ok(())
}

/// Reuses an existing package only when its exact file set and bytes still match.
fn verify_existing_binary_package(
    root: &Path,
    source_directory: &Path,
    output_directory: &Path,
    binaries: &[String],
    assets: &[DistributionAsset],
    summary: &str,
) -> Result<(), String> {
    let output_type = fs::symlink_metadata(output_directory)
        .map_err(|error| format!("could not inspect {}: {error}", output_directory.display()))?
        .file_type();
    if !output_type.is_dir() {
        return Err(format!(
            "existing package output is not a regular directory: {}",
            output_directory.display()
        ));
    }
    let mut expected = BTreeMap::new();
    for binary in binaries {
        let filename = release_binary_path(Path::new(""), binary)
            .to_string_lossy()
            .into_owned();
        let bytes = fs::read(release_binary_path(source_directory, binary))
            .map_err(|error| format!("could not read release binary {binary}: {error}"))?;
        if expected.insert(filename, bytes).is_some() {
            return Err(format!("duplicate release binary: {binary}"));
        }
    }
    for filename in [constants::LICENSE_FILE, constants::ROOT_README_FILE] {
        let bytes = fs::read(root.join(filename))
            .map_err(|error| format!("could not read release {filename}: {error}"))?;
        expected.insert(filename.to_owned(), bytes);
    }
    for asset in assets {
        if expected
            .insert(asset.filename.clone(), asset.bytes.clone())
            .is_some()
        {
            return Err(format!("duplicate release asset: {}", asset.filename));
        }
    }
    expected.insert(
        "package-summary.json".to_owned(),
        summary.as_bytes().to_vec(),
    );

    for entry in fs::read_dir(output_directory)
        .map_err(|error| format!("could not inspect {}: {error}", output_directory.display()))?
    {
        let entry = entry.map_err(|error| format!("could not inspect package entry: {error}"))?;
        let filename = entry.file_name().to_string_lossy().into_owned();
        if !entry
            .file_type()
            .map_err(|error| format!("could not inspect package {filename}: {error}"))?
            .is_file()
        {
            return Err(format!("package entry is not a regular file: {filename}"));
        }
        let expected_bytes = expected
            .remove(&filename)
            .ok_or_else(|| format!("unexpected package file: {filename}"))?;
        let actual_bytes = fs::read(entry.path())
            .map_err(|error| format!("could not read package file {filename}: {error}"))?;
        if actual_bytes != expected_bytes {
            return Err(format!(
                "existing package file differs from current build: {filename}"
            ));
        }
    }
    if !expected.is_empty() {
        return Err(format!(
            "existing package is incomplete: {}",
            expected.keys().cloned().collect::<Vec<_>>().join(", ")
        ));
    }
    println!(
        "{} existing package verified: {}",
        constants::INFO,
        output_directory.display()
    );
    Ok(())
}

/// Runs release checks and assembles artifacts without tagging or publishing.
fn release_prepare() -> Result<(), String> {
    run_recorded_workflow(
        "release",
        "prepare",
        vec![
            ("release-plan", Box::new(|| release_plan().map(drop))),
            (
                "main-head",
                Box::new(|| require_main_head_checkout().map(drop)),
            ),
            ("approval", Box::new(verify_release_approval)),
            ("quality", Box::new(|| quality(QualityProfile::Release))),
            ("package", Box::new(package)),
        ],
    )?;
    let plan = release_plan()?;
    println!(
        "{} release {} prepared; no publish action was performed",
        constants::INFO,
        plan.release_tag
    );
    Ok(())
}

/// Returns the host triple reported by the selected Rust compiler.
fn rust_host() -> Result<String, String> {
    let verbose = command_output(&rustc_command()?, &["-vV"])?;
    verbose
        .lines()
        .find_map(|line| line.strip_prefix("host: "))
        .map(str::to_owned)
        .ok_or_else(|| "rustc -vV did not report a host triple".to_owned())
}

/// Executes the active portable lifecycle command family.
fn portable(action: PortableAction) -> Result<(), String> {
    match action {
        PortableAction::Install(source) => install_portable(&source),
        PortableAction::Verify => verify_portable(),
        PortableAction::Snapshot => snapshot_portable(),
    }
}

/// Atomically installs and verifies one reviewed active portable package.
fn install_portable(source: &Path) -> Result<(), String> {
    let root = workspace_root()?;
    let destination = root.join(constants::PORTABLE_DIRECTORY);
    if destination.exists() {
        return Err(format!(
            "active portable destination already exists: {}",
            destination.display()
        ));
    }
    let source = fs::canonicalize(source).map_err(|error| {
        format!(
            "could not resolve portable source {}: {error}",
            source.display()
        )
    })?;
    if !source.is_dir() {
        return Err(format!(
            "portable source is not a directory: {}",
            source.display()
        ));
    }
    let staging = root.join(format!(".portable.install-{}", std::process::id()));
    if staging.exists() {
        return Err(format!(
            "portable installation staging path already exists: {}",
            staging.display()
        ));
    }
    if let Err(error) = copy_portable_tree(&source, &staging) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    fs::rename(&staging, &destination).map_err(|error| {
        format!(
            "could not publish portable package {} as {}: {error}",
            staging.display(),
            destination.display()
        )
    })?;
    if let Err(error) = verify_portable() {
        let rejected_root = result_root()?.join("portable/rejected");
        let rejected_run = match unique_generated_directory(&rejected_root) {
            Ok(directory) => directory,
            Err(move_error) => {
                fs::rename(&destination, &staging).map_err(|restore_error| {
                    format!(
                        "{error}; could not allocate rejected-package storage: {move_error}; could not retain the package at {}: {restore_error}",
                        staging.display()
                    )
                })?;
                return Err(format!(
                    "{error}; could not allocate rejected-package storage: {move_error}; rejected package retained at {}",
                    staging.display()
                ));
            }
        };
        let rejected = rejected_run.join(constants::PORTABLE_DIRECTORY);
        fs::rename(&destination, &rejected).map_err(|move_error| {
            format!(
                "{error}; could not move rejected package {} to {}: {move_error}",
                destination.display(),
                rejected.display()
            )
        })?;
        return Err(format!(
            "{error}; rejected package retained at {}",
            rejected.display()
        ));
    }
    println!(
        "{} active portable installed from {}",
        constants::INFO,
        source.display()
    );
    Ok(())
}

/// Copies a portable directory without following symbolic links or special files.
fn copy_portable_tree(source: &Path, destination: &Path) -> Result<(), String> {
    fs::create_dir(destination)
        .map_err(|error| format!("could not create {}: {error}", destination.display()))?;
    for entry in fs::read_dir(source)
        .map_err(|error| format!("could not inspect {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("could not inspect portable entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("could not inspect {}: {error}", entry.path().display()))?;
        let target = destination.join(entry.file_name());
        if file_type.is_dir() {
            copy_portable_tree(&entry.path(), &target)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), &target).map_err(|error| {
                format!(
                    "could not copy {} to {}: {error}",
                    entry.path().display(),
                    target.display()
                )
            })?;
        } else {
            return Err(format!(
                "portable source contains a symbolic link or special file: {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

/// Verifies the active portable package's required roots and traceability.
fn verify_portable() -> Result<(), String> {
    let root = workspace_root()?;
    verify_portable_layout(&root)?;
    let lifecycle = read_workspace_text(&root, constants::PORTABLE_LIFECYCLE_FILE)?;
    let active_series = configuration_value(&lifecycle, "active_series")
        .ok_or_else(|| "active portable lifecycle has no active_series".to_owned())?;
    let status = configuration_value(&lifecycle, "status")
        .ok_or_else(|| "active portable lifecycle has no status".to_owned())?;
    if status != "active" {
        return Err(format!(
            "installed portable status must be active; found {status:?}"
        ));
    }
    if !is_portable_series(&active_series) {
        return Err(format!(
            "active portable series must be a numeric v-prefixed identifier; found {active_series:?}"
        ));
    }
    check_portable_traceability_at(&root)?;
    verify_frozen_input_digests(&root, constants::PORTABLE_CONTRACT_FREEZE_FILE)?;
    verify_portable_links(&root)?;
    ensure_no_portable_archive_dependencies(&root)?;
    verify_existing_portable_snapshots(&root)?;
    println!("{} active portable package: pass", constants::INFO);
    Ok(())
}

/// Verifies the version-independent files and directories required from every portable package.
fn verify_portable_layout(root: &Path) -> Result<(), String> {
    for relative in [
        constants::PORTABLE_PLAN_FILE,
        constants::PORTABLE_LIFECYCLE_FILE,
        constants::PORTABLE_REQUIREMENTS_FILE,
        constants::PORTABLE_TRACEABILITY_FILE,
        constants::PORTABLE_CONTRACT_FREEZE_FILE,
        constants::PORTABLE_CONFORMANCE_MANIFEST_FILE,
    ] {
        if !root.join(relative).is_file() {
            return Err(format!("active portable file is missing: {relative}"));
        }
        if read_workspace_text(root, relative)?.trim().is_empty() {
            return Err(format!("active portable file is empty: {relative}"));
        }
    }
    for relative in [
        constants::PORTABLE_CONTRACT_DIRECTORY,
        constants::PORTABLE_FIXTURE_DIRECTORY,
        constants::PORTABLE_ORACLE_DIRECTORY,
    ] {
        if !root.join(relative).is_dir() {
            return Err(format!("active portable directory is missing: {relative}"));
        }
    }
    Ok(())
}

/// Returns whether a portable series is a nonempty numeric `v` identifier.
fn is_portable_series(value: &str) -> bool {
    value.strip_prefix('v').is_some_and(|digits| {
        !digits.is_empty() && digits.chars().all(|character| character.is_ascii_digit())
    })
}

/// Verifies an active portable package when one is installed for development.
fn verify_optional_portable() -> Result<(), String> {
    let root = workspace_root()?;
    if root.join(constants::PORTABLE_DIRECTORY).exists() {
        verify_portable()
    } else {
        println!(
            "{} no active portable package installed; release conformance remains available",
            constants::INFO
        );
        Ok(())
    }
}

/// Verifies every locally retained digest-addressed portable snapshot.
fn verify_existing_portable_snapshots(root: &Path) -> Result<(), String> {
    let snapshot_root = result_root()?.join(constants::PORTABLE_SNAPSHOT_DIRECTORY);
    if !snapshot_root.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(&snapshot_root)
        .map_err(|error| format!("could not inspect {}: {error}", snapshot_root.display()))?
    {
        let entry = entry.map_err(|error| format!("could not inspect snapshot entry: {error}"))?;
        if !entry
            .file_type()
            .map_err(|error| format!("could not inspect {}: {error}", entry.path().display()))?
            .is_dir()
            || entry.file_name().to_string_lossy().starts_with('.')
        {
            continue;
        }
        verify_portable_snapshot_directory(root, &entry.path())?;
    }
    Ok(())
}

/// Verifies one snapshot manifest, directory digest, and copied file set.
fn verify_portable_snapshot_directory(root: &Path, directory: &Path) -> Result<(), String> {
    let manifest = fs::read_to_string(directory.join("manifest.sha256"))
        .map_err(|error| format!("could not read snapshot manifest: {error}"))?;
    let expected_license_marker = line_spdx_marker(&project_license(root)?);
    if !manifest.starts_with(&expected_license_marker) {
        return Err("portable snapshot manifest has the wrong license marker".to_owned());
    }
    let mut records = String::new();
    for line in manifest
        .lines()
        .filter(|line| !line.starts_with('#') && !line.is_empty())
    {
        writeln!(&mut records, "{line}").expect("writing to a String cannot fail");
    }
    let expected_tree = directory
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .ok_or_else(|| "portable snapshot directory has no UTF-8 digest name".to_owned())?;
    if sha256_hex(records.as_bytes()) != expected_tree {
        return Err(format!(
            "portable snapshot directory {expected_tree} does not match its manifest"
        ));
    }
    for record in records.lines() {
        let mut fields = record.splitn(3, "  ");
        let expected_digest = fields
            .next()
            .ok_or_else(|| format!("invalid portable snapshot record: {record}"))?;
        let expected_size = fields
            .next()
            .ok_or_else(|| format!("invalid portable snapshot record: {record}"))?
            .parse::<usize>()
            .map_err(|error| format!("invalid portable snapshot size: {error}"))?;
        let relative = fields
            .next()
            .ok_or_else(|| format!("invalid portable snapshot record: {record}"))?;
        let path = directory.join("content").join(relative);
        let bytes = fs::read(&path)
            .map_err(|error| format!("could not read snapshot file {}: {error}", path.display()))?;
        if bytes.len() != expected_size || sha256_hex(&bytes) != expected_digest {
            return Err(format!(
                "portable snapshot file differs from manifest: {}",
                path.strip_prefix(root).unwrap_or(&path).display()
            ));
        }
    }
    Ok(())
}

/// Verifies every frozen input path and any digest retained beside it.
///
/// Active single-maintainer plans may track the input by path alone. Released
/// freezes can retain a matching `*_sha256` field for immutable archival
/// verification.
fn verify_frozen_input_digests(root: &Path, freeze_file: &str) -> Result<(), String> {
    let freeze = read_workspace_text(root, freeze_file)?;
    let values = configuration_section(&freeze, "fixture_corpus")?
        .into_iter()
        .collect::<BTreeMap<_, _>>();
    let frozen_paths = values
        .iter()
        .filter_map(|(key, path)| {
            key.strip_suffix("_path")
                .map(|name| (name.to_owned(), path.to_owned()))
        })
        .collect::<Vec<_>>();
    if frozen_paths.is_empty() {
        return Err("contract freeze has no frozen input paths".to_owned());
    }
    for (name, path) in frozen_paths {
        let digest_key = format!("{name}_sha256");
        let relative = Path::new(&path);
        if relative.as_os_str().is_empty()
            || !relative
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
        {
            return Err(format!(
                "frozen input path must be a safe workspace-relative path: {path}"
            ));
        }
        let bytes = fs::read(root.join(relative))
            .map_err(|error| format!("could not read frozen input {path}: {error}"))?;
        if let Some(expected) = values.get(&digest_key) {
            if !is_sha256(expected) {
                return Err(format!("contract freeze has an invalid {digest_key}"));
            }
            let actual = sha256_hex(&bytes);
            if actual != expected.as_str() {
                return Err(format!(
                    "frozen input {path} has SHA-256 {actual}, expected {expected}; contract review is required"
                ));
            }
        }
    }
    Ok(())
}

/// Verifies repository-local links in every active portable Markdown file.
fn verify_portable_links(root: &Path) -> Result<(), String> {
    let portable_root = root.join(constants::PORTABLE_DIRECTORY);
    let canonical_root = fs::canonicalize(root)
        .map_err(|error| format!("could not canonicalize workspace root: {error}"))?;
    let mut files = Vec::new();
    collect_regular_files(&portable_root, &mut files)?;
    let mut missing = Vec::new();
    for file in files
        .iter()
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md"))
    {
        let content = fs::read_to_string(file)
            .map_err(|error| format!("could not read {}: {error}", file.display()))?;
        for target in markdown_link_targets(&content) {
            let candidate = file.parent().unwrap_or(&portable_root).join(&target);
            let valid = fs::canonicalize(&candidate)
                .is_ok_and(|candidate| candidate.starts_with(&canonical_root));
            if !valid {
                missing.push(format!(
                    "{} -> {target}",
                    file.strip_prefix(root).unwrap_or(file).display()
                ));
            }
        }
    }
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "active portable links are missing: {}",
            missing.join(", ")
        ))
    }
}

/// Extracts local, anchor-free Markdown link paths from document text.
fn markdown_link_targets(content: &str) -> Vec<String> {
    content
        .split("](")
        .skip(1)
        .filter_map(|tail| tail.split_once(')').map(|(target, _)| target))
        .map(str::trim)
        .map(|target| target.trim_matches(['<', '>']))
        .filter(|target| {
            !target.is_empty()
                && !target.starts_with('#')
                && !target.contains("://")
                && !target.starts_with("mailto:")
        })
        .map(|target| target.split('#').next().unwrap_or(target))
        .filter(|target| !target.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Rejects production or executable-test references to archived portable inputs.
fn ensure_no_portable_archive_dependencies(root: &Path) -> Result<(), String> {
    let mut files = Vec::new();
    let mut production_roots = Vec::new();
    for manifest in workspace_package_manifests(root)? {
        let package_root = manifest
            .parent()
            .ok_or_else(|| format!("workspace manifest has no parent: {}", manifest.display()))?;
        collect_regular_files(package_root, &mut files)?;
        let content = fs::read_to_string(&manifest)
            .map_err(|error| format!("could not read {}: {error}", manifest.display()))?;
        if configuration::quality_value_from(&content, "package", "name").as_deref()
            != Some(constants::XTASK)
        {
            production_roots.push(package_root.to_path_buf());
        }
    }
    let forbidden = [
        concat!("portable", "/archive/"),
        concat!("portable", "/archived/"),
    ];
    let violations = files
        .iter()
        .filter(|path| {
            matches!(
                path.extension().and_then(|value| value.to_str()),
                Some("rs" | "toml")
            )
        })
        .filter_map(|path| {
            let content = fs::read_to_string(path).ok()?;
            let uses_active_portable = production_roots
                .iter()
                .any(|package_root| path.starts_with(package_root))
                && content.contains("portable/");
            (uses_active_portable || forbidden.iter().any(|value| content.contains(value))).then(
                || {
                    path.strip_prefix(root)
                        .unwrap_or(path)
                        .display()
                        .to_string()
                },
            )
        })
        .collect::<Vec<_>>();
    if violations.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "code depends on archived portable material: {}",
            violations.join(", ")
        ))
    }
}

/// Creates an atomic digest-addressed copy of the active portable package.
fn snapshot_portable() -> Result<(), String> {
    verify_portable()?;
    let root = workspace_root()?;
    let portable_root = root.join(constants::PORTABLE_DIRECTORY);
    let mut files = Vec::new();
    collect_regular_files(&portable_root, &mut files)?;
    files.sort();
    let records = portable_digest_records(&root, &files)?;
    let tree_digest = sha256_hex(records.as_bytes());
    let parent = result_root()?.join(constants::PORTABLE_SNAPSHOT_DIRECTORY);
    let output = parent.join(&tree_digest);
    let license_marker = line_spdx_marker(&project_license(&root)?);
    let manifest =
        format!("{license_marker}\n# sha256  bytes  repository-relative-path\n{records}");
    if output.exists() {
        let existing = fs::read_to_string(output.join("manifest.sha256"))
            .map_err(|error| format!("could not read existing snapshot manifest: {error}"))?;
        if existing != manifest {
            return Err(format!(
                "portable snapshot collision at {}",
                output.display()
            ));
        }
        println!(
            "{} portable snapshot: {}",
            constants::INFO,
            output.display()
        );
        return Ok(());
    }
    fs::create_dir_all(&parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    let partial = parent.join(format!(".{tree_digest}.partial-{}", std::process::id()));
    fs::create_dir(&partial)
        .map_err(|error| format!("could not create {}: {error}", partial.display()))?;
    let content_root = partial.join("content").join(constants::PORTABLE_DIRECTORY);
    for source in &files {
        let relative = source
            .strip_prefix(&portable_root)
            .map_err(|error| format!("portable path escaped its root: {error}"))?;
        let destination = content_root.join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
        }
        fs::copy(source, &destination).map_err(|error| {
            format!(
                "could not copy {} to {}: {error}",
                source.display(),
                destination.display()
            )
        })?;
    }
    fs::write(partial.join("manifest.sha256"), &manifest)
        .map_err(|error| format!("could not write portable snapshot manifest: {error}"))?;
    let lifecycle = read_workspace_text(&root, constants::PORTABLE_LIFECYCLE_FILE)?;
    let report = format!(
        "{{\"schema_version\":1,\"tree_sha256\":\"{tree_digest}\",\"file_count\":{},\"next_series\":\"{}\",\"status\":\"review-required\"}}\n",
        files.len(),
        json_string(&configuration_value(&lifecycle, "next_series").unwrap_or_default())
    );
    fs::write(partial.join("migration-report.json"), report)
        .map_err(|error| format!("could not write portable migration report: {error}"))?;
    fs::rename(&partial, &output).map_err(|error| {
        format!(
            "could not publish portable snapshot {} as {}: {error}",
            partial.display(),
            output.display()
        )
    })?;
    println!(
        "{} portable snapshot: {}",
        constants::INFO,
        output.display()
    );
    Ok(())
}

/// Builds sorted exact-byte digest records for active portable files.
fn portable_digest_records(root: &Path, files: &[PathBuf]) -> Result<String, String> {
    let mut records = String::new();
    for path in files {
        let bytes = fs::read(path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        let relative = path
            .strip_prefix(root)
            .map_err(|error| format!("portable path escaped workspace: {error}"))?
            .to_string_lossy()
            .replace('\\', "/");
        writeln!(
            &mut records,
            "{}  {}  {relative}",
            sha256_hex(&bytes),
            bytes.len()
        )
        .expect("writing to a String cannot fail");
    }
    Ok(records)
}

/// Verifies generated-output ownership and ignored tracking policy.
fn check_generated_outputs() -> Result<(), String> {
    let root = workspace_root()?;
    let inventory = read_workspace_text(&root, constants::GENERATED_OUTPUTS_FILE)?;
    let result_directory = automation_value("output", "results_root")?;
    if !is_safe_relative_path(Path::new(&result_directory)) {
        return Err("configured generated-results root is not a safe relative path".to_owned());
    }
    for name in [
        "rustdoc",
        "coverage",
        "fuzz",
        "mutation",
        "benchmark",
        "package",
        "release",
        "workflow-runs",
        "portable-snapshot",
        "portable-rejected",
        "quality-evaluation",
    ] {
        if !inventory.contains(&format!("name = \"{name}\"")) {
            return Err(format!("generated-output inventory is missing {name}"));
        }
    }
    for entry in inventory.split("[[output]]").skip(1) {
        let path = configuration_value(entry, "path")
            .ok_or_else(|| "generated output has no path".to_owned())?;
        let tracking = configuration_value(entry, "tracking");
        let ephemeral_output = (path.starts_with("target/")
            || path.starts_with(&format!("{result_directory}/")))
            && tracking.as_deref() == Some("ignored");
        if !ephemeral_output
            || configuration_value(entry, "owner").is_none()
            || configuration_value(entry, "generate").is_none()
            || configuration_value(entry, "validate").is_none()
        {
            return Err(format!(
                "generated output {path} has an unsafe or incomplete policy"
            ));
        }
    }
    let ignore = read_workspace_text(&root, ".gitignore")?;
    if !ignore.lines().any(|line| line.trim() == "target")
        || !ignore
            .lines()
            .any(|line| line.trim() == format!("{result_directory}/"))
    {
        return Err("Cargo target and configured results must remain ignored".to_owned());
    }
    println!("{} generated-output ownership: pass", constants::INFO);
    Ok(())
}

/// Verifies the categorized inventory of durable quality documents.
fn check_quality_inventory() -> Result<(), String> {
    let root = workspace_root()?;
    let expected_license_marker = html_spdx_marker(&project_license(&root)?);
    let manifest = read_workspace_text(&root, constants::QUALITY_MANIFEST_FILE)?;
    let mut registered = BTreeSet::new();
    for entry in manifest.split("[[document]]").skip(1) {
        let path = configuration_value(entry, "path")
            .ok_or_else(|| "quality document has no path".to_owned())?;
        let kind = configuration_value(entry, "kind")
            .ok_or_else(|| format!("quality document {path} has no kind"))?;
        let owner = configuration_value(entry, "owner")
            .ok_or_else(|| format!("quality document {path} has no owner"))?;
        let status = configuration_value(entry, "status")
            .ok_or_else(|| format!("quality document {path} has no status"))?;
        let expected_prefix = match kind.as_str() {
            "policy" => "quality/policy/",
            "review" => "quality/reviews/",
            "release-evidence" => "quality/evidence/",
            _ => return Err(format!("quality document {path} has unknown kind {kind:?}")),
        };
        if owner.is_empty() || status.is_empty() || !path.starts_with(expected_prefix) {
            return Err(format!(
                "quality document {path} has incomplete or inconsistent metadata"
            ));
        }
        if !registered.insert(path.clone()) {
            return Err(format!("quality document is registered twice: {path}"));
        }
        let content = read_workspace_text(&root, &path)?;
        if !content.starts_with(&expected_license_marker) {
            return Err(format!("quality document lacks its license marker: {path}"));
        }
    }

    let mut files = Vec::new();
    collect_regular_files(&root.join("quality"), &mut files)?;
    let actual = files
        .iter()
        .filter(|path| path.extension().and_then(|value| value.to_str()) == Some("md"))
        .filter(|path| path.file_name().and_then(|value| value.to_str()) != Some("README.md"))
        .filter(|path| path.as_path() != root.join(constants::QUALITY_STATUS_FILE))
        .map(|path| {
            path.strip_prefix(&root)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<BTreeSet<_>>();
    if actual != registered {
        let unregistered = actual.difference(&registered).collect::<Vec<_>>();
        let missing = registered.difference(&actual).collect::<Vec<_>>();
        return Err(format!(
            "quality inventory differs from durable documents; unregistered: {unregistered:?}; missing: {missing:?}"
        ));
    }
    println!("{} quality document inventory: pass", constants::INFO);
    Ok(())
}

/// Verifies directory ownership, durable test levels, fuzz ownership, and hygiene.
fn check_repository_structure() -> Result<(), String> {
    let root = workspace_root()?;
    let expected_license_marker = html_spdx_marker(&project_license(&root)?);
    let mut configured = BTreeSet::new();
    for directory in repository_directories(&root)? {
        let path = directory.path;
        let readme = directory.readme;
        if !is_safe_relative_path(Path::new(&path))
            || Path::new(&path).components().count() != 1
            || !is_safe_relative_path(Path::new(&readme))
            || !readme.starts_with(&format!("{path}/"))
            || directory.owner.is_empty()
            || directory.lifecycle.is_empty()
            || !root.join(&path).is_dir()
            || !root.join(&readme).is_file()
        {
            return Err(format!(
                "repository directory {path} has incomplete ownership"
            ));
        }
        let readme_content = read_workspace_text(&root, &readme)?;
        if !readme_content.starts_with(&expected_license_marker) {
            return Err(format!(
                "repository README lacks its license marker: {readme}"
            ));
        }
        if !configured.insert(path.clone()) {
            return Err(format!("repository directory is declared twice: {path}"));
        }
    }
    let tracked = Command::new(constants::GIT_COMMAND)
        .current_dir(&root)
        .args(["ls-files", "--cached", "-z"])
        .output()
        .map_err(|error| format!("could not enumerate tracked repository roots: {error}"))?;
    if !tracked.status.success() {
        return Err("could not enumerate tracked repository roots".to_owned());
    }
    let mut expected = BTreeSet::new();
    for file in tracked
        .stdout
        .split(|byte| *byte == 0)
        .filter(|file| !file.is_empty())
    {
        let name = std::str::from_utf8(file)
            .map_err(|error| format!("tracked repository path is not UTF-8: {error}"))?;
        if let Some((directory, _)) = name.split_once('/')
            && directory != constants::PORTABLE_DIRECTORY
        {
            expected.insert(directory.to_owned());
        }
    }
    if configured != expected {
        return Err(format!(
            "repository layout differs from required tracked roots: expected {expected:?}, found {configured:?}"
        ));
    }
    for manifest in workspace_package_manifests(&root)? {
        let readme = manifest.parent().unwrap_or(&root).join("README.md");
        if !readme.is_file() {
            return Err(format!(
                "workspace package has no responsibility README: {}",
                manifest.display()
            ));
        }
    }
    for readme in ["scripts/linux/README.md", "scripts/win/README.md"] {
        if !root.join(readme).is_file() {
            return Err(format!(
                "script directory has no responsibility README: {readme}"
            ));
        }
    }
    verify_test_level_inventory(&root)?;
    verify_coverage_policy()?;
    verify_fuzz_ownership(&root)?;
    verify_generated_file_hygiene(&root)?;
    verify_ignore_policy(&root)?;
    println!("{} repository ownership and hygiene: pass", constants::INFO);
    Ok(())
}

/// Verifies generated-state ignores without allowing source-of-truth inputs.
fn verify_ignore_policy(root: &Path) -> Result<(), String> {
    let ignore = read_workspace_text(root, ".gitignore")?;
    for required in [
        "target",
        "**/mutants.out*/",
        "fuzz/artifacts/",
        "fuzz/corpus/*/",
        "fuzz/coverage/",
        "massif.out.*",
        "callgrind.out.*",
        "perf.data*",
        "*.profraw",
        "/dist/",
        "/release/",
        ".idea/",
        ".vscode/",
        "*.swp",
    ] {
        if !ignore.lines().any(|line| line.trim() == required) {
            return Err(format!("generated-state ignore is missing {required}"));
        }
    }
    let results_ignore = format!("{}/", automation_value("output", "results_root")?);
    if !ignore.lines().any(|line| line.trim() == results_ignore) {
        return Err(format!(
            "generated-state ignore is missing {results_ignore}"
        ));
    }
    let bundle = ReleasedBundle::load(root)?;
    let freeze = bundle.member("specs/contracts/freeze.toml");
    let manifest = bundle.member("conformance/manifest.toml");
    for required in [
        "Cargo.lock",
        "rust-toolchain.toml",
        constants::AUTOMATION_CONFIG_FILE,
        constants::CONFORMANCE_CONFIG_FILE,
        "config/release.toml",
        &freeze,
        &manifest,
        "scripts/linux/bootstrap.sh",
    ] {
        let output = Command::new(constants::GIT_COMMAND)
            .current_dir(root)
            .args(["ls-files", "--error-unmatch", required])
            .output()
            .map_err(|error| format!("could not inspect tracked input {required}: {error}"))?;
        if !output.status.success() {
            return Err(format!(
                "required source-of-truth input is not tracked: {required}"
            ));
        }
    }
    Ok(())
}

/// Verifies coverage environment, outputs, thresholds, and exclusion policy.
fn verify_coverage_policy() -> Result<(), String> {
    let toolchain = quality_value("coverage", "toolchain")?;
    if toolchain != "nightly-only" && toolchain != "stable" {
        return Err(format!(
            "unsupported coverage toolchain policy: {toolchain}"
        ));
    }
    let html = quality_output_path("coverage", "html_output")?;
    if !html.ends_with("html/index.html") {
        return Err("coverage HTML output must end in html/index.html".to_owned());
    }
    let json = quality_output_path("coverage", "json_output")?;
    if json.extension().and_then(std::ffi::OsStr::to_str) != Some("json") {
        return Err("coverage JSON output must have a .json extension".to_owned());
    }
    for (minimum_key, observed_key) in [
        ("minimum_line_percent", "observed_line_percent"),
        ("minimum_function_percent", "observed_function_percent"),
        ("minimum_region_percent", "observed_region_percent"),
    ] {
        let minimum = quality_value("coverage", minimum_key)?
            .parse::<f64>()
            .map_err(|error| format!("invalid coverage {minimum_key}: {error}"))?;
        let observed = quality_value("coverage", observed_key)?
            .parse::<f64>()
            .map_err(|error| format!("invalid coverage {observed_key}: {error}"))?;
        if !(minimum > 0.0 && minimum <= 100.0 && observed >= minimum && observed <= 100.0) {
            return Err(format!(
                "coverage {observed_key} does not satisfy {minimum_key}"
            ));
        }
    }
    if quality_value("coverage", "exclusion_regex")?.is_empty() {
        return Err("coverage exclusions must be explicit".to_owned());
    }
    let exclusions = quality_value("coverage", "exclusion_policy")?;
    if !exclusions.starts_with("reviewed:") {
        return Err("coverage exclusions require an explicit reviewed rationale".to_owned());
    }
    Ok(())
}

/// Verifies every durable test purpose has one documented stable command.
fn verify_test_level_inventory(root: &Path) -> Result<(), String> {
    let inventory = read_workspace_text(root, constants::TEST_LEVELS_FILE)?;
    for (name, command) in [
        ("unit", "cargo xtask test unit"),
        ("smoke", "cargo xtask test smoke"),
        ("integration", "cargo xtask test integration"),
        ("system", "cargo xtask test system"),
        ("conformance", "cargo xtask test conformance"),
        ("property", "cargo xtask test property"),
        ("security", "cargo xtask test security"),
        ("fuzz-regression", "cargo xtask fuzz smoke"),
        ("performance", "cargo xtask test performance --profile pr"),
        ("all", "cargo xtask test all"),
    ] {
        if !inventory.contains(&format!("name = \"{name}\""))
            || !inventory.contains(&format!("command = \"{command}\""))
        {
            return Err(format!("test-level inventory is missing {name}: {command}"));
        }
    }
    Ok(())
}

/// Verifies configured fuzz targets have harnesses, owners, and state policy.
fn verify_fuzz_ownership(root: &Path) -> Result<(), String> {
    let quality = read_workspace_text(root, constants::QUALITY_GATES_FILE)?;
    for target in quality_array("fuzz", "targets")? {
        if !root
            .join("fuzz/fuzz_targets")
            .join(format!("{target}.rs"))
            .is_file()
            || !quality.contains(&format!("{target}_owner = \""))
        {
            return Err(format!(
                "fuzz target {target} has no harness or subsystem owner"
            ));
        }
    }
    for required in ["finding_policy = \"", "mutable_state = \""] {
        if !quality.contains(required) {
            return Err(format!("fuzz policy is missing {required}"));
        }
    }
    Ok(())
}

/// Rejects tracked generated products while retaining documented empty roots.
fn verify_generated_file_hygiene(root: &Path) -> Result<(), String> {
    let result_directory = automation_value("output", "results_root")?;
    let output = Command::new(constants::GIT_COMMAND)
        .current_dir(root)
        .args([
            "ls-files",
            "--",
            "target",
            "mutants.out",
            "mutants.out.old",
            "fuzz/artifacts",
            "fuzz/corpus",
            "portable/archive",
            "portable/archived",
        ])
        .arg(result_directory)
        .output()
        .map_err(|error| format!("could not inspect tracked generated files: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "could not inspect tracked generated files: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let tracked = String::from_utf8(output.stdout)
        .map_err(|error| format!("Git emitted non-UTF-8 paths: {error}"))?
        .lines()
        .filter(|path| *path != "fuzz/corpus/README.md")
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if tracked.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "generated or archived products are tracked: {}",
            tracked.join(", ")
        ))
    }
}

/// Runs one controlled benchmark, stress, or soak profile.
fn performance(profile: PerformanceProfile) -> Result<(), String> {
    let profile = match profile {
        PerformanceProfile::Pr => "pr",
        PerformanceProfile::Release => "release",
        PerformanceProfile::Soak => "soak",
    };
    match profile {
        "pr" | "release" | "soak" => {
            let harness = quality_value("performance", "harness")?;
            let (package, target) = harness
                .split_once('/')
                .ok_or_else(|| "quality performance harness must be package/target".to_owned())?;
            if package != constants::NEUTRAL_BENCH
                || target.is_empty()
                || !target
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
            {
                return Err(format!("invalid quality performance harness: {harness}"));
            }
            run_cargo(&[
                "bench",
                "--package",
                package,
                "--bench",
                target,
                "--",
                profile,
            ])
        }
        _ => unreachable!("performance profile is closed by command parsing"),
    }
}

/// Runs the built CLI and probe command-shell smoke checks.
fn run_shell_smoke() -> Result<(), String> {
    run_cargo(&[
        "run",
        "--quiet",
        "--package",
        constants::NEUTRAL_CLI,
        "--",
        "--help",
    ])?;
    run_cargo(&[
        "run",
        "--quiet",
        "--package",
        constants::NEUTRAL_PROBE,
        "--",
        "--help",
    ])
}

/// Verifies that every current test category has its configured minimum.
fn verify_test_counts(profile: &str) -> Result<(), String> {
    let test_list = command_output(
        &cargo_command()?,
        &[
            "test",
            "--workspace",
            "--lib",
            "--bins",
            "--tests",
            "--",
            "--list",
        ],
    )?;
    let minimums = test_minimums(profile)?;
    let discovered = minimums
        .keys()
        .map(|category| {
            (
                category.clone(),
                test_list.matches(&format!("{category}_")).count(),
            )
        })
        .collect();
    validate_test_minimums(&minimums, &discovered)
}

/// Rejects an active test category whose discovered count is below its minimum.
fn validate_test_minimums(
    minimums: &BTreeMap<String, usize>,
    discovered: &BTreeMap<String, usize>,
) -> Result<(), String> {
    for (category, minimum) in minimums {
        let discovered = discovered.get(category).copied().unwrap_or_default();
        if discovered < *minimum {
            return Err(format!(
                "active suite {category} requires at least {minimum} tests; discovered {discovered}"
            ));
        }
    }
    Ok(())
}

/// Runs one internal CI profile using only stable public task compositions.
fn ci(profile: CiProfile) -> Result<(), String> {
    match profile {
        CiProfile::Pr => run_ci_gate("pr", QualityProfile::Pr),
        CiProfile::Release => release_prepare(),
    }
}

/// Runs a stable quality composition and writes its generated CI summary.
fn run_ci_gate(profile: &str, quality_profile: QualityProfile) -> Result<(), String> {
    run_recorded_workflow(
        "ci",
        profile,
        vec![
            ("environment", Box::new(verify_environment)),
            ("quality", Box::new(move || quality(quality_profile))),
        ],
    )
}

/// Runs Cargo with inherited standard streams and converts failures to task errors.
fn run_cargo(arguments: &[&str]) -> Result<(), String> {
    let cargo = cargo_command()?;
    let status = Command::new(&cargo)
        .current_dir(workspace_root()?)
        .args(arguments)
        .status()
        .map_err(|error| format!("could not run {} {}: {error}", cargo, arguments.join(" ")))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{} {} failed with {status}", cargo, arguments.join(" ")))
}

/// Returns trimmed UTF-8 output from a successful command.
fn command_output(command: &str, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new(command)
        .args(arguments)
        .output()
        .map_err(|error| format!("could not run {command}: {error}"))?;
    if !output.status.success() {
        return Err(format!("{command} failed with {}", output.status));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| format!("{command} emitted non-UTF-8 output: {error}"))
}

/// Creates a process-unique directory below one approved generated-output root.
fn unique_generated_directory(parent: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "could not create generated evidence directory {}: {error}",
            parent.display()
        )
    })?;
    for suffix in 0_u16..1000 {
        let directory = parent.join(format!("run-{}-{suffix}", std::process::id()));
        match fs::create_dir(&directory) {
            Ok(()) => return Ok(directory),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(format!("could not create {}: {error}", directory.display())),
        }
    }
    Err("could not allocate a unique result directory".to_owned())
}

/// Runs ordered workflow steps while retaining start, pass, and failure events.
fn run_recorded_workflow(
    workflow: &str,
    profile: &str,
    steps: Vec<WorkflowStep<'_>>,
) -> Result<(), String> {
    let directory = unique_generated_directory(
        &result_root()?
            .join(constants::WORKFLOW_RESULT_DIRECTORY)
            .join(workflow)
            .join(profile),
    )?;
    let events = directory.join(constants::WORKFLOW_EVENTS_FILE);
    let summary = directory.join(constants::WORKFLOW_SUMMARY_FILE);
    let total = steps.len();
    let started_at = unix_time_millis()?;
    let started = Instant::now();
    write_workflow_summary(
        &summary, workflow, profile, "running", 0, total, started_at, None,
    )?;
    println!(
        "{} {workflow} {profile}: start; log {}",
        constants::INFO,
        directory.display()
    );

    for (index, (name, step)) in steps.into_iter().enumerate() {
        let step_started_at = unix_time_millis()?;
        let step_started = Instant::now();
        append_workflow_event(
            &events,
            workflow,
            profile,
            name,
            "start",
            step_started_at,
            0,
            None,
        )?;
        println!("{} {workflow}/{name}: start", constants::INFO);
        match step() {
            Ok(()) => {
                let elapsed = step_started.elapsed().as_millis();
                append_workflow_event(
                    &events,
                    workflow,
                    profile,
                    name,
                    "pass",
                    unix_time_millis()?,
                    elapsed,
                    None,
                )?;
                write_workflow_summary(
                    &summary,
                    workflow,
                    profile,
                    "running",
                    index + 1,
                    total,
                    started_at,
                    None,
                )?;
                println!("{} {workflow}/{name}: pass ({elapsed} ms)", constants::INFO);
            }
            Err(error) => {
                let elapsed = step_started.elapsed().as_millis();
                append_workflow_event(
                    &events,
                    workflow,
                    profile,
                    name,
                    "fail",
                    unix_time_millis()?,
                    elapsed,
                    Some(&error),
                )?;
                write_workflow_summary(
                    &summary,
                    workflow,
                    profile,
                    "fail",
                    index,
                    total,
                    started_at,
                    Some(&error),
                )?;
                return Err(format!(
                    "{workflow}/{name} failed: {error}; workflow log: {}",
                    directory.display()
                ));
            }
        }
    }

    let elapsed = started.elapsed().as_millis();
    write_workflow_summary(
        &summary, workflow, profile, "pass", total, total, started_at, None,
    )?;
    println!(
        "{} {workflow} {profile}: pass ({elapsed} ms); log {}",
        constants::INFO,
        directory.display()
    );
    Ok(())
}

/// Appends one machine-readable workflow event without replacing earlier events.
#[allow(clippy::too_many_arguments)]
fn append_workflow_event(
    path: &Path,
    workflow: &str,
    profile: &str,
    step: &str,
    status: &str,
    timestamp_unix_ms: u128,
    duration_ms: u128,
    error: Option<&str>,
) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("could not open workflow event log: {error}"))?;
    writeln!(
        file,
        "{{\"workflow\":\"{}\",\"profile\":\"{}\",\"step\":\"{}\",\"status\":\"{}\",\"timestamp_unix_ms\":{timestamp_unix_ms},\"duration_ms\":{duration_ms},\"error\":{}}}",
        json_string(workflow),
        json_string(profile),
        json_string(step),
        json_string(status),
        json_optional_string(error)
    )
    .map_err(|error| format!("could not write workflow event log: {error}"))
}

/// Writes the current machine-readable summary for an aggregate workflow.
#[allow(clippy::too_many_arguments)]
fn write_workflow_summary(
    path: &Path,
    workflow: &str,
    profile: &str,
    status: &str,
    completed_steps: usize,
    total_steps: usize,
    started_at_unix_ms: u128,
    error: Option<&str>,
) -> Result<(), String> {
    let root = workspace_root()?;
    let manifest = read_workspace_text(&root, constants::WORKSPACE_MANIFEST_FILE)?;
    let version = workspace_package_version(&manifest)?;
    let license = workspace_package_license(&manifest)?;
    let commit = command_output(constants::GIT_COMMAND, &["rev-parse", "HEAD"])?;
    let worktree_clean =
        command_output(constants::GIT_COMMAND, &["status", "--porcelain"])?.is_empty();
    let rustc = command_output(&rustc_command()?, &["--version"])?;
    fs::write(
        path,
        format!(
            "{{\"schema_version\":1,\"workflow\":\"{}\",\"profile\":\"{}\",\"status\":\"{}\",\"completed_steps\":{completed_steps},\"total_steps\":{total_steps},\"started_at_unix_ms\":{started_at_unix_ms},\"updated_at_unix_ms\":{},\"source_commit\":\"{}\",\"worktree_clean\":{worktree_clean},\"package_version\":\"{}\",\"license\":\"{}\",\"rustc\":\"{}\",\"error\":{}}}\n",
            json_string(workflow),
            json_string(profile),
            json_string(status),
            unix_time_millis()?,
            json_string(&commit),
            json_string(&version),
            json_string(&license),
            json_string(&rustc),
            json_optional_string(error)
        ),
    )
    .map_err(|error| format!("could not write workflow summary: {error}"))
}

/// Encodes an optional string as one JSON value.
fn json_optional_string(value: Option<&str>) -> String {
    value.map_or_else(
        || "null".to_owned(),
        |value| format!("\"{}\"", json_string(value)),
    )
}

/// Returns milliseconds elapsed since the Unix epoch for generated evidence.
fn unix_time_millis() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| format!("system clock precedes Unix epoch: {error}"))
}

/// Checks all declared package and effect boundaries against Cargo's graph.
fn check_boundaries() -> Result<(), String> {
    let workspace_root = workspace_root()?;
    verify_pure_source_effects(&workspace_root)?;
    verify_release_path_independence(&workspace_root)?;
    let policy = direct_dependency_policy();

    for (package, allowed_dependencies) in &policy {
        let dependencies = direct_dependencies(&workspace_root, package)?;
        validate_direct_dependencies(package, &dependencies, allowed_dependencies)?;
    }

    let compiler_closure = package_names(&tree_output(
        &workspace_root,
        constants::NEUTRAL_COMPILER,
        "normal",
        None,
    )?);
    validate_allowed_packages(
        &format!("{} pure compilation closure", constants::NEUTRAL_COMPILER),
        &compiler_closure,
        &set([
            constants::NEUTRAL_COMPILER,
            constants::NEUTRAL_CORE,
            constants::NEUTRAL_IR,
            constants::NEUTRAL_VOCABULARY,
            "block-buffer",
            "cfg-if",
            "cpufeatures",
            "crypto-common",
            "digest",
            "generic-array",
            "sha2",
            "typenum",
            "version_check",
        ]),
    )?;

    let probe_closure = package_names(&tree_output(
        &workspace_root,
        constants::NEUTRAL_PROBE,
        "all",
        None,
    )?);
    validate_allowed_packages(
        "neutral-probe dependency tree",
        &probe_closure,
        &set([
            constants::NEUTRAL_PROBE,
            constants::NEUTRAL_CORE,
            constants::NEUTRAL_ENCODING,
            constants::NEUTRAL_IR,
            constants::NEUTRAL_READER,
            constants::NEUTRAL_VOCABULARY,
            "block-buffer",
            "cfg-if",
            "cpufeatures",
            "crypto-common",
            "digest",
            "generic-array",
            "sha2",
            "typenum",
            "version_check",
        ]),
    )?;

    println!("{} dependency boundaries: pass", constants::INFO);
    Ok(())
}

/// Rejects ambient host APIs from the captured-compilation source closure.
fn verify_pure_source_effects(root: &Path) -> Result<(), String> {
    for package in [
        constants::NEUTRAL_CORE,
        constants::NEUTRAL_IR,
        constants::NEUTRAL_VOCABULARY,
        constants::NEUTRAL_COMPILER,
    ] {
        let mut files = Vec::new();
        collect_regular_files(
            &workspace_package_directory(root, package)?.join("src"),
            &mut files,
        )?;
        for file in files
            .iter()
            .filter(|path| path.extension().and_then(std::ffi::OsStr::to_str) == Some("rs"))
        {
            let content = fs::read_to_string(file)
                .map_err(|error| format!("could not read {}: {error}", file.display()))?;
            for forbidden in [
                "std::fs",
                "std::env",
                "std::net",
                "std::process",
                "std::time",
                "Command::new",
                "SystemTime",
                "Instant::now",
            ] {
                if content.contains(forbidden) {
                    return Err(format!(
                        "pure compilation source {} contains ambient API {forbidden}",
                        file.display()
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Rejects user-specific absolute paths from release configuration and adapters.
fn verify_release_path_independence(root: &Path) -> Result<(), String> {
    for relative in [
        constants::RELEASE_CONFIG_FILE,
        "xtask/src/release.rs",
        "scripts/linux/release.sh",
        "scripts/win/release.ps1",
    ] {
        let content = read_workspace_text(root, relative)?;
        for forbidden in [
            "/home/",
            "C:\\Users\\",
            "$HOME",
            "${HOME}",
            "%USERPROFILE%",
            "~/",
        ] {
            if content.contains(forbidden) {
                return Err(format!(
                    "release surface {relative} contains user-specific path marker {forbidden}"
                ));
            }
        }
    }
    Ok(())
}

/// Checks accepted IDs, completed syntax items, and fixture/oracle inventories.
fn check_traceability() -> Result<(), String> {
    let root = workspace_root()?;
    let bundle = ReleasedBundle::load(&root)?;
    check_traceability_bundle(
        &bundle.member("specs/REQUIREMENTS.md"),
        &bundle.member("specs/contracts/syntax.md"),
        &bundle.member("specs/contracts/syntax-checklist.md"),
        &bundle.member("specs/TRACEABILITY.md"),
        &bundle.member("conformance/manifest.toml"),
        &bundle.member("specs/fixtures"),
        &bundle.member("conformance/oracles"),
    )?;
    verify_frozen_input_digests(&root, &bundle.member("specs/contracts/freeze.toml"))
}

/// Checks the version-independent conformance inventory of an installed portable package.
fn check_portable_traceability_at(root: &Path) -> Result<(), String> {
    let manifest = read_workspace_text(root, constants::PORTABLE_CONFORMANCE_MANIFEST_FILE)?;
    ensure_inventory_registered(root, constants::PORTABLE_FIXTURE_DIRECTORY, &manifest)?;
    ensure_inventory_registered(root, constants::PORTABLE_ORACLE_DIRECTORY, &manifest)?;
    ensure_registered_paths_exist(root, &manifest)?;
    println!("{} traceability coherence: pass", constants::INFO);
    Ok(())
}

/// Checks one self-contained contract, fixture, and oracle bundle.
fn check_traceability_bundle(
    requirements_file: &str,
    syntax_file: &str,
    checklist_file: &str,
    traceability_file: &str,
    manifest_file: &str,
    fixture_directory: &str,
    oracle_directory: &str,
) -> Result<(), String> {
    let root = workspace_root()?;
    let requirements = read_workspace_text(&root, requirements_file)?;
    let syntax = read_workspace_text(&root, syntax_file)?;
    let checklist = read_workspace_text(&root, checklist_file)?;
    let traceability = read_workspace_text(&root, traceability_file)?;
    let manifest = read_workspace_text(&root, manifest_file)?;

    let requirement_ids = contract_ids(&requirements, "NL-");
    let syntax_ids = contract_ids(&syntax, "SYN-");
    let checklist_ids = contract_ids(&checklist, "SYN-");
    ensure_ids_covered("requirements", &requirement_ids, &traceability)?;
    ensure_ids_covered("syntax", &syntax_ids, &traceability)?;
    if syntax_ids != checklist_ids {
        return Err("master syntax and implementation checklist IDs differ".to_owned());
    }
    ensure_syntax_complete(syntax_file, &syntax)?;
    ensure_syntax_complete(checklist_file, &checklist)?;
    ensure_inventory_registered(&root, fixture_directory, &manifest)?;
    ensure_inventory_registered(&root, oracle_directory, &manifest)?;
    ensure_registered_paths_exist(&root, &manifest)?;

    println!("{} traceability coherence: pass", constants::INFO);
    Ok(())
}

/// Reads one required UTF-8 workspace file with a path-safe error.
fn read_workspace_text(root: &Path, relative: &str) -> Result<String, String> {
    fs::read_to_string(root.join(relative))
        .map_err(|error| format!("could not read {relative}: {error}"))
}

/// Extracts unique contract identifiers beginning with `prefix`.
fn contract_ids(content: &str, prefix: &str) -> BTreeSet<String> {
    content
        .split(|character: char| !(character.is_ascii_alphanumeric() || character == '-'))
        .filter(|token| token.starts_with(prefix))
        .map(str::to_owned)
        .collect()
}

/// Requires every accepted identifier to occur in the evidence index.
fn ensure_ids_covered(
    category: &str,
    identifiers: &BTreeSet<String>,
    traceability: &str,
) -> Result<(), String> {
    if identifiers.is_empty() {
        return Err(format!("{category} contains no contract identifiers"));
    }
    let missing = identifiers
        .iter()
        .filter(|identifier| !traceability.contains(identifier.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "traceability is missing {category} IDs: {}",
            missing.join(", ")
        ))
    }
}

/// Rejects an unchecked master syntax item after traceability closure.
fn ensure_syntax_complete(path: &str, content: &str) -> Result<(), String> {
    let unchecked = content
        .lines()
        .filter(|line| line.trim_start().starts_with("- [ ]") && line.contains("SYN-"))
        .collect::<Vec<_>>();
    if unchecked.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{path} contains {} unchecked syntax items",
            unchecked.len()
        ))
    }
}

/// Requires every normative file beneath `directory` to occur in the manifest.
fn ensure_inventory_registered(root: &Path, directory: &str, manifest: &str) -> Result<(), String> {
    let mut files = Vec::new();
    collect_regular_files(&root.join(directory), &mut files)?;
    let missing = files
        .iter()
        .filter_map(|path| path.strip_prefix(root).ok())
        .map(|path| path.to_string_lossy().replace('\\', "/"))
        .filter(|path| {
            matches!(
                Path::new(path).extension().and_then(|value| value.to_str()),
                Some("neu" | "json" | "toml")
            )
        })
        .filter(|path| !manifest.contains(path))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "conformance manifest omits normative files: {}",
            missing.join(", ")
        ))
    }
}

/// Recursively collects regular files without following directory symlinks.
fn collect_regular_files(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(directory)
        .map_err(|error| format!("could not inspect {}: {error}", directory.display()))?;
    for entry in entries {
        let entry = entry.map_err(|error| format!("could not inspect directory entry: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("could not inspect {}: {error}", entry.path().display()))?;
        if file_type.is_dir() {
            collect_regular_files(&entry.path(), files)?;
        } else if file_type.is_file() {
            files.push(entry.path());
        }
    }
    Ok(())
}

/// Rejects inline Rust test bodies and requires path-based crate-local test modules.
fn check_test_layout() -> Result<(), String> {
    let root = workspace_root()?;
    let mut source_files = Vec::new();
    for manifest in workspace_package_manifests(&root)? {
        let source = manifest
            .parent()
            .ok_or_else(|| format!("workspace manifest has no parent: {}", manifest.display()))?
            .join("src");
        if source.is_dir() {
            collect_regular_files(&source, &mut source_files)?;
        }
    }
    let mut violations = Vec::new();
    for path in source_files {
        if path.extension().and_then(|value| value.to_str()) != Some("rs")
            || !path
                .components()
                .any(|component| component.as_os_str() == "src")
        {
            continue;
        }
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("could not read {}: {error}", path.display()))?;
        if source_has_non_path_test_configuration(&content) {
            violations.push(
                path.strip_prefix(&root)
                    .unwrap_or(&path)
                    .display()
                    .to_string(),
            );
        }
    }
    if violations.is_empty() {
        println!("{} crate-local test layout: pass", constants::INFO);
        Ok(())
    } else {
        Err(format!(
            "production source contains inline or non-path test modules: {}",
            violations.join(", ")
        ))
    }
}

/// Returns whether any test-only source declaration lacks an immediate path attribute.
fn source_has_non_path_test_configuration(content: &str) -> bool {
    let lines = content.lines().collect::<Vec<_>>();
    lines.iter().enumerate().any(|(index, line)| {
        line.trim() == constants::TEST_CONFIGURATION_MARKER
            && lines[index + 1..]
                .iter()
                .find(|candidate| !candidate.trim().is_empty())
                .is_none_or(|candidate| {
                    !candidate
                        .trim()
                        .starts_with(constants::TEST_PATH_ATTRIBUTE_MARKER)
                })
    })
}

/// Requires every workspace-relative path registered by the manifest to exist.
fn ensure_registered_paths_exist(root: &Path, manifest: &str) -> Result<(), String> {
    let missing = manifest
        .split('"')
        .filter(|value| {
            value.starts_with("conformance/releases/") || value.starts_with("portable/")
        })
        .filter(|value| !root.join(value).is_file())
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "manifest paths do not exist: {}",
            missing.join(", ")
        ))
    }
}

/// Finds the owning Cargo workspace even if the automation crate moves.
fn workspace_root() -> Result<PathBuf, String> {
    let manifest_directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for directory in manifest_directory.ancestors() {
        let manifest = directory.join(constants::WORKSPACE_MANIFEST_FILE);
        if !manifest.is_file() {
            continue;
        }
        let content = fs::read_to_string(&manifest)
            .map_err(|error| format!("could not read {}: {error}", manifest.display()))?;
        if content.lines().any(|line| line.trim() == "[workspace]") {
            return Ok(directory.to_path_buf());
        }
    }
    Err("automation package is not inside a Cargo workspace".to_owned())
}

/// Returns the exact normal-dependency policy for every workspace package.
fn direct_dependency_policy() -> BTreeMap<&'static str, BTreeSet<&'static str>> {
    BTreeMap::from([
        (constants::NEUTRAL_BENCH, set([])),
        (
            constants::NEUTRAL_CLI,
            set([
                constants::NEUTRAL_COMPILER,
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_ENCODING,
                constants::NEUTRAL_READER,
                constants::NEUTRAL_VOCABULARY,
            ]),
        ),
        (
            constants::NEUTRAL_COMPILER,
            set([
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_IR,
                constants::NEUTRAL_VOCABULARY,
            ]),
        ),
        (constants::NEUTRAL_CORE, set(["sha2"])),
        (
            constants::NEUTRAL_ENCODING,
            set([
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_IR,
                constants::NEUTRAL_READER,
                constants::NEUTRAL_VOCABULARY,
            ]),
        ),
        (constants::NEUTRAL_IR, set([constants::NEUTRAL_CORE])),
        (
            constants::NEUTRAL_PROBE,
            set([
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_ENCODING,
                constants::NEUTRAL_READER,
            ]),
        ),
        (
            constants::NEUTRAL_READER,
            set([
                constants::NEUTRAL_CORE,
                constants::NEUTRAL_IR,
                constants::NEUTRAL_VOCABULARY,
            ]),
        ),
        (constants::NEUTRAL_TEST_SUITE, set([])),
        (constants::NEUTRAL_TEST_SUPPORT, set([])),
        (
            constants::NEUTRAL_VOCABULARY,
            set([constants::NEUTRAL_CORE, constants::NEUTRAL_IR]),
        ),
        (constants::XTASK, set(["sha2"])),
    ])
}

/// Resolves the direct normal dependencies of one workspace package.
fn direct_dependencies(
    workspace_root: &PathBuf,
    package: &str,
) -> Result<BTreeSet<String>, String> {
    let output = tree_output(workspace_root, package, "normal", Some(1))?;
    let mut packages = package_names(&output);
    packages.remove(package);
    Ok(packages)
}

/// Runs `cargo tree` and returns its UTF-8 output for one package.
fn tree_output(
    workspace_root: &PathBuf,
    package: &str,
    edges: &str,
    depth: Option<u8>,
) -> Result<String, String> {
    let mut command = Command::new(cargo_command()?);
    command.current_dir(workspace_root).args([
        "tree",
        "--locked",
        "--package",
        package,
        "--edges",
        edges,
        "--prefix",
        "none",
    ]);

    if let Some(depth) = depth {
        command.args(["--depth", &depth.to_string()]);
    }

    let output = command
        .output()
        .map_err(|error| format!("could not run cargo tree for {package}: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo tree failed for {package}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    String::from_utf8(output.stdout)
        .map_err(|error| format!("cargo tree emitted non-UTF-8 output for {package}: {error}"))
}

/// Extracts package names from Cargo's no-prefix tree output.
fn package_names(tree: &str) -> BTreeSet<String> {
    tree.lines()
        .filter_map(|line| {
            let mut words = line.split_whitespace();
            let name = words.next()?;
            let version_or_kind = words.next()?;
            version_or_kind.starts_with('v').then(|| name.to_owned())
        })
        .collect()
}

/// Rejects direct dependencies that differ from the package's exact policy.
fn validate_direct_dependencies(
    package: &str,
    actual: &BTreeSet<String>,
    allowed: &BTreeSet<&str>,
) -> Result<(), String> {
    let allowed = allowed.iter().map(|name| (*name).to_owned()).collect();
    validate_exact_packages(&format!("{package} direct dependencies"), actual, &allowed)
}

/// Compares an observed package set with an exact expected package set.
fn validate_exact_packages(
    boundary: &str,
    actual: &BTreeSet<String>,
    expected: &BTreeSet<String>,
) -> Result<(), String> {
    if actual == expected {
        return Ok(());
    }

    let unexpected = actual.difference(expected).collect::<Vec<_>>();
    let missing = expected.difference(actual).collect::<Vec<_>>();
    Err(format!(
        "{boundary} violates the policy; unexpected: {unexpected:?}; missing: {missing:?}"
    ))
}

/// Rejects packages outside an allowlist for a named dependency boundary.
fn validate_allowed_packages(
    boundary: &str,
    actual: &BTreeSet<String>,
    allowed: &BTreeSet<&str>,
) -> Result<(), String> {
    let unexpected = actual
        .iter()
        .filter(|package| !allowed.contains(package.as_str()))
        .collect::<Vec<_>>();

    if unexpected.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{boundary} contains forbidden packages: {unexpected:?}"
        ))
    }
}

/// Builds a sorted package-name set from an array literal.
fn set<const N: usize>(values: [&'static str; N]) -> BTreeSet<&'static str> {
    values.into_iter().collect()
}

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
