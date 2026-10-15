// SPDX-License-Identifier: Apache-2.0

//! Successor composition source syntax diagnostics.

use super::*;
use crate::{
    CapturedCompositionProjectRequest, CapturedProjectRequest, CapturedSourceInput,
    ProjectCaptureControls, ProjectCaptureLimitValues, ProjectCaptureLimits,
    capture_composition_project,
};
use neutral_core::{StructuralLimits, profile::LanguageProfile};
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};

fn capture(body: &str) -> CapturedCompositionProject {
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
    capture_composition_project(CapturedCompositionProjectRequest::new(
        CapturedProjectRequest::new(
            profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new(
                "unit",
                "example",
                format!("neu \"1.0\"\nmodule example\n{body}\n").into_bytes(),
            )],
            Vec::new(),
            ProjectCaptureControls::new(
                ProjectCaptureLimits::new(bounds),
                CancellationToken::new(),
            ),
        ),
        profile::REQUIRED_FEATURES
            .iter()
            .map(|feature| (*feature).to_owned())
            .collect(),
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(4096, 16).unwrap(),
        )),
    ))
    .unwrap()
}

#[test]
fn malformed_declarations_and_members_return_syntax_diagnostics() {
    for (body, has_location) in [
        ("public num answer", true),
        ("public List answer = 1", true),
        ("public record Item { List field }", false),
    ] {
        let captured = capture(body);
        let failure = match parse(&captured.sources()[0], &captured, &CancellationToken::new()) {
            Err(failure) => failure,
            Ok(_) => panic!("accepted malformed composition source: {body}"),
        };
        assert_eq!(failure.code, codes::INVALID_SOURCE, "{body}");
        assert_eq!(failure.location.is_some(), has_location, "{body}");
    }
}

#[test]
fn inverted_declaration_spans_fail_as_invalid_source() {
    let captured = capture("public num answer = 1");
    let tokens = [
        Token {
            kind: TokenKind::Num,
            span: ByteSpan::new(2, 3).unwrap(),
        },
        Token {
            kind: TokenKind::Identifier("answer".to_owned()),
            span: ByteSpan::new(0, 1).unwrap(),
        },
    ];
    let failure = match parse_declaration(
        &captured.sources()[0],
        &tokens,
        captured.composition_limits(),
    ) {
        Err(failure) => failure,
        Ok(_) => panic!("accepted inverted declaration span"),
    };
    assert_eq!(failure.code, codes::INVALID_SOURCE);
    assert_eq!(failure.location, None);
}
