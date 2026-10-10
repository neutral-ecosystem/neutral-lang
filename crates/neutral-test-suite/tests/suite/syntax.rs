// SPDX-License-Identifier: Apache-2.0

//! syntax cross-package regressions.

use super::*;

#[test]
/// Verifies generic probe traversal and consumer diagnostics use reader views.
fn system_minimal_reader_to_probe() {
    let document = compile_reader(MINIMAL_SOURCE);
    let summary = summarize(&document);
    assert_eq!(summary.module(), "minimal");
    assert_eq!(summary.declarations(), ["answer: num = 42/1"]);
    assert_eq!(summary.diagnostics().len(), 0);

    let element = document.declarations()[0].element_id();
    let diagnostic = source_linked_diagnostic(&document, element)
        .expect("known declaration should map to source");
    assert_eq!(diagnostic.code().as_str(), probe_diagnostics::OBSERVATION);
    assert_eq!(
        (
            diagnostic.primary().span().start(),
            diagnostic.primary().span().end()
        ),
        (26, 41)
    );
}

#[test]
/// Verifies all frozen positive oracle fields for the source case.
fn conformance_minimal_positive_oracle() {
    let artifacts = compile_artifacts(MINIMAL_SOURCE);
    let document = artifacts.logical_document();
    let declaration = &document.declarations()[0];
    let mapping = artifacts
        .source_map()
        .entry(declaration.element_id())
        .expect("declaration mapping should exist");
    assert_eq!(artifacts.source_map().source_byte_length(), 42);
    assert_eq!(
        artifacts.source_map().source_digest().to_string(),
        "sha256:2c0ba86566082520c52d6af9d772890512ae76e5b59dcd628ca64a398ae1522f"
    );
    assert_eq!(
        (
            artifacts.source_map().module_span().start(),
            artifacts.source_map().module_span().end()
        ),
        (10, 24)
    );
    assert_eq!(
        (
            mapping.declaration_span().start(),
            mapping.declaration_span().end()
        ),
        (26, 41)
    );
    assert_eq!(
        (mapping.type_span().start(), mapping.type_span().end()),
        (26, 29)
    );
    assert_eq!(
        (mapping.name_span().start(), mapping.name_span().end()),
        (30, 36)
    );
    assert_eq!(
        (mapping.value_span().start(), mapping.value_span().end()),
        (39, 41)
    );
    assert_eq!(
        artifacts.derivation().language_behavior_version(),
        LANGUAGE_BEHAVIOR_VERSION
    );
    assert_eq!(
        artifacts.derivation().logical_ir_schema_version(),
        LOGICAL_IR_SCHEMA_VERSION
    );
    assert_eq!(
        artifacts.derivation().source_map_version(),
        SOURCE_MAP_VERSION
    );
    assert_eq!(
        artifacts.derivation().provenance_version(),
        PROVENANCE_VERSION
    );
    assert_eq!(artifacts.derivation().resource_facts().declarations(), 1);
    assert_eq!(artifacts.derivation().resource_facts().diagnostics(), 0);
    assert_eq!(artifacts.provenance().len(), 1);
    assert_eq!(
        artifacts.provenance()[0].origin(),
        neutral_ir::ValueOrigin::ExplicitSource
    );
    assert_eq!(
        artifacts.provenance()[0].normalization(),
        neutral_ir::Normalization::ExactNumberCanonicalization
    );
}

