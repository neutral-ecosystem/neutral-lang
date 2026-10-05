// SPDX-License-Identifier: Apache-2.0

//! commands / development responsibilities for repository automation.

use crate::constants::flags;
use crate::{
    BuildProfile, CiProfile, QualityProfile, TestLevel, check_boundaries, check_generated_outputs,
    check_repository_structure, check_test_layout, check_traceability, check_versions,
    check_workflow_contract, quality, release_prepare, run_cargo, run_recorded_workflow,
    test_suite, verify_environment, verify_optional_portable, verify_quality_ledger,
    verify_repository_markdown_links,
};

/// Runs the local auto-formatting and test loop without generating the documentation site.
pub(crate) fn develop() -> Result<(), String> {
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
        ],
    )
}

/// Checks or applies Rust formatting across the complete workspace.
pub(crate) fn format_workspace(write: bool) -> Result<(), String> {
    if write {
        run_cargo(&["fmt", "--all"])
    } else {
        run_cargo(&["fmt", "--all", "--", "--check"])
    }
}

/// Runs warning-free Clippy across every workspace target and feature.
pub(crate) fn lint() -> Result<(), String> {
    run_cargo(&[
        "clippy",
        flags::WORKSPACE,
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ])
}

/// Runs locked compilation plus repository boundary and coherence checks.
pub(crate) fn check() -> Result<(), String> {
    run_cargo(&["check", flags::WORKSPACE, "--all-targets", flags::LOCKED])?;
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

/// Runs the requested durable Cargo build profile.
pub(crate) fn build(profile: BuildProfile) -> Result<(), String> {
    match profile {
        BuildProfile::Dev => run_cargo(&["build", flags::WORKSPACE, flags::LOCKED]),
        BuildProfile::Release => {
            run_cargo(&["build", flags::WORKSPACE, "--release", flags::LOCKED])
        }
    }
}

/// Runs one internal CI profile using only stable public task compositions.
pub(crate) fn ci(profile: CiProfile) -> Result<(), String> {
    match profile {
        CiProfile::Pr => run_ci_gate("pr", QualityProfile::Pr),
        CiProfile::Release => release_prepare(),
    }
}

/// Runs a stable quality composition and writes its generated CI summary.
pub(crate) fn run_ci_gate(profile: &str, quality_profile: QualityProfile) -> Result<(), String> {
    run_recorded_workflow(
        "ci",
        profile,
        vec![
            ("environment", Box::new(verify_environment)),
            ("quality", Box::new(move || quality(quality_profile))),
        ],
    )
}
