// SPDX-License-Identifier: Apache-2.0

//! Owner package for executable cross-package Neutral tests.
//!
//! Smoke, integration, system, conformance, determinism, and security tests are
//! introduced here as their stages become active. This non-published crate must
//! not provide production APIs or duplicate normative fixtures.

#[cfg(test)]
/// Cross-package tests for active Stage 2 and Stage 3 vertical slices.
mod tests {
    use neutral_compiler::{
        CompilationFailureDetail, CompilationRequest, CompilationResult, LANGUAGE_BEHAVIOR_VERSION,
        capture, compile, compile_captured, diagnostics,
    };
    use neutral_core::{CancellationToken, ResultClass, StructuralLimits};
    use neutral_ir::{LOGICAL_IR_SCHEMA_VERSION, PROVENANCE_VERSION, SOURCE_MAP_VERSION};
    use neutral_probe::diagnostics as probe_diagnostics;
    use neutral_probe::{source_linked_diagnostic, summarize};
    use neutral_reader::ValidatedDocument;
    use std::{sync::Arc, thread};

    /// Compact expected rejection tuple used by frozen negative cases.
    type FailureOracle<'a> = (&'a [u8], ResultClass, &'a str, (u64, u64));

    /// Frozen positive minimal source fixture.
    const MINIMAL_SOURCE: &[u8] =
        include_bytes!("../../../portable/spec/v0/fixtures/positive/minimal-core.neu");
    /// Frozen missing-module negative fixture.
    const MISSING_MODULE: &[u8] =
        include_bytes!("../../../portable/spec/v0/fixtures/negative/missing-module-header.neu");
    /// Frozen unsupported-version negative fixture.
    const UNSUPPORTED_VERSION: &[u8] = include_bytes!(
        "../../../portable/spec/v0/fixtures/negative/unsupported-language-version.neu"
    );
    /// Frozen comment-equivalent positive fixture.
    const COMMENTS_SOURCE: &[u8] =
        include_bytes!("../../../portable/spec/v0/fixtures/positive/comments-equivalent.neu");
    /// Frozen identifier-boundary positive fixture.
    const IDENTIFIER_SOURCE: &[u8] =
        include_bytes!("../../../portable/spec/v0/fixtures/positive/identifier-boundaries.neu");
    /// Frozen invalid identifier fixture.
    const INVALID_IDENTIFIER: &[u8] =
        include_bytes!("../../../portable/spec/v0/fixtures/negative/invalid-identifier.neu");
    /// Frozen protected-name fixture.
    const PROTECTED_NAME: &[u8] =
        include_bytes!("../../../portable/spec/v0/fixtures/negative/protected-name.neu");
    /// Frozen unterminated block-comment fixture.
    const UNTERMINATED_COMMENT: &[u8] = include_bytes!(
        "../../../portable/spec/v0/fixtures/negative/unterminated-block-comment.neu"
    );
    /// Frozen unsupported-symbol fixture.
    const UNSUPPORTED_SYMBOL: &[u8] =
        include_bytes!("../../../portable/spec/v0/fixtures/negative/unsupported-symbol.neu");
    /// Frozen punctuation-rejection fixture.
    const PUNCTUATION_REJECTION: &[u8] =
        include_bytes!("../../../portable/spec/v0/fixtures/negative/punctuation-rejection.neu");
    /// Frozen comment/newline ambiguity fixture.
    const COMMENT_NEWLINE: &[u8] =
        include_bytes!("../../../portable/spec/v0/fixtures/negative/comment-newline-ambiguity.neu");
    /// Frozen adjacent string-token boundary fixture.
    const STRING_BOUNDARY: &[u8] =
        include_bytes!("../../../portable/spec/v0/fixtures/negative/string-token-boundary.neu");
    /// Frozen escaped Unicode string fixture.
    const STRING_SOURCE: &[u8] =
        include_bytes!("../../../portable/spec/v0/fixtures/positive/string-escapes-unicode.neu");
    /// Frozen true Boolean fixture.
    const BOOLEAN_TRUE: &[u8] =
        include_bytes!("../../../portable/spec/v0/fixtures/positive/boolean-true.neu");
    /// Frozen false Boolean fixture.
    const BOOLEAN_FALSE: &[u8] =
        include_bytes!("../../../portable/spec/v0/fixtures/positive/boolean-false.neu");

