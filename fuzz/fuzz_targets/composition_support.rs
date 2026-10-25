// SPDX-License-Identifier: Apache-2.0

//! Bounded current expression-project capture and independent wire round trip.

use neutral_compiler::{
    CapturedExpressionProject, CapturedExpressionProjectRequest, CapturedProjectRequest,
    CapturedSourceInput, ProjectCaptureControls, ProjectCaptureLimitValues, ProjectCaptureLimits,
    capture_expression_project, compile_expression_project,
};
use neutral_core::{CancellationToken, StructuralLimits, profile::LanguageProfile};
use neutral_encoding::{
    DecodeLimits,
    expression::{decode_expression_project, encode_expression_project},
};
use neutral_ir::expression::{
    profile::ExpressionLimits, profile::SemanticProfiles, templates::ExpansionLimits,
};
use neutral_reader::{SemanticLimits, ValidatedExpressionProject};
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};
use std::sync::OnceLock;

pub fn capture(bytes: &[u8]) -> Option<CapturedExpressionProject> {
    let limits = ProjectCaptureLimitValues {
        total_source_bytes: 65_536,
        source_bytes_per_unit: 65_536,
        source_units: 4,
        source_id_bytes: 128,
        module_id_bytes: 128,
        vocabulary_units: 4,
        vocabulary_bytes_per_unit: 65_536,
        total_vocabulary_bytes: 65_536,
        imports_per_module: 16,
        import_edges: 64,
        scc_units: 4,
        declarations: 256,
        diagnostics: 16,
        output_bytes: 1_048_576,
    };
    capture_expression_project(CapturedExpressionProjectRequest::new(
        CapturedProjectRequest::new(
            neutral_ir::expression::profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "fuzz:source",
                "example",
                bytes.to_vec(),
            )],
            Vec::new(),
            ProjectCaptureControls::new(
                ProjectCaptureLimits::new(limits),
                CancellationToken::new(),
            ),
        ),
        SemanticProfiles::try_frozen().ok()?,
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(65_536, 32).ok()?,
        )),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    ))
    .ok()
}

pub fn semantic_limits() -> SemanticLimits {
    SemanticLimits {
        project: neutral_encoding::hard_project_limits(),
        composition: CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(65_536, 32).unwrap(),
        )),
        expression: ExpressionLimits::hard(),
        expansion: ExpansionLimits::hard(),
    }
}

pub fn encode(captured: &CapturedExpressionProject) -> Option<Vec<u8>> {
    let cancel = CancellationToken::new();
    let ir = compile_expression_project(captured, &cancel).ok()?;
    let project = ValidatedExpressionProject::from_ir(ir, semantic_limits(), &cancel).ok()?;
    encode_expression_project(&project, &cancel).ok()
}

pub fn mutated_seed(bytes: &[u8]) -> Vec<u8> {
    static SEED: OnceLock<Vec<u8>> = OnceLock::new();
    let mut seed = SEED
        .get_or_init(|| {
            encode(&capture(include_bytes!("../seeds/composition-source.neu")).unwrap()).unwrap()
        })
        .clone();
    if let Some((&first, tail)) = bytes.split_first() {
        let offset = usize::from(first) % seed.len();
        for (destination, replacement) in seed[offset..].iter_mut().zip(tail) {
            *destination = *replacement;
        }
    }
    seed
}

pub fn decode(bytes: &[u8]) -> Option<ValidatedExpressionProject> {
    decode_expression_project(
        bytes,
        DecodeLimits::hard(),
        semantic_limits(),
        &CancellationToken::new(),
    )
    .ok()
}
