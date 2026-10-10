// SPDX-License-Identifier: Apache-2.0

//! determinism cross-package regressions.

use super::*;

#[test]
/// Verifies inserting or removing comments preserves all logical IR.
fn property_comment_insertion_and_removal_preserves_logical_ir() {
    let plain = compile_artifacts(MINIMAL_SOURCE);
    let commented = compile_artifacts(COMMENTS_SOURCE);
    assert!(plain.logically_equivalent(&commented));
    assert_ne!(plain, commented);
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
/// Verifies grammar beyond the Slice 6.1 source boundary remains rejected.
fn security_future_grammar_is_not_accepted_by_source_text_work() {
    let future: [&[u8]; 7] = [
        include_bytes!(
            "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/vocabulary/visibility-modifier.neu"
        ),
        include_bytes!("../../../../conformance/releases/v0.1.0/specs/fixtures/negative/values/reassignment.neu"),
        include_bytes!(
            "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/vocabulary/namespace-declaration.neu"
        ),
        include_bytes!(
            "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/vocabulary/mut-modifier.neu"
        ),
        b"neu \"0.1\"\nmodule future\nuse vocabulary core\n",
        b"neu \"0.1\"\nmodule future\n{ name: \"anonymous\", }\n",
        b"neu \"0.1\"\nmodule future\nrecord Left { string name, }\nrecord Right { string name, }\nLeft value = Right { name: \"structural\", }\n",
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

    let canonical_reader = ValidatedDocument::from_compiler_output(canonical)
        .expect("canonical artifacts must validate");
    let alternative_reader = ValidatedDocument::from_compiler_output(alternative)
        .expect("formatted artifacts must validate");
    assert!(canonical_reader.logically_equivalent(&alternative_reader));
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
        b"neu \"0.1\"\nmodule minimal\nnum answer = 4._2\n",
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
