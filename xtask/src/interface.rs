// SPDX-License-Identifier: Apache-2.0

//! Stable, stage-free command grammar for repository automation.

use std::path::PathBuf;

/// Stable command name for repository formatting.
const FORMAT_COMMAND: &str = "fmt";
/// Stable command name for linting.
const LINT_COMMAND: &str = "lint";
/// Stable command name for repository checks.
const CHECK_COMMAND: &str = "check";
/// Stable command name for tests.
const TEST_COMMAND: &str = "test";
/// Stable command name for coverage.
const COVERAGE_COMMAND: &str = "coverage";
/// Stable command name for fuzzing.
const FUZZ_COMMAND: &str = "fuzz";
/// Stable command name for aggregate quality checks.
const QUALITY_COMMAND: &str = "quality";
/// Stable command name for builds.
const BUILD_COMMAND: &str = "build";
/// Stable command name for artifact validation.
const VALIDATE_COMMAND: &str = "validate";
/// Stable command name for distribution assembly.
const PACKAGE_COMMAND: &str = "package";
/// Stable command name for release preparation.
const RELEASE_COMMAND: &str = "release";
/// Stable command name for package-version operations.
const VERSION_COMMAND: &str = "version";
/// Stable command name for the active portable lifecycle.
const PORTABLE_COMMAND: &str = "portable";
/// Stable command name for generated-output cleanup.
const CLEAN_COMMAND: &str = "clean";
/// Internal command name used only by CI wrappers.
const CI_COMMAND: &str = "ci";
/// Contributor bootstrap command name.
const BOOTSTRAP_COMMAND: &str = "bootstrap";
/// Environment inspection command name.
const ENVIRONMENT_COMMAND: &str = "environment";
/// Workspace documentation command name.
const DOCS_COMMAND: &str = "docs";
/// Ordered local-development workflow command name.
const DEV_COMMAND: &str = "dev";
/// Mutation-analysis command name.
const MUTATE_COMMAND: &str = "mutate";
/// Help option accepted at the root command boundary.
const HELP_OPTION: &str = "--help";
/// Short help option accepted at the root command boundary.
const SHORT_HELP_OPTION: &str = "-h";
/// Profile selector shared by profiled commands.
const PROFILE_OPTION: &str = "--profile";

/// Complete help rendered by `cargo xtask --help`.
pub(crate) const HELP: &str = "Neutral repository automation\n\n\
Usage: cargo xtask <command> [options]\n\n\
Stable commands:\n\
  bootstrap                                      verify the host workspace\n\
  dev                                            format, check, lint, test, and document\n\
  environment verify|manifest                    inspect the selected tools\n\
  fmt [--write]                                  check or apply Rust formatting\n\
  lint                                           run warning-free workspace linting\n\
  check                                          check code and repository contracts\n\
  test unit|smoke|integration|system|conformance|property|security|all\n\
  test performance --profile pr|release|soak     run a controlled performance profile\n\
  coverage                                       enforce configured LLVM coverage\n\
  fuzz smoke|campaign                            run bounded or full fuzzing\n\
  quality [--profile pr|release]                 run the documented quality composition\n\
  quality status|render|verify                   inspect or synchronize the quality ledger\n\
  quality evaluate --profile pr|release          run and retain a commit-bound evaluation\n\
  quality approve --release <version>            approve a passing release evaluation\n\
  build --profile dev|release                    build the workspace\n\
  docs                                           generate workspace API documentation\n\
  validate <artifact>|binaries                   validate a release artifact or binaries\n\
  package                                        assemble the selected distribution\n\
  release prepare                                prepare, but never publish, a release\n\
  version show|check|prepare <version>            inspect or prepare package versioning\n\
  portable install <directory>                   atomically install a reviewed portable\n\
  portable verify|snapshot                       verify or snapshot the active portable\n\
  clean                                          remove ignored generated evidence\n\n\
Automation workflows:\n\
  ci pr|release                                  run the same logged gates locally and in CI";

/// Command fragments that every contributor-facing command map must expose.
pub(crate) const DOCUMENTED_COMMANDS: &[&str] = &[
    "cargo xtask dev",
    "cargo xtask fmt",
    "cargo xtask lint",
    "cargo xtask check",
    "cargo xtask test",
    "cargo xtask coverage",
    "cargo xtask fuzz",
    "cargo xtask quality",
    "cargo xtask build",
    "cargo xtask validate",
    "cargo xtask package",
    "cargo xtask release prepare",
    "cargo xtask version",
    "cargo xtask portable",
    "cargo xtask clean",
    "cargo xtask ci",
];

