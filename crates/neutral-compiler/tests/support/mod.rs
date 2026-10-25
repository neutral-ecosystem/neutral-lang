// SPDX-License-Identifier: Apache-2.0

//! Shared current `/3` capture fixture for private compiler boundary tests.

use crate::{
    CapturedExpressionProject, CapturedExpressionProjectRequest, CapturedProjectRequest,
    CapturedSourceInput, ProjectCaptureControls, ProjectCaptureLimitValues, ProjectCaptureLimits,
    capture_expression_project,
};
use neutral_core::{CancellationToken, StructuralLimits, profile::LanguageProfile};
use neutral_ir::expression::{profile, templates::ExpansionLimits};
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};

/// Creates a tiny `/3` capture with an accessible private composition context.
pub(crate) fn capture_expression_source(body: &str) -> CapturedExpressionProject {
    let bounds = ProjectCaptureLimitValues {
        total_source_bytes: 4096,
        source_bytes_per_unit: 4096,
        source_units: 4,
        source_id_bytes: 64,
        module_id_bytes: 64,
        vocabulary_units: 4,
        vocabulary_bytes_per_unit: 4096,
        total_vocabulary_bytes: 4096,
        imports_per_module: 4,
        import_edges: 4,
        scc_units: 4,
        declarations: 16,
        diagnostics: 16,
        output_bytes: 16_384,
    };
    let source = format!("neu \"1.0\"\nmodule example\n{body}\n");
    capture_expression_project(CapturedExpressionProjectRequest::new(
        CapturedProjectRequest::new(
            profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "unit",
                "example",
                source.into_bytes(),
            )],
            Vec::new(),
            ProjectCaptureControls::new(
                ProjectCaptureLimits::new(bounds),
                CancellationToken::new(),
            ),
        ),
        profile::SemanticProfiles::try_frozen().unwrap(),
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(4096, 16).unwrap(),
        )),
        profile::ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    ))
    .unwrap()
}
