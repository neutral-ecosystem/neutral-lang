// SPDX-License-Identifier: Apache-2.0

//! One typed quality-policy snapshot per command; no global cache or stale defaults.

use crate::{
    configuration, configuration_array_from, constants, is_safe_relative_path, read_workspace_text,
    sha256_hex,
};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

/// Typed executable acceptance policy; historical status annotations confer no pass.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct QualityPolicy {
    /// Coverage command inputs and thresholds.
    pub(crate) coverage: CoverageSettings,
    /// Mutation target and acceptance threshold.
    pub(crate) mutation: MutationSettings,
    /// Fuzzer targets, seeds, and duration.
    pub(crate) fuzz: FuzzSettings,
    /// Controlled performance harness.
    pub(crate) performance: PerformanceSettings,
}

/// Coverage inputs used by both command construction and receipt validation.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct CoverageSettings {
    /// Generated HTML landing page relative to result root.
    pub(crate) html_output: PathBuf,
    /// Generated native JSON report relative to result root.
    pub(crate) json_output: PathBuf,
    /// Reviewed producer exclusions.
    pub(crate) exclusion_regex: String,
    /// Minimum measured line coverage percentage.
    pub(crate) minimum_line_percent: f64,
    /// Minimum measured function coverage percentage.
    pub(crate) minimum_function_percent: f64,
    /// Minimum measured region coverage percentage.
    pub(crate) minimum_region_percent: f64,
    /// Human review metadata retained but never interpreted as acceptance.
    #[serde(flatten)]
    _annotations: BTreeMap<String, toml::Value>,
}

/// Mutation inputs shared by the tool and independent result checks.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct MutationSettings {
    /// Reviewed nonempty set of workspace-relative production sources.
    pub(crate) critical_targets: Vec<String>,
    /// Required viable caught percentage.
    pub(crate) minimum_caught_percent: f64,
    /// Historical/descriptive annotations, not evidence.
    #[serde(flatten)]
    _annotations: BTreeMap<String, toml::Value>,
}

/// Fuzz campaign inputs shared by execution and duration/target validation.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct FuzzSettings {
    /// Nonempty set of configured target names.
    pub(crate) targets: Vec<String>,
    /// Mutable corpus directory relative to the workspace.
    pub(crate) corpus_root: PathBuf,
    /// Immutable seed directory relative to the workspace.
    pub(crate) seed_root: PathBuf,
    /// Required tool-reported duration for every target.
    pub(crate) minimum_seconds_per_target: u64,
    /// Ownership/readiness/status annotations, not execution results.
    #[serde(flatten)]
    _annotations: BTreeMap<String, toml::Value>,
}

/// Performance command selection; measured reports remain the acceptance authority.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct PerformanceSettings {
    /// Cargo package/benchmark target pair.
    pub(crate) harness: BenchmarkHarness,
    /// Required measured operations, shared by every performance receipt check.
    pub(crate) required_phases: Vec<String>,
    /// Human profile/status descriptions, not evidence.
    #[serde(flatten)]
    _annotations: BTreeMap<String, toml::Value>,
}

/// Validated Cargo benchmark selection, parsed once at the configuration boundary.
#[derive(Clone, Debug, Deserialize)]
#[serde(try_from = "String")]
pub(crate) struct BenchmarkHarness {
    /// Owning workspace package.
    pub(crate) package: String,
    /// Benchmark target within that package.
    pub(crate) target: String,
}

impl TryFrom<String> for BenchmarkHarness {
    type Error = String;

    /// Rejects unsupported package names and non-target syntax before dispatch.
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let (package, target) = value
            .split_once('/')
            .ok_or("quality performance harness must be package/target")?;
        if package != constants::NEUTRAL_BENCH
            || target.is_empty()
            || !target
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
        {
            return Err("invalid quality performance harness".to_owned());
        }
        Ok(Self {
            package: package.to_owned(),
            target: target.to_owned(),
        })
    }
}