#[test]
/// Verifies both frozen negative oracles return exact diagnostics and no IR.
fn conformance_minimal_negative_oracles() {
    let cases = [
        (MISSING_MODULE, diagnostics::MISSING_MODULE_HEADER, (10, 10)),
        (
            UNSUPPORTED_VERSION,
            diagnostics::UNSUPPORTED_LANGUAGE_VERSION,
            (4, 9),
        ),
    ];
    for (source, code, expected_span) in cases {
        let captured = capture(CompilationRequest::new(
            source.to_vec(),
            limits(),
            CancellationToken::new(),
        ))
        .expect("frozen negative source should capture");
        let CompilationResult::Failure(failure) = compile_captured(&captured) else {
            panic!("frozen negative source must expose no authoritative IR");
        };
        assert_eq!(failure.class(), ResultClass::Syntax);
        assert_eq!(failure.detail(), CompilationFailureDetail::SyntaxRejected);
        assert_eq!(failure.diagnostics()[0].code().as_str(), code);
        let span = failure.diagnostics()[0].primary().span();
        assert_eq!((span.start(), span.end()), expected_span);
    }
}

#[test]
/// Verifies all frozen Slice 3.1 positive source facts and reader output.
fn conformance_source_text_positive_oracles() {
    let cases = [
        (
            COMMENTS_SOURCE,
            "minimal",
            "answer",
            227,
            (130, 144),
            (170, 226),
            (170, 173),
            (198, 204),
            (224, 226),
        ),
        (
            IDENTIFIER_SOURCE,
            "minimal2_core",
            "answer2_value3",
            95,
            (50, 70),
            (71, 94),
            (71, 74),
            (75, 89),
            (92, 94),
        ),
    ];
    for (source, module, name, length, module_span, declaration, type_span, name_span, value) in
        cases
    {
        let artifacts = compile_artifacts(source);
        let mapping = artifacts
            .source_map()
            .entry(neutral_ir::ElementId::new(0))
            .expect("frozen declaration should have a source mapping");
        assert_eq!(artifacts.logical_document().module().module_name(), module);
        assert_eq!(artifacts.logical_document().declarations()[0].name(), name);
        assert_eq!(artifacts.source_map().source_byte_length(), length);
        assert_eq!(span_pair(artifacts.source_map().module_span()), module_span);
        assert_eq!(span_pair(mapping.declaration_span()), declaration);
        assert_eq!(span_pair(mapping.type_span()), type_span);
        assert_eq!(span_pair(mapping.name_span()), name_span);
        assert_eq!(span_pair(mapping.value_span()), value);
        let summary = summarize(&compile_reader(source));
        assert_eq!(summary.module(), module);
        assert_eq!(summary.declarations(), [format!("{name}: num = 42/1")]);
    }
}

#[test]
/// Verifies frozen Slice 3.1 failures expose exact classes, codes, and spans.
fn conformance_source_text_negative_oracles() {
    let cases = [
        (
            INVALID_IDENTIFIER,
            ResultClass::Semantics,
            diagnostics::INVALID_NAME,
            (57, 69),
        ),
        (
            PROTECTED_NAME,
            ResultClass::Semantics,
            diagnostics::PROTECTED_NAME,
            (69, 75),
        ),
        (
            UNTERMINATED_COMMENT,
            ResultClass::Syntax,
            diagnostics::UNTERMINATED_BLOCK_COMMENT,
            (81, 97),
        ),
        (
            UNSUPPORTED_SYMBOL,
            ResultClass::Syntax,
            diagnostics::UNSUPPORTED_SYMBOL,
            (80, 81),
        ),
        (
            PUNCTUATION_REJECTION,
            ResultClass::Syntax,
            diagnostics::UNSUPPORTED_SYMBOL,
            (64, 65),
        ),
        (
            COMMENT_NEWLINE,
            ResultClass::Syntax,
            diagnostics::MALFORMED_BOUNDARY,
            (132, 133),
        ),
        (
            STRING_BOUNDARY,
            ResultClass::Syntax,
            diagnostics::MALFORMED_BOUNDARY,
            (55, 56),
        ),
    ];
    for (source, class, code, expected_span) in cases {
        let failure = compile_failure(source);
        assert_eq!(failure.class(), class);
        assert_eq!(failure.diagnostics()[0].code().as_str(), code);
        assert_eq!(
            span_pair(failure.diagnostics()[0].primary().span()),
            expected_span
        );
    }
}
