// SPDX-License-Identifier: Apache-2.0

//! Owner package for executable cross-package Neutral tests.
//!
//! Smoke, integration, system, conformance, determinism, and security tests are
//! introduced here as their stages become active. This non-published crate must
//! not provide production APIs or duplicate normative fixtures.

#[cfg(test)]
/// Cross-package tests for the active Stage 2 minimal vertical slice.
mod tests {
    use neutral_compiler::{
        CompilationFailureDetail, CompilationRequest, CompilationResult, capture, compile,
        compile_captured,
    };
    use neutral_core::{CancellationToken, ResultClass, StructuralLimits};
    use neutral_probe::{source_linked_diagnostic, summarize};
    use neutral_reader::ValidatedDocument;
    use std::{sync::Arc, thread};

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

    /// Returns deterministic bounds for the minimal Stage 2 source slice.
    fn limits() -> StructuralLimits {
        StructuralLimits::new(1_024, 16).expect("Stage 2 test limits should be valid")
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
        assert_eq!(diagnostic.code().as_str(), "NEU-PROBE-001");
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
        assert_eq!(artifacts.derivation().language_behavior_version(), "0.1.0");
        assert_eq!(artifacts.derivation().logical_ir_schema_version(), "0.1.0");
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
            (MISSING_MODULE, "NEU-SYN-001", (10, 10)),
            (UNSUPPORTED_VERSION, "NEU-SYN-002", (4, 9)),
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
}
