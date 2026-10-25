// SPDX-License-Identifier: Apache-2.0
//! Current expression-project decoder hardening regressions.
use neutral_compiler::{
    CapturedExpressionProjectRequest, CapturedProjectRequest, CapturedSourceInput,
    ProjectCaptureControls, ProjectCaptureLimitValues, ProjectCaptureLimits,
    capture_expression_project, compile_expression_project,
};
use neutral_core::{CancellationToken, StructuralLimits, profile::LanguageProfile};
use neutral_encoding::{
    DecodeLimits,
    expression::{decode_expression_project, encode_expression_project},
};
use neutral_ir::expression::{
    profile::{self, ExpressionLimits, SemanticProfiles},
    templates::ExpansionLimits,
};
use neutral_reader::{SemanticLimits, ValidatedExpressionProject};
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};

const SOURCE: &[u8] = b"neu \"1.0\"\nmodule hardening\npublic num answer = 42\n";
pub(super) fn artifact() -> (Vec<u8>, SemanticLimits) {
    let project_bytes = 1_048_576;
    let scalar = StructuralLimits::new(project_bytes, 64).unwrap();
    let cancel = CancellationToken::new();
    let captured = capture_expression_project(CapturedExpressionProjectRequest::new(
        CapturedProjectRequest::new(
            profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "suite:source",
                "hardening",
                SOURCE.to_vec(),
            )],
            Vec::new(),
            ProjectCaptureControls::new(
                ProjectCaptureLimits::new(ProjectCaptureLimitValues {
                    total_source_bytes: project_bytes,
                    source_bytes_per_unit: project_bytes,
                    source_units: 8,
                    source_id_bytes: 256,
                    module_id_bytes: 256,
                    vocabulary_units: 8,
                    vocabulary_bytes_per_unit: project_bytes,
                    total_vocabulary_bytes: project_bytes,
                    imports_per_module: 16,
                    import_edges: 64,
                    scc_units: 8,
                    declarations: 4096,
                    diagnostics: 64,
                    output_bytes: project_bytes,
                }),
                cancel.clone(),
            ),
        ),
        SemanticProfiles::try_frozen().unwrap(),
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(scalar)),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    ))
    .unwrap();
    let ir = compile_expression_project(&captured, &cancel).unwrap();
    let limits = SemanticLimits {
        project: ir.limits,
        composition: captured.composition_limits(),
        expression: captured.expression_limits(),
        expansion: captured.expansion_limits(),
    };
    let authority = ValidatedExpressionProject::from_ir(ir, limits, &cancel).unwrap();
    (
        encode_expression_project(&authority, &cancel).unwrap(),
        limits,
    )
}

#[test]
fn security_current_decoder_rejects_old_magic_and_truncation() {
    let (bytes, limits) = artifact();
    assert!(bytes.starts_with(b"NEUP3\r\n\x1a"));
    for old in [b"NEUIR\r\n\x1a", b"NEUPR\r\n\x1a", b"NEUP2\r\n\x1a"] {
        assert!(
            decode_expression_project(old, DecodeLimits::hard(), limits, &CancellationToken::new())
                .is_err()
        );
    }
    for end in 0..bytes.len() {
        assert!(
            decode_expression_project(
                &bytes[..end],
                DecodeLimits::hard(),
                limits,
                &CancellationToken::new()
            )
            .is_err(),
            "truncated at {end}"
        );
    }
}

#[test]
fn fuzz_smoke_current_artifact_byte_mutations_fail_or_decode_completely() {
    let (bytes, limits) = artifact();
    for index in 0..bytes.len() {
        let mut mutation = bytes.clone();
        mutation[index] ^= 1;
        let _ = decode_expression_project(
            &mutation,
            DecodeLimits::hard(),
            limits,
            &CancellationToken::new(),
        );
    }
    assert!(
        decode_expression_project(
            &bytes,
            DecodeLimits::hard().with_artifact_bytes(bytes.len() - 1),
            limits,
            &CancellationToken::new()
        )
        .is_err()
    );
}