    /// Returns deterministic bounds for active scalar source slices.
    fn limits() -> StructuralLimits {
        StructuralLimits::new(1_024, 16).expect("scalar test limits should be valid")
    }

    /// Compiles source and returns shared authoritative artifacts.
    fn compile_artifacts(source: &[u8]) -> Arc<neutral_ir::CompilationArtifacts> {
        let request = CompilationRequest::new(source.to_vec(), limits(), CancellationToken::new());
        match compile(request).expect("nonempty bounded source should capture") {
            CompilationResult::Success(artifacts) => artifacts,
            CompilationResult::Failure(failure) => {
                panic!("minimal source unexpectedly failed: {:?}", failure.detail())
            }
        }
    }

    /// Compiles and opens one immutable validated reader document.
    fn compile_reader(source: &[u8]) -> ValidatedDocument {
        ValidatedDocument::from_compiler_output(compile_artifacts(source))
            .expect("compiler artifacts should validate")
    }

    /// Compiles one rejected source and returns its complete bounded failure.
    fn compile_failure(source: &[u8]) -> neutral_compiler::CompilationFailure {
        let request = CompilationRequest::new(source.to_vec(), limits(), CancellationToken::new());
        match compile(request).expect("bounded source should capture") {
            CompilationResult::Failure(failure) => failure,
            CompilationResult::Success(_) => panic!("negative source unexpectedly produced IR"),
        }
    }

    #[test]
    /// Verifies the active unit boundary produces exact minimal logical values.
    fn unit_minimal_logical_value_is_exact() {
        let artifacts = compile_artifacts(MINIMAL_SOURCE);
        let declaration = &artifacts.logical_document().declarations()[0];
        assert_eq!(declaration.name(), "answer");
        assert_eq!(declaration.resolved_type().to_string(), "num");
        assert_eq!(declaration.value().to_string(), "42/1");
    }

    #[test]
    /// Verifies compilation artifacts pass through the immutable reader boundary.
    fn integration_minimal_compiler_to_reader() {
        let document = compile_reader(MINIMAL_SOURCE);
        assert_eq!(document.module_name(), "minimal");
        assert_eq!(document.declarations().len(), 1);
        assert!(document.declaration_by_name("answer").is_some());
    }

