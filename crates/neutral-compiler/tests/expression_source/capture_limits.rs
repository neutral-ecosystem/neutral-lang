// SPDX-License-Identifier: Apache-2.0

//! Exact current-envelope bounds, independent of expression/model limits.

use super::*;

const SOURCE: &[u8] = b"neu \"1.0\"\nmodule project::main\nuse ExampleDomain as domain\n";

/// Tiny positive controls isolate capture from later compile/artifact publication.
fn controls() -> ProjectCaptureLimitValues {
    ProjectCaptureLimitValues {
        total_source_bytes: SOURCE.len() as u64,
        source_bytes_per_unit: SOURCE.len() as u64,
        source_units: 1,
        source_id_bytes: "source:main".len() as u64,
        module_id_bytes: "project::main".len() as u64,
        vocabulary_units: 1,
        vocabulary_bytes_per_unit: capture::EXPRESSION_VOCABULARY_BYTES.len() as u64,
        total_vocabulary_bytes: capture::EXPRESSION_VOCABULARY_BYTES.len() as u64,
        imports_per_module: 1,
        import_edges: 1,
        scc_units: 1,
        declarations: 1,
        diagnostics: 1,
        output_bytes: 1,
    }
}

/// Captures through the only production entry point, without any old-profile adapter.
fn with_controls(
    values: ProjectCaptureLimitValues,
) -> Result<CapturedExpressionProject, ExpressionCaptureFailure> {
    capture_expression_project(CapturedExpressionProjectRequest::new(
        CapturedProjectRequest::new(
            expression_profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "source:main",
                "project::main",
                SOURCE.to_vec(),
            )],
            vec![capture::expression_vocabulary()],
            ProjectCaptureControls::new(
                ProjectCaptureLimits::new(values),
                CancellationToken::new(),
            ),
        ),
        expression_profile::SemanticProfiles::try_frozen().unwrap(),
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(16_384, 16).unwrap(),
        )),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    ))
}

/// Every byte-retention ceiling accepts exact input and rejects one byte beyond it.
#[test]
fn expression_capture_byte_ceilings_are_inclusive_and_independent() {
    let exact = controls();
    assert!(with_controls(exact).is_ok());
    for reduce in [
        |v: &mut ProjectCaptureLimitValues| v.total_source_bytes -= 1,
        |v: &mut ProjectCaptureLimitValues| v.source_bytes_per_unit -= 1,
        |v: &mut ProjectCaptureLimitValues| v.source_id_bytes -= 1,
        |v: &mut ProjectCaptureLimitValues| v.module_id_bytes -= 1,
        |v: &mut ProjectCaptureLimitValues| v.vocabulary_bytes_per_unit -= 1,
        |v: &mut ProjectCaptureLimitValues| v.total_vocabulary_bytes -= 1,
    ] {
        let mut short = exact;
        reduce(&mut short);
        assert!(with_controls(short).is_err());
        let recovered = with_controls(exact).unwrap();
        assert_eq!(recovered.sources()[0].bytes(), SOURCE);
        assert_eq!(recovered.resource_facts().source_units(), 1);
    }
}

/// No dimension may be disabled, including controls enforced by later phases.
#[test]
fn expression_capture_rejects_each_zero_control_before_publication() {
    for zero in [
        |v: &mut ProjectCaptureLimitValues| v.total_source_bytes = 0,
        |v: &mut ProjectCaptureLimitValues| v.source_bytes_per_unit = 0,
        |v: &mut ProjectCaptureLimitValues| v.source_units = 0,
        |v: &mut ProjectCaptureLimitValues| v.source_id_bytes = 0,
        |v: &mut ProjectCaptureLimitValues| v.module_id_bytes = 0,
        |v: &mut ProjectCaptureLimitValues| v.vocabulary_units = 0,
        |v: &mut ProjectCaptureLimitValues| v.vocabulary_bytes_per_unit = 0,
        |v: &mut ProjectCaptureLimitValues| v.total_vocabulary_bytes = 0,
        |v: &mut ProjectCaptureLimitValues| v.imports_per_module = 0,
        |v: &mut ProjectCaptureLimitValues| v.import_edges = 0,
        |v: &mut ProjectCaptureLimitValues| v.scc_units = 0,
        |v: &mut ProjectCaptureLimitValues| v.declarations = 0,
        |v: &mut ProjectCaptureLimitValues| v.diagnostics = 0,
        |v: &mut ProjectCaptureLimitValues| v.output_bytes = 0,
    ] {
        let mut values = controls();
        zero(&mut values);
        let failure = with_controls(values)
            .err()
            .expect("zero control must reject");
        assert_eq!(failure.code(), ProjectCaptureError::LimitExceeded.code());
    }
}

/// Current capture observes cancellation at each integrity/header/publication handoff.
#[test]
fn expression_capture_every_phase_cancels_atomically_and_recovers() {
    use crate::project_capture::{ProjectCaptureCheckpoint as Phase, capture_expression_observed};
    for selected in [
        Phase::Start,
        Phase::Integrity,
        Phase::Headers,
        Phase::Publish,
    ] {
        let cancellation = CancellationToken::new();
        let request = super::request_with_inputs(
            vec![CapturedSourceInput::new(
                "source:main",
                "project::main",
                SOURCE.to_vec(),
            )],
            vec![capture::expression_vocabulary()],
            expression_profile::CAPTURE_REQUEST_VERSION,
            expression_profile::SemanticProfiles::try_frozen().unwrap(),
            CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
                StructuralLimits::new(16_384, 16).unwrap(),
            )),
            (ExpressionLimits::hard(), ExpansionLimits::hard()),
            cancellation.clone(),
        );
        let failed = capture_expression_observed(request, |phase| {
            if phase == selected {
                cancellation.cancel();
            }
        });
        assert!(
            failed
                .err()
                .expect("cancelled phase must reject")
                .is_cancelled()
        );
        assert!(with_controls(controls()).is_ok());
    }
}
