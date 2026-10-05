// SPDX-License-Identifier: Apache-2.0

//! commands / quality responsibilities for repository automation.

use crate::{
    BuildProfile, QualityAction, QualityProfile, TestLevel, ValidationTarget, WorkflowStep,
    approve_quality_release, build, check, constants, documentation, evaluate_quality,
    format_workspace, lint, quality_evidence, quality_status, render_quality_status, run_cargo,
    run_recorded_workflow, test_execution, test_suite, validate, verify_quality_ledger,
};

/// Dispatches one managed quality workflow action.
pub(crate) fn quality_action(action: QualityAction) -> Result<(), String> {
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
pub(crate) fn quality(profile: QualityProfile) -> Result<(), String> {
    let profile_name = quality_profile_name(profile);
    let mut steps: Vec<WorkflowStep<'_>> = vec![
        ("format", Box::new(|| format_workspace(false))),
        ("check", Box::new(check)),
        ("lint", Box::new(lint)),
        (
            "tests",
            Box::new(|| test_execution::run(TestLevel::All, true)),
        ),
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
                "dependency-advisories",
                Box::new(quality_evidence::advisory_scan)
                    as Box<dyn FnOnce() -> Result<(), String>>,
            ),
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

/// Verifies that every configured expensive quality gate has retained pass evidence.
pub(crate) fn verify_recorded_quality_gates() -> Result<(), String> {
    quality_evidence::verify_all()
}

/// Returns the stable command spelling for one quality profile.
pub(crate) fn quality_profile_name(profile: QualityProfile) -> &'static str {
    match profile {
        QualityProfile::Pr => "pr",
        QualityProfile::Release => "release",
    }
}