impl QualityPolicy {
    /// Parses and validates all operational values before any measurement starts.
    pub(crate) fn parse(content: &str) -> Result<Self, String> {
        let policy: Self = configuration::parse(content, constants::QUALITY_GATES_FILE)?;
        for percentage in [
            policy.coverage.minimum_line_percent,
            policy.coverage.minimum_function_percent,
            policy.coverage.minimum_region_percent,
            policy.mutation.minimum_caught_percent,
        ] {
            if !percentage.is_finite() || !(0.0..=100.0).contains(&percentage) {
                return Err("quality percentages must be finite and between 0 and 100".to_owned());
            }
        }
        for path in [&policy.coverage.html_output, &policy.coverage.json_output] {
            if !crate::results::is_safe_result_path(path) {
                return Err("coverage outputs must be safe relative result paths".to_owned());
            }
        }
        let targets = &policy.mutation.critical_targets;
        if targets.is_empty()
            || targets
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != targets.len()
            || targets.iter().any(|target| {
                !is_safe_relative_path(Path::new(target))
                    || Path::new(target)
                        .extension()
                        .is_none_or(|extension| extension != "rs")
            })
            || !is_safe_relative_path(&policy.fuzz.corpus_root)
            || !is_safe_relative_path(&policy.fuzz.seed_root)
            || policy.fuzz.targets.is_empty()
            || policy
                .fuzz
                .targets
                .iter()
                .any(|t| !is_safe_relative_path(Path::new(t)))
            || policy.fuzz.minimum_seconds_per_target == 0
        {
            return Err(
                "quality targets and paths must be safe and the fuzz budget positive".to_owned(),
            );
        }
        let phases = &policy.performance.required_phases;
        if phases.is_empty()
            || phases
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len()
                != phases.len()
            || phases.iter().any(|phase| {
                phase.is_empty()
                    || !phase
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
            })
        {
            return Err("performance phases must be nonempty, unique operation names".to_owned());
        }
        Ok(policy)
    }
}

/// Immutable settings and exact policy bytes bound to one quality command.
#[derive(Clone)]
pub(crate) struct QualitySettings {
    /// Typed operational values.
    pub(crate) policy: QualityPolicy,
    /// Exact configuration bytes, including reviewed annotations.
    pub(crate) policy_sha256: String,
    /// Advisory freshness from typed automation settings.
    pub(crate) advisory_max_age_seconds: u64,
    /// Exact lock coverage from dependency policy.
    pub(crate) advisory_locks: Vec<String>,
    /// Evidence directory relative to the configured generated-result root.
    pub(crate) evidence_root: PathBuf,
}

impl QualitySettings {
    /// Reads quality, automation, and dependency policy once for this invocation.
    pub(crate) fn load(root: &Path) -> Result<Self, String> {
        let content = read_workspace_text(root, constants::QUALITY_GATES_FILE)?;
        let policy = QualityPolicy::parse(&content)?;
        let automation: crate::configuration_models::Automation =
            configuration::read(root, constants::AUTOMATION_CONFIG_FILE)?;
        configuration::require_schema(automation.schema_version, "automation configuration")?;
        if automation.quality.advisory_max_age_seconds == 0 {
            return Err("advisory freshness must be positive".to_owned());
        }
        let evidence_root = PathBuf::from(automation.quality.evidence_root);
        if !is_safe_relative_path(&evidence_root) {
            return Err("quality evidence root must be relative".to_owned());
        }
        let dependency = read_workspace_text(root, constants::DEPENDENCY_SOURCES_FILE)?;
        let mut advisory_locks = vec![constants::CARGO_LOCK_FILE.to_owned()];
        advisory_locks.extend(configuration_array_from(
            &dependency,
            "",
            "isolated_tool_lockfiles",
        )?);
        if advisory_locks
            .iter()
            .any(|lock| !is_safe_relative_path(Path::new(lock)))
        {
            return Err("unsafe advisory lockfile".to_owned());
        }
        Ok(Self {
            policy,
            policy_sha256: sha256_hex(content.as_bytes()),
            advisory_max_age_seconds: automation.quality.advisory_max_age_seconds,
            advisory_locks,
            evidence_root,
        })
    }
}

#[cfg(test)]
#[path = "../../tests/unit/quality_settings.rs"]
mod tests;