    #[test]
    /// Verifies generic probe traversal and consumer diagnostics use reader views.
    fn system_minimal_reader_to_probe() {
        let document = compile_reader(MINIMAL_SOURCE);
        let summary = summarize(&document);
        assert_eq!(summary.module(), "minimal");
        assert_eq!(summary.declarations(), ["answer: num = 42/1"]);
        assert!(summary.diagnostics().is_empty());

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
    /// Verifies all frozen positive oracle fields for the Stage 2 source case.
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
    fn conformance_stage3_source_text_positive_oracles() {
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
    fn conformance_stage3_source_text_negative_oracles() {
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

    #[test]
    /// Verifies frozen string and Boolean values, source facts, and provenance.
    fn conformance_stage3_string_and_boolean_positive_oracles() {
        let cases = [
            (
                STRING_SOURCE,
                "string",
                "\"quote:\\\" slash:\\\\ newline:\\n nul:\\0 unicode:🙂 raw:é\"",
                152,
                (72, 151),
                (72, 78),
                (79, 86),
                (89, 151),
                neutral_ir::Normalization::StringEscapeDecoding,
                51,
            ),
            (
                BOOLEAN_TRUE,
                "bool",
                "true",
                92,
                (72, 91),
                (72, 76),
                (77, 84),
                (87, 91),
                neutral_ir::Normalization::BooleanIdentity,
                0,
            ),
            (
                BOOLEAN_FALSE,
                "bool",
                "false",
                93,
                (72, 92),
                (72, 76),
                (77, 84),
                (87, 92),
                neutral_ir::Normalization::BooleanIdentity,
                0,
            ),
        ];
        for (
            source,
            expected_type,
            expected_value,
            length,
            declaration,
            type_span,
            name_span,
            value_span,
            normalization,
            decoded_bytes,
        ) in cases
        {
            let artifacts = compile_artifacts(source);
            let binding = &artifacts.logical_document().declarations()[0];
            let mapping = artifacts
                .source_map()
                .entry(binding.element_id())
                .expect("scalar binding should have source facts");
            assert_eq!(binding.resolved_type().to_string(), expected_type);
            assert_eq!(binding.value().to_string(), expected_value);
            assert_eq!(artifacts.source_map().source_byte_length(), length);
            assert_eq!(span_pair(mapping.declaration_span()), declaration);
            assert_eq!(span_pair(mapping.type_span()), type_span);
            assert_eq!(span_pair(mapping.name_span()), name_span);
            assert_eq!(span_pair(mapping.value_span()), value_span);
            assert_eq!(artifacts.provenance()[0].normalization(), normalization);
            assert_eq!(
                artifacts
                    .derivation()
                    .resource_facts()
                    .decoded_string_bytes(),
                decoded_bytes
            );
        }
    }

    #[test]
    /// Verifies every frozen invalid string, Boolean, and version spelling.
    fn conformance_stage3_string_and_boolean_negative_oracles() {
        let cases: [FailureOracle<'_>; 9] = [
            (
                include_bytes!(
                    "../../../portable/spec/v0/fixtures/negative/string-unknown-escape.neu"
                ),
                ResultClass::Syntax,
                diagnostics::INVALID_STRING_LITERAL,
                (93, 95),
            ),
            (
                include_bytes!(
                    "../../../portable/spec/v0/fixtures/negative/string-invalid-surrogate.neu"
                ),
                ResultClass::Syntax,
                diagnostics::INVALID_STRING_LITERAL,
                (90, 98),
            ),
            (
                include_bytes!(
                    "../../../portable/spec/v0/fixtures/negative/string-out-of-range.neu"
                ),
                ResultClass::Syntax,
                diagnostics::INVALID_STRING_LITERAL,
                (90, 100),
            ),
            (
                include_bytes!(
                    "../../../portable/spec/v0/fixtures/negative/string-raw-control.neu"
                ),
                ResultClass::Syntax,
                diagnostics::INVALID_STRING_LITERAL,
                (93, 94),
            ),
            (
                include_bytes!(
                    "../../../portable/spec/v0/fixtures/negative/string-unterminated.neu"
                ),
                ResultClass::Syntax,
                diagnostics::UNTERMINATED_STRING_LITERAL,
                (89, 102),
            ),
            (
                include_bytes!(
                    "../../../portable/spec/v0/fixtures/negative/string-type-mismatch.neu"
                ),
                ResultClass::Semantics,
                diagnostics::TYPE_MISMATCH,
                (89, 93),
            ),
            (
                include_bytes!(
                    "../../../portable/spec/v0/fixtures/negative/invalid-boolean-literal.neu"
                ),
                ResultClass::Syntax,
                diagnostics::MALFORMED_BOUNDARY,
                (91, 92),
            ),
            (
                include_bytes!("../../../portable/spec/v0/fixtures/negative/version-escape.neu"),
                ResultClass::Syntax,
                diagnostics::UNSUPPORTED_LANGUAGE_VERSION,
                (4, 14),
            ),
            (
                include_bytes!(
                    "../../../portable/spec/v0/fixtures/negative/version-leading-zero.neu"
                ),
                ResultClass::Syntax,
                diagnostics::UNSUPPORTED_LANGUAGE_VERSION,
                (4, 10),
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

    #[test]
    /// Verifies reader traversal exposes typed values without reading source text.
    fn integration_reader_exposes_typed_string_and_boolean_values() {
        let string_document = compile_reader(STRING_SOURCE);
        assert!(matches!(
            string_document.declarations()[0].value(),
            neutral_ir::LogicalValue::String(_)
        ));
        let bool_document = compile_reader(BOOLEAN_FALSE);
        assert!(matches!(
            bool_document.declarations()[0].value(),
            neutral_ir::LogicalValue::Boolean(false)
        ));
    }

    #[test]
    /// Verifies probe rendering never emits decoded control characters directly.
    fn system_probe_escapes_hostile_string_controls() {
        let summary = summarize(&compile_reader(STRING_SOURCE));
        let rendered = &summary.declarations()[0];
        assert!(rendered.contains("\\n"));
        assert!(rendered.contains("\\0"));
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\r'));
        assert!(!rendered.contains('\t'));
        assert!(!rendered.contains('\0'));
    }

    #[test]
    /// Verifies all simple escapes and Unicode scalar boundaries decode exactly.
    fn property_string_escapes_and_unicode_scalar_boundaries() {
        let values = [
            r#""""#,
            r#""\"\\""#,
            r#""\n\r\t\0""#,
            r#""\u{0}""#,
            r#""\u{10ffff}""#,
            "\"é🙂\"",
        ];
        for value in values {
            let source = format!("neu \"0.1\"\nmodule scalar_strings\nstring message = {value}\n");
            assert!(matches!(
                compile(CompilationRequest::new(
                    source.into_bytes(),
                    limits(),
                    CancellationToken::new(),
                )),
                Ok(CompilationResult::Success(_))
            ));
        }
    }

    #[test]
    /// Verifies decoded string limits fail through the resource result boundary.
    fn security_decoded_string_limit_fails_before_ir_allocation() {
        let source = include_bytes!("../../../portable/spec/v0/fixtures/negative/string-limit.neu");
        let limits = StructuralLimits::new(1_024, 16)
            .expect("base limits should be valid")
            .with_string_bytes(8)
            .expect("string limit should be valid");
        let result = compile(CompilationRequest::new(
            source.to_vec(),
            limits,
            CancellationToken::new(),
        ))
        .expect("source bytes should remain within capture limits");
        let CompilationResult::Failure(failure) = result else {
            panic!("over-limit decoded string must produce no IR");
        };
        assert_eq!(failure.class(), ResultClass::Resource);
        assert_eq!(
            failure.detail(),
            CompilationFailureDetail::ResourceLimitExceeded
        );
        assert_eq!(
            failure.diagnostics()[0].code().as_str(),
            diagnostics::STRING_LIMIT_EXCEEDED
        );
        assert_eq!(
            span_pair(failure.diagnostics()[0].primary().span()),
            (89, 100)
        );
    }

    #[test]
    /// Verifies inserting or removing comments preserves all logical IR.
    fn property_comment_insertion_and_removal_preserves_logical_ir() {
        let plain = compile_artifacts(MINIMAL_SOURCE);
        let commented = compile_artifacts(COMMENTS_SOURCE);
        assert!(
            plain
                .logical_document()
                .logically_equivalent(commented.logical_document())
        );
        assert_ne!(plain.source_map(), commented.source_map());
    }

    #[test]
    /// Verifies generated ASCII identifier spellings match the frozen categories.
    fn property_ascii_identifier_boundaries_match_the_frozen_grammar() {
        for name in ["a", "a0", "a_b", "answer2_value3"] {
            let source = format!("neu \"0.1\"\nmodule {name}\nnum value = 42\n");
            assert!(matches!(
                compile(CompilationRequest::new(
                    source.into_bytes(),
                    limits(),
                    CancellationToken::new(),
                )),
                Ok(CompilationResult::Success(_))
            ));
        }
        for name in ["A", "_a", "a_", "a__b", "2a", "a-B", "é"] {
            let source = format!("neu \"0.1\"\nmodule {name}\nnum value = 42\n");
            assert!(matches!(
                compile(CompilationRequest::new(
                    source.into_bytes(),
                    limits(),
                    CancellationToken::new(),
                )),
                Ok(CompilationResult::Failure(_))
            ));
        }
    }

    #[test]
    /// Verifies unterminated and misleading nested comments fail deterministically.
    fn security_misleading_comments_fail_safely_and_deterministically() {
        let misleading = b"neu \"0.1\"\nmodule minimal\nnum answer = 42 /* outer /* inner */ */\n";
        for source in [UNTERMINATED_COMMENT, misleading.as_slice()] {
            let first = compile_failure(source);
            let second = compile_failure(source);
            assert_eq!(first, second);
            assert!(first.diagnostics().len() <= limits().diagnostics() as usize);
        }
    }

    #[test]
    /// Verifies planned Stage 3 grammar remains rejected until its own slice.
    fn security_future_grammar_is_not_accepted_by_source_text_work() {
        let future: [&[u8]; 4] = [
            include_bytes!("../../../portable/spec/v0/fixtures/negative/visibility-modifier.neu"),
            include_bytes!("../../../portable/spec/v0/fixtures/negative/reassignment.neu"),
            include_bytes!("../../../portable/spec/v0/fixtures/negative/namespace-declaration.neu"),
            include_bytes!("../../../portable/spec/v0/fixtures/negative/mut-modifier.neu"),
        ];
        for source in future {
            assert!(matches!(
                compile(CompilationRequest::new(
                    source.to_vec(),
                    limits(),
                    CancellationToken::new(),
                )),
                Ok(CompilationResult::Failure(_))
            ));
        }
    }

    #[test]
    /// Verifies formatting changes preserve meaning, symbol identity, and fingerprint.
    fn property_formatting_preserves_minimal_logical_identity() {
        let formatted = b"neu \"0.1\"\r\nmodule minimal\r\n\r\nnum\tanswer = 00042";
        let canonical = compile_artifacts(MINIMAL_SOURCE);
        let alternative = compile_artifacts(formatted);
        assert!(
            canonical
                .logical_document()
                .logically_equivalent(alternative.logical_document())
        );
        let left = &canonical.logical_document().declarations()[0];
        let right = &alternative.logical_document().declarations()[0];
        assert_eq!(left.symbol_identity(), right.symbol_identity());
        assert_eq!(left.fingerprint(), right.fingerprint());
        assert_ne!(
            canonical.source_map().source_digest(),
            alternative.source_map().source_digest()
        );
        assert_ne!(canonical.source_map(), alternative.source_map());
    }

    #[test]
    /// Verifies repeated compilation produces equal authoritative artifacts.
    fn property_repeated_compilation_is_deterministic() {
        assert_eq!(
            compile_artifacts(MINIMAL_SOURCE),
            compile_artifacts(MINIMAL_SOURCE)
        );
    }

    #[test]
    /// Verifies concurrent compilation produces equal authoritative artifacts.
    fn property_concurrent_compilation_is_deterministic() {
        let handles = (0..4)
            .map(|_| thread::spawn(|| compile_artifacts(MINIMAL_SOURCE)))
            .collect::<Vec<_>>();
        let results = handles
            .into_iter()
            .map(|handle| handle.join().expect("compiler thread should not panic"))
            .collect::<Vec<_>>();
        assert!(results.windows(2).all(|pair| pair[0] == pair[1]));
    }

    #[test]
    /// Verifies malformed minimal inputs always fail without authoritative output.
    fn security_malformed_inputs_never_produce_ir() {
        let malformed: [&[u8]; 5] = [
            b"\xff",
            b"neu \"0.1\"\0\nmodule minimal\nnum answer = 42\n",
            b"neu \"0.1\"\nmodule minimal\nnum answer = 4.2\n",
            b"neu \"0.1\"\nmodule minimal\nnum answer = --42\n",
            b"neu \"0.1\"\nmodule Minimal\nnum answer = 42\n",
        ];
        for source in malformed {
            let result = compile(CompilationRequest::new(
                source.to_vec(),
                limits(),
                CancellationToken::new(),
            ))
            .expect("bounded malformed source should capture");
            assert!(matches!(result, CompilationResult::Failure(_)));
        }
    }

    #[test]
    /// Verifies source byte limits fail before frontend or authoritative allocation.
    fn security_source_limit_fails_before_compilation() {
        let limits = StructuralLimits::new(8, 1).expect("test limits should be valid");
        assert!(
            compile(CompilationRequest::new(
                MINIMAL_SOURCE.to_vec(),
                limits,
                CancellationToken::new(),
            ))
            .is_err()
        );
    }

    #[test]
    /// Verifies bounded single-byte mutations terminate without panics or partial IR.
    fn fuzz_smoke_minimal_single_byte_mutations_terminate() {
        for index in 0..MINIMAL_SOURCE.len() {
            let mut mutation = MINIMAL_SOURCE.to_vec();
            mutation[index] ^= 0x80;
            let _ = compile(CompilationRequest::new(
                mutation,
                limits(),
                CancellationToken::new(),
            ));
        }
    }

    #[test]
    /// Verifies the complete minimal path remains runnable as a smoke gate.
    fn smoke_minimal_end_to_end_path_remains_runnable() {
        let summary = summarize(&compile_reader(MINIMAL_SOURCE));
        assert_eq!(summary.declarations().len(), 1);
    }

    /// Converts a checked byte span into a compact assertion pair.
    fn span_pair(span: neutral_core::ByteSpan) -> (u64, u64) {
        (span.start(), span.end())
    }
}
