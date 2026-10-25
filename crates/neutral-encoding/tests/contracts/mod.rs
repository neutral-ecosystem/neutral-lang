// SPDX-License-Identifier: Apache-2.0

//! Shared bounded codec limits and safe error contracts.

use super::*;
use neutral_core::{CancellationToken, StructuralLimits};
use neutral_ir::expression::{profile::ExpressionLimits, templates::ExpansionLimits};
use neutral_reader::SemanticLimits;
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};

fn semantic_limits() -> SemanticLimits {
    SemanticLimits {
        project: hard_project_limits(),
        composition: CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(1_048_576, 64).unwrap(),
        )),
        expression: ExpressionLimits::hard(),
        expansion: ExpansionLimits::hard(),
    }
}

#[test]
fn decoder_limit_contract_applies_every_host_ceiling() {
    let limits = DecodeLimits::hard()
        .with_artifact_bytes(70)
        .with_section_bytes(60)
        .with_nesting_depth(50)
        .with_container_items(40)
        .with_text_bytes(30)
        .with_byte_string_bytes(20)
        .with_traversal_nodes(10);
    assert_eq!(limits.maximum_artifact_bytes(), 70);
    assert_eq!(limits.maximum_section_bytes(), 60);
    assert_eq!(limits.maximum_nesting_depth(), 50);
    assert_eq!(limits.maximum_container_items(), 40);
    assert_eq!(limits.maximum_text_bytes(), 30);
    assert_eq!(limits.maximum_byte_string_bytes(), 20);
    assert_eq!(limits.maximum_traversal_nodes(), 10);
    assert_eq!(
        DecodeLimits::hard()
            .with_artifact_bytes(usize::MAX)
            .maximum_artifact_bytes(),
        constants::MAXIMUM_ARTIFACT_BYTES
    );
}

#[test]
fn decoder_error_contract_exposes_safe_diagnostics() {
    let error = expression::decode_expression_project(
        b"",
        DecodeLimits::hard(),
        semantic_limits(),
        &CancellationToken::new(),
    )
    .expect_err("empty artifact must fail");
    assert_eq!(error.class(), DecodeErrorClass::MalformedFrame);
    assert_eq!(error.code(), DecodeErrorClass::MalformedFrame.code());
    assert_eq!(error.offset(), None);
}

#[test]
fn decoder_rejects_historical_project_magic_without_fallback() {
    for bytes in [b"NEUPR\r\n\x1a".as_slice(), b"NEUP2\r\n\x1a".as_slice()] {
        let error = expression::decode_expression_project(
            bytes,
            DecodeLimits::hard(),
            semantic_limits(),
            &CancellationToken::new(),
        )
        .unwrap_err();
        assert_eq!(error.class(), DecodeErrorClass::MalformedFrame);
    }
}