/// One parsed repository-automation request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Task {
    /// Render stable command help.
    Help,
    /// Verify the contributor environment and initialize ignored output roots.
    Bootstrap,
    /// Run the ordered, auto-formatting local-development workflow.
    Dev,
    /// Verify the selected environment.
    EnvironmentVerify,
    /// Print a machine-readable environment manifest.
    EnvironmentManifest,
    /// Check or write Rust formatting.
    Format {
        /// Whether formatting changes should be applied.
        write: bool,
    },
    /// Run warning-free linting.
    Lint,
    /// Run compilation and repository-coherence checks.
    Check,
    /// Build the workspace using a durable profile.
    Build(BuildProfile),
    /// Generate workspace documentation.
    Docs,
    /// Run one durable test level.
    Test(TestLevel),
    /// Run a controlled performance profile.
    Performance(PerformanceProfile),
    /// Run one fuzz mode.
    Fuzz(FuzzMode),
    /// Run configured LLVM coverage.
    Coverage,
    /// Run configured mutation analysis.
    Mutate,
    /// Operate the managed quality ledger or run an aggregate profile.
    Quality(QualityAction),
    /// Validate released binaries or one encoded artifact.
    Validate(ValidationTarget),
    /// Assemble the selected release distribution.
    Package,
    /// Prepare a release without publishing it.
    ReleasePrepare,
    /// Inspect or prepare package versioning.
    Version(VersionAction),
    /// Verify or snapshot the active portable package.
    Portable(PortableAction),
    /// Remove ignored generated evidence.
    Clean,
    /// Run one internal CI composition.
    Ci(CiProfile),
}

/// Supported workspace build profiles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BuildProfile {
    /// Fast developer build.
    Dev,
    /// Optimized release build.
    Release,
}

/// Independently runnable, durable test levels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TestLevel {
    /// All crate-owned tests.
    Unit,
    /// Built-binary help/version smoke tests.
    Smoke,
    /// Cross-package integration tests.
    Integration,
    /// Process and filesystem system tests.
    System,
    /// Manifest-driven normative conformance tests.
    Conformance,
    /// Property and metamorphic tests.
    Property,
    /// Security and adversarial tests.
    Security,
    /// Complete ordinary test composition.
    All,
}

/// Controlled performance profiles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PerformanceProfile {
    /// Bounded pull-request performance smoke.
    Pr,
    /// Release qualification profile.
    Release,
    /// Extended soak profile.
    Soak,
}

/// Supported fuzzing modes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FuzzMode {
    /// Deterministic bounded fuzz regressions.
    Smoke,
    /// Full configured coverage-guided campaign.
    Campaign,
}

/// Aggregate quality profiles.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QualityProfile {
    /// Normal push/pull-request quality checks.
    Pr,
    /// Full release qualification checks.
    Release,
}

/// Managed quality workflow actions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum QualityAction {
    /// Run an aggregate quality profile without creating an evaluation record.
    Run(QualityProfile),
    /// Print the approved-release ledger.
    Status,
    /// Run a profile and retain a commit-bound evaluation.
    Evaluate(QualityProfile),
    /// Approve the matching release evaluation through an explicit human action.
    Approve(String),
    /// Regenerate the human-readable status document.
    Render,
    /// Verify the manifest, approvals, evidence hashes, and rendered status.
    Verify,
}

/// Artifact target selected for validation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ValidationTarget {
    /// Validate the release-mode CLI and standalone probe binaries.
    Binaries,
    /// Validate one encoded artifact through the standalone probe boundary.
    Artifact(PathBuf),
}

/// Stable package-version actions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum VersionAction {
    /// Display package and frozen contract versions.
    Show,
    /// Check centralized version invariants.
    Check,
    /// Prepare deterministic derived changes for a package version.
    Prepare(String),
}

/// Stable active-portable lifecycle actions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PortableAction {
    /// Install a reviewed portable package from one source directory.
    Install(PathBuf),
    /// Verify active portable ownership and links.
    Verify,
    /// Create a local deterministic archive candidate.
    Snapshot,
}

/// Internal CI compositions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CiProfile {
    /// Normal push/pull-request composition.
    Pr,
    /// Full release composition.
    Release,
}

