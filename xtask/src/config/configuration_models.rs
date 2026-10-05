// SPDX-License-Identifier: Apache-2.0

//! Closed configuration schemas for repository automation.

use serde::Deserialize;
use std::collections::BTreeMap;

/// Release-authority settings remain a separate closed configuration.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Release {
    /// Configuration schema, not a release version.
    pub(crate) schema_version: u32,
    /// Explicit publication authority.
    pub(crate) distribution: Distribution,
}

/// Reviewed distribution channels and binary selection.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Distribution {
    /// Source-tag authority.
    pub(crate) source_tag: bool,
    /// GitHub binary authority.
    pub(crate) github_binaries: bool,
    /// Registry publication authority.
    pub(crate) crates_io: bool,
    /// Package names for binary assets.
    pub(crate) binaries: Vec<String>,
}

/// Extensible conformance manifest projected to its activation facts only.
#[derive(Debug, Deserialize)]
pub(crate) struct ConformanceActivation {
    /// Empty plans need not yet declare active suites.
    #[serde(default)]
    pub(crate) suite: Vec<SuiteActivation>,
}

/// Activation metadata independent of case ordering or future contract-specific fields.
#[derive(Debug, Deserialize)]
pub(crate) struct SuiteActivation {
    /// Required or planned suite status.
    pub(crate) status: String,
    /// Explicit stage for required suites.
    pub(crate) active_from_stage: Option<u8>,
}

/// Versioned local automation settings, separate from release qualification.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Automation {
    /// Independent configuration schema.
    pub(crate) schema_version: u32,
    /// Executable settings.
    pub(crate) tools: Tools,
    /// Generated-output settings.
    pub(crate) output: Output,
    /// Retained evidence defaults.
    pub(crate) quality: Quality,
    /// Test execution settings.
    pub(crate) testing: Testing,
}

/// Executables invoked directly, never through a shell.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Tools {
    /// Cargo executable.
    pub(crate) cargo: String,
    /// Rust compiler executable.
    pub(crate) rustc: String,
}

/// Generated-results defaults.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Output {
    /// Workspace-relative ignored result root.
    pub(crate) results_root: String,
}

/// Evidence defaults, not success claims or quality thresholds.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Quality {
    /// Relative measurement directory.
    pub(crate) evidence_root: String,
    /// Advisory report freshness bound.
    pub(crate) advisory_max_age_seconds: u64,
}

/// Supported test backends; unknown selections are errors.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TestRunner {
    /// Structured inventory and process-isolated execution.
    Nextest,
    /// Explicit compatibility backend.
    Cargo,
}

/// Test execution settings shared by local and CI commands.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Testing {
    /// Successful-test details are opt-in; failure output is never suppressed.
    #[serde(default)]
    pub(crate) verbose: bool,
    /// Default test backend.
    pub(crate) runner: TestRunner,
    /// Workspace-relative nextest configuration.
    pub(crate) config: String,
    /// Local nextest profile.
    pub(crate) profile: String,
    /// Full-gate nextest profile.
    pub(crate) ci_profile: String,
}

/// One top-level responsibility boundary.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RepositoryDirectory {
    /// Relative root.
    pub(crate) path: String,
    /// Ownership README.
    pub(crate) readme: String,
    /// Human owner classification.
    pub(crate) owner: String,
    /// Directory lifecycle.
    pub(crate) lifecycle: String,
}

/// Closed directory inventory.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Layout {
    /// Configuration schema.
    pub(crate) schema_version: u32,
    /// Directory responsibilities.
    pub(crate) directory: Vec<RepositoryDirectory>,
}

/// One extensible named test-minimum profile.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TestMinimumProfile {
    /// Category names and required counts.
    pub(crate) minimum: BTreeMap<String, usize>,
}
