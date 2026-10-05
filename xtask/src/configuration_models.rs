// SPDX-License-Identifier: Apache-2.0

//! Closed configuration schemas for repository automation.

use serde::Deserialize;
use std::collections::BTreeMap;

/// Release-authority settings remain a separate closed configuration.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Release {
    /// Configuration schema, not a release version.
    pub(super) schema_version: u32,
    /// Explicit publication authority.
    pub(super) distribution: Distribution,
}

/// Reviewed distribution channels and binary selection.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Distribution {
    /// Source-tag authority.
    pub(super) source_tag: bool,
    /// GitHub binary authority.
    pub(super) github_binaries: bool,
    /// Registry publication authority.
    pub(super) crates_io: bool,
    /// Package names for binary assets.
    pub(super) binaries: Vec<String>,
}

/// Extensible conformance manifest projected to its activation facts only.
#[derive(Debug, Deserialize)]
pub(super) struct ConformanceActivation {
    /// Empty plans need not yet declare active suites.
    #[serde(default)]
    pub(super) suite: Vec<SuiteActivation>,
}

/// Activation metadata independent of case ordering or future contract-specific fields.
#[derive(Debug, Deserialize)]
pub(super) struct SuiteActivation {
    /// Required or planned suite status.
    pub(super) status: String,
    /// Explicit stage for required suites.
    pub(super) active_from_stage: Option<u8>,
}

/// Versioned local automation settings, separate from release qualification.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Automation {
    /// Independent configuration schema.
    pub(super) schema_version: u32,
    /// Executable settings.
    pub(super) tools: Tools,
    /// Generated-output settings.
    pub(super) output: Output,
    /// Retained evidence defaults.
    pub(super) quality: Quality,
    /// Test execution settings.
    pub(super) testing: Testing,
}

/// Executables invoked directly, never through a shell.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Tools {
    /// Cargo executable.
    pub(super) cargo: String,
    /// Rust compiler executable.
    pub(super) rustc: String,
}

/// Generated-results defaults.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Output {
    /// Workspace-relative ignored result root.
    pub(super) results_root: String,
}

/// Evidence defaults, not success claims or quality thresholds.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Quality {
    /// Relative measurement directory.
    pub(super) evidence_root: String,
    /// Advisory report freshness bound.
    pub(super) advisory_max_age_seconds: u64,
}

/// Supported test backends; unknown selections are errors.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(super) enum TestRunner {
    /// Structured inventory and process-isolated execution.
    Nextest,
    /// Explicit compatibility backend.
    Cargo,
}

/// Test execution settings shared by local and CI commands.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Testing {
    /// Successful-test details are opt-in; failure output is never suppressed.
    #[serde(default)]
    pub(super) verbose: bool,
    /// Default test backend.
    pub(super) runner: TestRunner,
    /// Workspace-relative nextest configuration.
    pub(super) config: String,
    /// Local nextest profile.
    pub(super) profile: String,
    /// Full-gate nextest profile.
    pub(super) ci_profile: String,
}

/// One top-level responsibility boundary.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RepositoryDirectory {
    /// Relative root.
    pub(super) path: String,
    /// Ownership README.
    pub(super) readme: String,
    /// Human owner classification.
    pub(super) owner: String,
    /// Directory lifecycle.
    pub(super) lifecycle: String,
}

/// Closed directory inventory.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Layout {
    /// Configuration schema.
    pub(super) schema_version: u32,
    /// Directory responsibilities.
    pub(super) directory: Vec<RepositoryDirectory>,
}

/// One extensible named test-minimum profile.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct TestMinimumProfile {
    /// Category names and required counts.
    pub(super) minimum: BTreeMap<String, usize>,
}