/// Parses exact command arguments into one stable task.
pub(crate) fn parse(arguments: &[String]) -> Result<Task, String> {
    let values = arguments.iter().map(String::as_str).collect::<Vec<_>>();
    match values.as_slice() {
        [] | [HELP_OPTION | SHORT_HELP_OPTION] => Ok(Task::Help),
        [BOOTSTRAP_COMMAND] => Ok(Task::Bootstrap),
        [DEV_COMMAND] => Ok(Task::Dev),
        [ENVIRONMENT_COMMAND, "verify"] => Ok(Task::EnvironmentVerify),
        [ENVIRONMENT_COMMAND, "manifest"] => Ok(Task::EnvironmentManifest),
        [FORMAT_COMMAND] => Ok(Task::Format { write: false }),
        [FORMAT_COMMAND, "--write"] => Ok(Task::Format { write: true }),
        [LINT_COMMAND] => Ok(Task::Lint),
        [CHECK_COMMAND] => Ok(Task::Check),
        [DOCS_COMMAND] => Ok(Task::Docs),
        [BUILD_COMMAND, PROFILE_OPTION, "dev"] => Ok(Task::Build(BuildProfile::Dev)),
        [BUILD_COMMAND, PROFILE_OPTION, "release"] => Ok(Task::Build(BuildProfile::Release)),
        [TEST_COMMAND, "performance", PROFILE_OPTION, profile] => {
            parse_performance_profile(profile).map(Task::Performance)
        }
        [TEST_COMMAND, level] => parse_test_level(level).map(Task::Test),
        [FUZZ_COMMAND, mode] => parse_fuzz_mode(mode).map(Task::Fuzz),
        [COVERAGE_COMMAND] => Ok(Task::Coverage),
        [MUTATE_COMMAND] => Ok(Task::Mutate),
        [QUALITY_COMMAND] => Ok(Task::Quality(QualityAction::Run(QualityProfile::Pr))),
        [QUALITY_COMMAND, PROFILE_OPTION, profile] => parse_quality_profile(profile)
            .map(QualityAction::Run)
            .map(Task::Quality),
        [QUALITY_COMMAND, "status"] => Ok(Task::Quality(QualityAction::Status)),
        [QUALITY_COMMAND, "render"] => Ok(Task::Quality(QualityAction::Render)),
        [QUALITY_COMMAND, "verify"] => Ok(Task::Quality(QualityAction::Verify)),
        [QUALITY_COMMAND, "evaluate", PROFILE_OPTION, profile] => parse_quality_profile(profile)
            .map(QualityAction::Evaluate)
            .map(Task::Quality),
        [QUALITY_COMMAND, "approve", "--release", release] => {
            Ok(Task::Quality(QualityAction::Approve((*release).to_owned())))
        }
        [VALIDATE_COMMAND, "binaries"] => Ok(Task::Validate(ValidationTarget::Binaries)),
        [VALIDATE_COMMAND, artifact] => Ok(Task::Validate(ValidationTarget::Artifact(
            PathBuf::from(artifact),
        ))),
        [PACKAGE_COMMAND] => Ok(Task::Package),
        [RELEASE_COMMAND, "prepare"] => Ok(Task::ReleasePrepare),
        [VERSION_COMMAND, "show"] => Ok(Task::Version(VersionAction::Show)),
        [VERSION_COMMAND, "check"] => Ok(Task::Version(VersionAction::Check)),
        [VERSION_COMMAND, "prepare", version] => {
            Ok(Task::Version(VersionAction::Prepare((*version).to_owned())))
        }
        [PORTABLE_COMMAND, "verify"] => Ok(Task::Portable(PortableAction::Verify)),
        [PORTABLE_COMMAND, "snapshot"] => Ok(Task::Portable(PortableAction::Snapshot)),
        [PORTABLE_COMMAND, "install", source] => Ok(Task::Portable(PortableAction::Install(
            PathBuf::from(source),
        ))),
        [CLEAN_COMMAND] => Ok(Task::Clean),
        [CI_COMMAND, "pr"] => Ok(Task::Ci(CiProfile::Pr)),
        [CI_COMMAND, "release"] => Ok(Task::Ci(CiProfile::Release)),
        _ => Err(format!(
            "unsupported command: {}; run `cargo xtask --help`",
            values.join(" ")
        )),
    }
}

/// Parses one durable test-level name.
fn parse_test_level(value: &str) -> Result<TestLevel, String> {
    match value {
        "unit" => Ok(TestLevel::Unit),
        "smoke" => Ok(TestLevel::Smoke),
        "integration" => Ok(TestLevel::Integration),
        "system" => Ok(TestLevel::System),
        "conformance" => Ok(TestLevel::Conformance),
        "property" => Ok(TestLevel::Property),
        "security" => Ok(TestLevel::Security),
        "all" => Ok(TestLevel::All),
        _ => Err(format!("unknown test level: {value}")),
    }
}

/// Parses one controlled performance-profile name.
fn parse_performance_profile(value: &str) -> Result<PerformanceProfile, String> {
    match value {
        "pr" => Ok(PerformanceProfile::Pr),
        "release" => Ok(PerformanceProfile::Release),
        "soak" => Ok(PerformanceProfile::Soak),
        _ => Err(format!("unknown performance profile: {value}")),
    }
}

/// Parses one fuzz-mode name.
fn parse_fuzz_mode(value: &str) -> Result<FuzzMode, String> {
    match value {
        "smoke" => Ok(FuzzMode::Smoke),
        "campaign" => Ok(FuzzMode::Campaign),
        _ => Err(format!("unknown fuzz mode: {value}")),
    }
}

/// Parses one aggregate quality-profile name.
fn parse_quality_profile(value: &str) -> Result<QualityProfile, String> {
    match value {
        "pr" => Ok(QualityProfile::Pr),
        "release" => Ok(QualityProfile::Release),
        _ => Err(format!("unknown quality profile: {value}")),
    }
}
