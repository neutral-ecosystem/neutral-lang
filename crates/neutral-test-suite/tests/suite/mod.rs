// SPDX-License-Identifier: Apache-2.0

//! Cross-package tests for active Stage 2 through Stage 8 vertical slices.

use neutral_compiler::{
    CompilationFailureDetail, CompilationRequest, CompilationResult, LANGUAGE_BEHAVIOR_VERSION,
    capture, compile, compile_captured, diagnostics, format as format_source,
};
use neutral_core::{CancellationToken, ResultClass, StructuralLimits, VocabularyContentDigest};
use neutral_encoding::{
    DecodeErrorClass, DecodeLimits, EncodingError, ProducerInfo, SectionKind,
    constants as encoding, decode, encode,
};
use neutral_ir::{
    CompilationArtifacts, Declaration, LOGICAL_IR_SCHEMA_VERSION, LogicalDocument,
    PROVENANCE_VERSION, ReferenceProvenanceRecord, SOURCE_MAP_VERSION, ValueOrigin,
};
use neutral_probe::diagnostics as probe_diagnostics;
use neutral_probe::{source_linked_diagnostic, summarize};
use neutral_reader::{ReaderError, ValidatedDocument};
use neutral_vocabulary::{VOCABULARY_ENCODING_VERSION, VOCABULARY_SCHEMA_VERSION, VocabularyLock};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
};

/// Compact expected rejection tuple used by frozen negative cases.
type FailureOracle<'a> = (&'a [u8], ResultClass, &'a str, (u64, u64));

/// Frozen positive minimal source fixture.
const MINIMAL_SOURCE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/syntax/minimal-core.neu"
);
/// Frozen missing-module negative fixture.
const MISSING_MODULE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/syntax/missing-module-header.neu"
);
/// Frozen unsupported-version negative fixture.
const UNSUPPORTED_VERSION: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/syntax/unsupported-language-version.neu"
);
/// Frozen comment-equivalent positive fixture.
const COMMENTS_SOURCE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/syntax/comments-equivalent.neu"
);
/// Frozen identifier-boundary positive fixture.
const IDENTIFIER_SOURCE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/syntax/identifier-boundaries.neu"
);
/// Frozen invalid identifier fixture.
const INVALID_IDENTIFIER: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/identifiers/invalid-identifier.neu"
);
/// Frozen protected-name fixture.
const PROTECTED_NAME: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/identifiers/protected-name.neu"
);
/// Frozen unterminated block-comment fixture.
const UNTERMINATED_COMMENT: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/syntax/unterminated-block-comment.neu"
);
/// Frozen unsupported-symbol fixture.
const UNSUPPORTED_SYMBOL: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/syntax/unsupported-symbol.neu"
);
/// Frozen punctuation-rejection fixture.
const PUNCTUATION_REJECTION: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/syntax/punctuation-rejection.neu"
);
/// Frozen comment/newline ambiguity fixture.
const COMMENT_NEWLINE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/syntax/comment-newline-ambiguity.neu"
);
/// Frozen adjacent string-token boundary fixture.
const STRING_BOUNDARY: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/syntax/string-token-boundary.neu"
);
/// Frozen escaped Unicode string fixture.
const STRING_SOURCE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/strings/string-escapes-unicode.neu"
);
/// Frozen true Boolean fixture.
const BOOLEAN_TRUE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/booleans/boolean-true.neu"
);
/// Frozen false Boolean fixture.
const BOOLEAN_FALSE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/booleans/boolean-false.neu"
);
/// Frozen fraction-number positive fixture.
const NUMBER_FRACTION: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/numbers/number-fraction.neu"
);
/// Frozen exponent-number positive fixture.
const NUMBER_EXPONENT: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/numbers/number-exponent.neu"
);
/// Frozen separator-number positive fixture.
const NUMBER_SEPARATORS: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/numbers/number-separators.neu"
);
/// Frozen zero-number positive fixture.
const NUMBER_ZERO: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/numbers/number-zero.neu"
);
/// Frozen nullable string-null positive fixture.
const NULLABLE_STRING_NULL: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/nullability/nullable-string-null.neu"
);
/// Frozen nullable number-null positive fixture.
const NULLABLE_NUM_NULL: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/nullability/nullable-num-null.neu"
);
/// Frozen nullable Boolean-null positive fixture.
const NULLABLE_BOOL_NULL: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/nullability/nullable-bool-null.neu"
);
/// Frozen outer scalar-widening positive fixture.
const NULLABLE_SCALAR_WIDENING: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/nullability/nullable-scalar-widening.neu"
);
/// Frozen basic nominal-record positive fixture.
const NOMINAL_RECORD: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/records/nominal-record.neu"
);
/// Frozen forward record-collection positive fixture.
const RECORD_FORWARD_ORDER: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/records/record-forward-order.neu"
);
/// Frozen nested contextual-record positive fixture.
const NESTED_RECORD: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/records/nested-record.neu"
);
/// Frozen required/defaulted and nullable/non-nullable field-state fixture.
const FIELD_STATE_DEFAULTS: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/defaults/field-state-matrix.neu"
);
/// Frozen recursively closed contextual-record default fixture.
const NESTED_RECORD_DEFAULT: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/defaults/nested-record-default.neu"
);
/// Frozen explicit override of a defaulted field fixture.
const EXPLICIT_DEFAULT_OVERRIDE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/defaults/explicit-default-override.neu"
);
/// Frozen ordered string-list fixture.
const ORDERED_STRINGS: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/lists/ordered-strings.neu"
);
/// Frozen empty contextual-list fixture.
const EMPTY_LIST: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/lists/empty-list.neu"
);
/// Frozen nested list with nullable elements fixture.
const NESTED_NULLABLE_LIST: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/lists/nested-nullable-list.neu"
);
/// Frozen record/list/default combined fixture.
const RECORD_LIST_DEFAULT: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/lists/record-list-default.neu"
);
/// Frozen forward and transitive immutable-value reuse fixture.
const FORWARD_TRANSITIVE_REUSE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/reuse/forward-transitive.neu"
);
/// Frozen nested immutable-value reuse fixture.
const NESTED_REUSE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/reuse/nested-reuse.neu"
);
/// Frozen outer-nullable immutable-value reuse fixture.
const NULLABLE_REUSE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/reuse/nullable-widening.neu"
);
/// Frozen reuse with closed record/list defaults compatibility fixture.
const REUSE_DEFAULTS: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/values/defaults-compatibility.neu"
);
/// Frozen forward typed identity-reference fixture.
const FORWARD_REFERENCE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/references/forward-target.neu"
);
/// Frozen recursive nominal identity-cycle fixture.
const RECURSIVE_REFERENCE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/references/recursive-identity-cycle.neu"
);
/// Frozen field-name-neutral identity-reference fixture.
const FIELD_NAME_REFERENCE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/references/field-name-neutrality.neu"
);
/// Planned core fixture now activated by complete reuse and reference slices.
const COMBINED_REUSE_REFERENCE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/values/immutable-value-reuse.neu"
);
/// Frozen minimal qualified vocabulary source fixture.
const MINIMAL_VOCABULARY: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/positive/vocabulary/minimal-vocabulary.neu"
);
/// Exact accepted comprehensive vocabulary bundle bytes.
const VOCABULARY_BUNDLE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/vocabulary/bundles/positive/comprehensive.json"
);
/// Published language showcase containing one complete executable example.
const LANGUAGE_SHOWCASE: &str =
    include_str!("../../../../conformance/releases/v0.1.0/specs/examples/LANGUAGE-SHOWCASE.md");
/// Logically equivalent vocabulary bundle with different member order and bytes.
const REORDERED_VOCABULARY_BUNDLE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/vocabulary/bundles/positive/reordered.json"
);
/// Frozen fixture vocabulary logical identity.
const FIXTURE_VOCABULARY_IDENTITY: &str = "Fixture";
/// Frozen fixture vocabulary release version.
const FIXTURE_VOCABULARY_VERSION: &str = "0.1.0";
/// Package-metadata version used for nonsemantic test producer envelopes.
const TEST_PRODUCER_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Reproducible seed for the stable decoder mutation campaign.
const DECODER_FUZZ_SEED: u64 = 0x4e45_5554_5241_4c37;
/// Number of structured encoded-artifact mutation cases per campaign run.
const DECODER_MUTATION_CASES: usize = 2_048;
/// Number of arbitrary byte-sequence cases per campaign run.
const DECODER_ARBITRARY_CASES: usize = 1_024;
/// Maximum changes applied to one structured mutation.
const DECODER_MAX_CHANGES: usize = 8;
/// Maximum arbitrary input length exercised by the stable campaign.
const DECODER_MAX_ARBITRARY_BYTES: usize = 4_096;
/// Captured bundle requiring an unsupported structural feature.
const UNKNOWN_FEATURE_VOCABULARY_BUNDLE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/vocabulary/bundles/negative/unknown-feature.json"
);
/// Frozen missing captured vocabulary source fixture.
const MISSING_VOCABULARY_CAPTURE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/vocabulary/missing-capture.neu"
);
/// Frozen unknown qualified vocabulary type fixture.
const UNKNOWN_VOCABULARY_TYPE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/vocabulary/unknown-type.neu"
);
/// Frozen unknown vocabulary payload field fixture.
const UNKNOWN_VOCABULARY_FIELD: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/vocabulary/unknown-payload-field.neu"
);
/// Frozen incompatible vocabulary payload field fixture.
const WRONG_VOCABULARY_FIELD_TYPE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/vocabulary/wrong-payload-type.neu"
);
/// Frozen missing vocabulary payload field fixture.
const MISSING_VOCABULARY_FIELD: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/vocabulary/missing-payload-field.neu"
);
/// Frozen duplicate vocabulary payload field fixture.
const DUPLICATE_VOCABULARY_FIELD: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/vocabulary/duplicate-payload-field.neu"
);
/// Frozen vocabulary namespace collision fixture.
const VOCABULARY_NAME_COLLISION: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/vocabulary/vocabulary-name-collision.neu"
);
/// Frozen executable-shape hostile vocabulary bundle.
const EXECUTABLE_VOCABULARY_BUNDLE: &[u8] = include_bytes!(
    "../../../../conformance/releases/v0.1.0/specs/fixtures/vocabulary/bundles/negative/executable-member.json"
);

/// Returns deterministic bounds for active scalar source slices.
fn limits() -> StructuralLimits {
    StructuralLimits::new(1_024, 16).expect("scalar test limits should be valid")
}

/// Creates exact lock facts for the supplied immutable fixture bytes.
fn vocabulary_lock(bytes: &[u8]) -> VocabularyLock {
    VocabularyLock::new(
        FIXTURE_VOCABULARY_IDENTITY,
        FIXTURE_VOCABULARY_VERSION,
        VOCABULARY_ENCODING_VERSION,
        VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(bytes),
        Vec::new(),
    )
    .expect("frozen vocabulary lock should be valid")
}

/// Compiles source with one exact host-captured vocabulary input.
fn compile_with_vocabulary(source: &[u8], bytes: &[u8]) -> CompilationResult {
    let request = CompilationRequest::new(
        source.to_vec(),
        StructuralLimits::new(16_384, 16).expect("vocabulary limits should be valid"),
        CancellationToken::new(),
    )
    .with_captured_vocabulary(bytes.to_vec(), vocabulary_lock(bytes));
    compile(request).expect("bounded captured source should capture")
}

/// Formats source with the optional exact vocabulary fixture capture.
fn format_fixture(source: &[u8], uses_vocabulary: bool) -> Vec<u8> {
    let request = CompilationRequest::new(
        source.to_vec(),
        StructuralLimits::new(16_384, 16).expect("formatter limits should be valid"),
        CancellationToken::new(),
    );
    let request = if uses_vocabulary {
        request.with_captured_vocabulary(
            VOCABULARY_BUNDLE.to_vec(),
            vocabulary_lock(VOCABULARY_BUNDLE),
        )
    } else {
        request
    };
    format_source(request)
        .expect("positive fixture should format")
        .into_bytes()
}

/// Compiles source with the optional exact vocabulary fixture capture.
fn compile_fixture(source: &[u8], uses_vocabulary: bool) -> Arc<CompilationArtifacts> {
    if uses_vocabulary {
        let CompilationResult::Success(artifacts) =
            compile_with_vocabulary(source, VOCABULARY_BUNDLE)
        else {
            panic!("positive vocabulary source should compile");
        };
        artifacts
    } else {
        compile_artifacts(source)
    }
}

/// Compiles source and returns shared authoritative artifacts.
fn compile_artifacts(source: &[u8]) -> Arc<neutral_ir::CompilationArtifacts> {
    let request = CompilationRequest::new(source.to_vec(), limits(), CancellationToken::new());
    match compile(request).expect("nonempty bounded source should capture") {
        CompilationResult::Success(artifacts) => artifacts,
        CompilationResult::Failure(failure) => {
            panic!(
                "positive source unexpectedly failed: {:?} {:?}",
                failure.detail(),
                failure.diagnostics()
            )
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

/// Replaces only the logical payload while retaining companion artifacts for hostile tests.
fn replace_logical_document(
    artifacts: &CompilationArtifacts,
    logical_document: LogicalDocument,
) -> CompilationArtifacts {
    CompilationArtifacts::new(
        logical_document,
        artifacts.source_map().clone(),
        artifacts.provenance().to_vec(),
        artifacts.derivation().clone(),
    )
    .with_field_provenance(artifacts.field_provenance().to_vec())
    .with_reuse_provenance(artifacts.reuse_provenance().to_vec())
    .with_reference_provenance(artifacts.reference_provenance().to_vec())
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
/// Verifies frozen numeric spellings lower to exact normalized public values.
fn conformance_stage3_exact_number_positive_oracles() {
    let cases = [
        (NUMBER_FRACTION, "123e-2/1"),
        (NUMBER_EXPONENT, "-125e1/1"),
        (NUMBER_SEPARATORS, "16777216/1"),
        (NUMBER_ZERO, "0/1"),
    ];
    for (source, expected) in cases {
        let artifacts = compile_artifacts(source);
        let declaration = &artifacts.logical_document().declarations()[0];
        assert_eq!(declaration.value().to_string(), expected);
    }
}

#[test]
/// Verifies equivalent exact numeric spellings share definition fingerprints.
fn property_equivalent_number_spellings_normalize_and_fingerprint_equally() {
    let spellings = ["1.2300", "123e-2", "0.01230e2"];
    let artifacts = spellings.map(|spelling| {
        compile_artifacts(
            format!("neu \"0.1\"\nmodule numeric\nnum answer = {spelling}\n").as_bytes(),
        )
    });
    let first = &artifacts[0].logical_document().declarations()[0];
    for artifacts in &artifacts[1..] {
        let declaration = &artifacts.logical_document().declarations()[0];
        assert_eq!(declaration.value(), first.value());
        assert_eq!(declaration.fingerprint(), first.fingerprint());
    }
}

#[test]
/// Verifies frozen malformed numeric spellings produce stable semantic diagnostics.
fn conformance_stage3_exact_number_negative_oracles() {
    let cases = [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/numbers/number-invalid-separator.neu"
            ) as &[u8],
            (96, 100),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/numbers/number-missing-fraction.neu"
            ),
            (95, 97),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/numbers/number-invalid-exponent.neu"
            ),
            (95, 98),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/numbers/number-base-prefix.neu"
            ),
            (90, 94),
        ),
    ];
    for (source, span) in cases {
        let failure = compile_failure(source);
        assert_eq!(failure.class(), ResultClass::Semantics);
        assert_eq!(
            failure.diagnostics()[0].code().as_str(),
            diagnostics::INVALID_NUMBER
        );
        assert_eq!(span_pair(failure.diagnostics()[0].primary().span()), span);
    }
}

#[test]
/// Verifies digit and scale bounds reject values before exact-number allocation.
fn security_exact_number_limits_fail_before_ir_allocation() {
    let digit_limits = limits()
        .with_numeric_digits(4)
        .expect("numeric digit limit should be valid");
    let scale_limits = limits()
        .with_numeric_scale(2)
        .expect("numeric scale limit should be valid");
    for (source, limits, span) in [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/numbers/number-digit-limit.neu"
            ) as &[u8],
            digit_limits,
            (90, 95),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/numbers/number-scale-limit.neu"
            ),
            scale_limits,
            (90, 93),
        ),
    ] {
        let result = compile(CompilationRequest::new(
            source.to_vec(),
            limits,
            CancellationToken::new(),
        ))
        .expect("numeric fixture bytes should capture");
        let CompilationResult::Failure(failure) = result else {
            panic!("over-limit number must produce no IR");
        };
        assert_eq!(failure.class(), ResultClass::Resource);
        assert_eq!(
            failure.diagnostics()[0].code().as_str(),
            diagnostics::NUMBER_LIMIT_EXCEEDED
        );
        assert_eq!(span_pair(failure.diagnostics()[0].primary().span()), span);
    }
}

#[test]
/// Verifies nullable scalar null and outer widening fixtures through public IR.
fn conformance_stage3_nullable_scalar_positive_oracles() {
    let null_cases = [
        (NULLABLE_STRING_NULL, "string?"),
        (NULLABLE_NUM_NULL, "num?"),
        (NULLABLE_BOOL_NULL, "bool?"),
    ];
    for (source, expected_type) in null_cases {
        let artifacts = compile_artifacts(source);
        let declaration = &artifacts.logical_document().declarations()[0];
        assert_eq!(declaration.resolved_type().to_string(), expected_type);
        assert_eq!(declaration.value(), &neutral_ir::LogicalValue::Null);
        assert_eq!(
            artifacts.provenance()[0].normalization(),
            neutral_ir::Normalization::NullIdentity
        );
    }

    let widened = compile_artifacts(NULLABLE_SCALAR_WIDENING);
    let declaration = &widened.logical_document().declarations()[0];
    assert_eq!(declaration.resolved_type().to_string(), "string?");
    assert_eq!(declaration.value().to_string(), "\"present\"");
}

#[test]
/// Verifies reader/probe retain explicit null as a typed declaration value.
fn system_nullable_null_remains_distinct_from_absence() {
    let document = compile_reader(NULLABLE_STRING_NULL);
    let declaration = document
        .declaration_by_name("value")
        .expect("explicit null declaration must not be omitted");
    assert_eq!(declaration.resolved_type().to_string(), "string?");
    assert_eq!(declaration.value(), &neutral_ir::LogicalValue::Null);
    assert_eq!(
        summarize(&document).declarations(),
        ["value: string? = null"]
    );
}

#[test]
/// Verifies null and malformed nullability fail at their frozen boundaries.
fn conformance_stage3_nullable_scalar_negative_oracles() {
    let cases: [FailureOracle<'_>; 2] = [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/nullability/nonnullable-null.neu"
            ),
            ResultClass::Semantics,
            diagnostics::TYPE_MISMATCH,
            (89, 93),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/nullability/double-nullable.neu"
            ),
            ResultClass::Syntax,
            diagnostics::MALFORMED_BOUNDARY,
            (80, 81),
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
/// Verifies nullable type identity enters null definition fingerprints.
fn property_typed_null_fingerprints_remain_distinct() {
    let string = compile_artifacts(NULLABLE_STRING_NULL);
    let number = compile_artifacts(NULLABLE_NUM_NULL);
    let boolean = compile_artifacts(NULLABLE_BOOL_NULL);
    let fingerprints = [string, number, boolean]
        .map(|artifacts| artifacts.logical_document().declarations()[0].fingerprint());
    assert_ne!(fingerprints[0], fingerprints[1]);
    assert_ne!(fingerprints[1], fingerprints[2]);
    assert_ne!(fingerprints[0], fingerprints[2]);
}

#[test]
/// Verifies frozen nominal record fixtures through public logical IR.
fn conformance_stage4_nominal_record_positive_oracles() {
    let basic = compile_artifacts(NOMINAL_RECORD);
    let record = &basic.logical_document().record_types()[0];
    assert_eq!(record.name(), "Metadata");
    assert_eq!(
        record
            .fields()
            .iter()
            .map(|field| format!("{}: {}", field.name(), field.resolved_type()))
            .collect::<Vec<_>>(),
        ["enabled: bool", "image: string", "note: string?"]
    );
    let binding = &basic.logical_document().declarations()[0];
    assert_eq!(binding.resolved_type().to_string(), "Metadata");
    assert_eq!(
        binding.value().to_string(),
        "{enabled: true, image: \"example.invalid/tool:1\", note: null}"
    );

    let forward = compile_artifacts(RECORD_FORWARD_ORDER);
    assert_eq!(
        forward.logical_document().record_types()[0].name(),
        "Config"
    );
    assert_eq!(
        forward.logical_document().declarations()[0].name(),
        "config"
    );

    let nested = compile_artifacts(NESTED_RECORD);
    assert_eq!(nested.logical_document().record_types().len(), 2);
    assert_eq!(
        nested.logical_document().declarations()[0]
            .value()
            .to_string(),
        "{metadata: {name: \"neutral\"}}"
    );
}

#[test]
/// Verifies reader and probe expose nominal schemas and contextual values.
fn system_nominal_records_cross_reader_and_probe() {
    let document = compile_reader(NOMINAL_RECORD);
    assert_eq!(
        document
            .record_type_by_name("Metadata")
            .unwrap()
            .fields()
            .len(),
        3
    );
    let summary = summarize(&document);
    assert_eq!(
        summary.record_types(),
        ["record Metadata { enabled: bool, image: string, note: string? }"]
    );
    assert_eq!(
        summary.declarations(),
        ["metadata: Metadata = {enabled: true, image: \"example.invalid/tool:1\", note: null}"]
    );
}

#[test]
/// Verifies root and field source order cannot change logical record meaning.
fn property_record_declaration_and_field_order_is_nonsemantic() {
    let first = b"neu \"0.1\"\nmodule ordering\nrecord Config { string name, bool enabled, }\nConfig config = { name: \"x\", enabled: true, }\n";
    let second = b"neu \"0.1\"\nmodule ordering\nConfig config = { enabled: true, name: \"x\", }\nrecord Config { bool enabled, string name, }\n";
    let first = compile_artifacts(first);
    let second = compile_artifacts(second);
    assert!(
        first
            .logical_document()
            .logically_equivalent(second.logical_document())
    );
}

#[test]
/// Verifies frozen nominal record failures own stable codes and source spans.
fn conformance_stage4_nominal_record_negative_oracles() {
    let cases: [FailureOracle<'_>; 9] = [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/duplicate-declaration.neu"
            ),
            ResultClass::Semantics,
            diagnostics::DUPLICATE_DECLARATION,
            (114, 118),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/duplicate-schema-field.neu"
            ),
            ResultClass::Semantics,
            diagnostics::DUPLICATE_RECORD_FIELD,
            (121, 125),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/missing-value-field.neu"
            ),
            ResultClass::Semantics,
            diagnostics::MISSING_RECORD_FIELD,
            (142, 166),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/unknown-value-field.neu"
            ),
            ResultClass::Semantics,
            diagnostics::UNKNOWN_RECORD_FIELD,
            (151, 156),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/duplicate-value-field.neu"
            ),
            ResultClass::Semantics,
            diagnostics::DUPLICATE_VALUE_FIELD,
            (153, 157),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/wrong-kind-type.neu"
            ),
            ResultClass::Semantics,
            diagnostics::WRONG_DECLARATION_KIND,
            (120, 125),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/embedded-recursion.neu"
            ),
            ResultClass::Semantics,
            diagnostics::EMBEDDED_RECORD_RECURSION,
            (135, 140),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/field-shorthand.neu"
            ),
            ResultClass::Syntax,
            diagnostics::MALFORMED_BOUNDARY,
            (130, 131),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/records/wrong-field-type.neu"
            ),
            ResultClass::Semantics,
            diagnostics::TYPE_MISMATCH,
            (137, 142),
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
/// Verifies record field and nesting limits fail through the resource boundary.
fn security_record_limits_fail_before_schema_resolution() {
    let fields = b"neu \"0.1\"\nmodule limits\nrecord Item { string first, string second, }\n";
    let field_limits = limits()
        .with_record_fields(1)
        .expect("record field limit should be valid");
    let nested = b"neu \"0.1\"\nmodule limits\nrecord Inner { string name, }\nrecord Outer { Inner inner, }\nOuter value = { inner: { name: \"x\", }, }\n";
    let depth_limits = limits()
        .with_nesting_depth(1)
        .expect("record depth limit should be valid");
    for (source, limits) in [
        (fields.as_slice(), field_limits),
        (nested.as_slice(), depth_limits),
    ] {
        let result = compile(CompilationRequest::new(
            source.to_vec(),
            limits,
            CancellationToken::new(),
        ))
        .expect("bounded record source should capture");
        let CompilationResult::Failure(failure) = result else {
            panic!("over-limit record source must produce no IR");
        };
        assert_eq!(failure.class(), ResultClass::Resource);
        assert_eq!(
            failure.diagnostics()[0].code().as_str(),
            diagnostics::RECORD_LIMIT_EXCEEDED
        );
    }
}

#[test]
/// Verifies every required/defaulted and nullable/non-nullable field state.
fn conformance_stage4_closed_defaults_positive_oracles() {
    let matrix = compile_artifacts(FIELD_STATE_DEFAULTS);
    let schema = &matrix.logical_document().record_types()[0];
    assert_eq!(
        schema
            .fields()
            .iter()
            .map(|field| (field.name(), field.is_required()))
            .collect::<Vec<_>>(),
        [
            ("default_name", false),
            ("default_note", false),
            ("required_name", true),
            ("required_note", true),
        ]
    );
    assert_eq!(
        matrix.logical_document().declarations()[0]
            .value()
            .to_string(),
        "{default_name: \"fallback\", default_note: null, required_name: \"given\", required_note: null}"
    );
    assert_eq!(
        matrix
            .field_provenance()
            .iter()
            .map(|record| (record.field_path().join("."), record.origin()))
            .collect::<Vec<_>>(),
        [
            ("default_name".to_owned(), ValueOrigin::UserRecordDefault),
            ("default_note".to_owned(), ValueOrigin::UserRecordDefault),
            ("required_name".to_owned(), ValueOrigin::ExplicitRecordField),
            ("required_note".to_owned(), ValueOrigin::ExplicitRecordField),
        ]
    );

    let nested = compile_artifacts(NESTED_RECORD_DEFAULT);
    assert_eq!(
        nested.logical_document().declarations()[0]
            .value()
            .to_string(),
        "{metadata: {label: \"nested\"}}"
    );
    assert_eq!(
        nested.logical_document().record_types()[0].fields()[0]
            .default_value()
            .expect("closed record default must be materialized")
            .to_string(),
        "{label: \"nested\"}"
    );
}

#[test]
/// Verifies explicit and omitted fields retain final values and distinct provenance.
fn property_default_omission_changes_provenance_not_value_kind() {
    let omitted = compile_artifacts(
        b"neu \"0.1\"\nmodule same_default\nrecord Config { string name = \"same\", }\nConfig config = {}\n",
    );
    let explicit = compile_artifacts(
        b"neu \"0.1\"\nmodule same_default\nrecord Config { string name = \"same\", }\nConfig config = { name: \"same\", }\n",
    );
    assert!(
        omitted
            .logical_document()
            .logically_equivalent(explicit.logical_document())
    );
    assert_eq!(
        omitted.field_provenance()[0].origin(),
        ValueOrigin::UserRecordDefault
    );
    assert_eq!(
        explicit.field_provenance()[0].origin(),
        ValueOrigin::ExplicitRecordField
    );
}

#[test]
/// Verifies reader and probe expose final values and field provenance.
fn system_closed_defaults_cross_reader_and_probe() {
    let document = compile_reader(EXPLICIT_DEFAULT_OVERRIDE);
    let summary = summarize(&document);
    assert_eq!(
        summary.declarations(),
        ["config: Config = {name: \"explicit\"}"]
    );
    assert_eq!(
        summary.record_types(),
        ["record Config { name: string = \"fallback\" }"]
    );
    assert_eq!(summary.field_provenance().len(), 1);
    assert!(summary.field_provenance()[0].ends_with(":name:explicit-record-field"));
}

#[test]
/// Verifies the reader rejects record values with missing field evidence.
fn security_reader_rejects_incomplete_field_provenance() {
    let artifacts = compile_artifacts(FIELD_STATE_DEFAULTS)
        .as_ref()
        .clone()
        .with_field_provenance(Vec::new());
    assert_eq!(
        ValidatedDocument::from_compiler_output(Arc::new(artifacts))
            .expect_err("record fields without provenance must fail closed"),
        ReaderError::InvalidFieldProvenance
    );
}

#[test]
/// Verifies non-closed and ill-typed defaults fail with frozen ownership.
fn conformance_stage4_closed_defaults_negative_oracles() {
    let cases: [FailureOracle<'_>; 5] = [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/defaults/nonconstant-default.neu"
            ),
            ResultClass::Semantics,
            diagnostics::NON_CONSTANT_DEFAULT,
            (137, 143),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/defaults/reference-default.neu"
            ),
            ResultClass::Syntax,
            diagnostics::MALFORMED_BOUNDARY,
            (114, 124),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/defaults/expression-default.neu"
            ),
            ResultClass::Syntax,
            diagnostics::UNSUPPORTED_SYMBOL,
            (111, 112),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/defaults/wrong-default-type.neu"
            ),
            ResultClass::Semantics,
            diagnostics::TYPE_MISMATCH,
            (111, 115),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/defaults/missing-nested-required.neu"
            ),
            ResultClass::Semantics,
            diagnostics::MISSING_RECORD_FIELD,
            (159, 161),
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
/// Verifies ordered, empty, nested, nullable, and defaulted list values.
fn conformance_stage4_ordered_lists_positive_oracles() {
    let ordered = compile_artifacts(ORDERED_STRINGS);
    assert_eq!(
        ordered.logical_document().declarations()[0]
            .resolved_type()
            .to_string(),
        "List<string>"
    );
    assert_eq!(
        ordered.logical_document().declarations()[0]
            .value()
            .to_string(),
        "[\"first\", \"second\", \"third\"]"
    );
    assert_eq!(
        compile_artifacts(EMPTY_LIST)
            .logical_document()
            .declarations()[0]
            .value()
            .to_string(),
        "[]"
    );
    assert_eq!(
        compile_artifacts(NESTED_NULLABLE_LIST)
            .logical_document()
            .declarations()[0]
            .value()
            .to_string(),
        "[[\"first\", null], [], [\"last\"]]"
    );
    let combined = compile_artifacts(RECORD_LIST_DEFAULT);
    assert_eq!(
        combined.logical_document().declarations()[0]
            .value()
            .to_string(),
        "{items: [{name: \"default\"}], labels: []}"
    );
}

#[test]
/// Verifies malformed and heterogeneous list items have stable ownership.
fn conformance_stage4_ordered_lists_negative_oracles() {
    let cases: [FailureOracle<'_>; 2] = [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/lists/wrong-item-type.neu"
            ),
            ResultClass::Semantics,
            diagnostics::TYPE_MISMATCH,
            (105, 109),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/lists/missing-comma.neu"
            ),
            ResultClass::Syntax,
            diagnostics::MALFORMED_BOUNDARY,
            (107, 115),
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
/// Verifies item, nesting, and traversal limits fail before excess growth.
fn security_list_limits_fail_before_proportional_allocation() {
    let cases = [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/lists/item-limit.neu"
            ) as &[u8],
            limits()
                .with_list_items(2)
                .expect("list item limit should be valid"),
            (100, 101),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/lists/nesting-limit.neu"
            ),
            limits()
                .with_nesting_depth(1)
                .expect("list depth limit should be valid"),
            (103, 104),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/lists/traversal-limit.neu"
            ),
            limits()
                .with_traversal_nodes(2)
                .expect("traversal limit should be valid"),
            (102, 103),
        ),
    ];
    for (source, limits, expected_span) in cases {
        let result = compile(CompilationRequest::new(
            source.to_vec(),
            limits,
            CancellationToken::new(),
        ))
        .expect("bounded list fixture should capture");
        let CompilationResult::Failure(failure) = result else {
            panic!("over-limit list must produce no IR");
        };
        assert_eq!(failure.class(), ResultClass::Resource);
        assert_eq!(
            failure.diagnostics()[0].code().as_str(),
            diagnostics::LIST_LIMIT_EXCEEDED
        );
        assert_eq!(
            span_pair(failure.diagnostics()[0].primary().span()),
            expected_span
        );
    }
}

#[test]
/// Verifies list order is logical and generic arguments are invariant.
fn property_list_order_and_invariant_types_are_logical() {
    let first =
        compile_artifacts(b"neu \"0.1\"\nmodule order\nList<string> values = [\"a\", \"b\"]\n");
    let second =
        compile_artifacts(b"neu \"0.1\"\nmodule order\nList<string> values = [\"b\", \"a\"]\n");
    assert_ne!(
        first.logical_document().declarations()[0].fingerprint(),
        second.logical_document().declarations()[0].fingerprint()
    );
    assert_ne!(
        neutral_ir::ResolvedType::list(neutral_ir::ResolvedType::String),
        neutral_ir::ResolvedType::list(neutral_ir::ResolvedType::nullable(
            neutral_ir::ResolvedType::String,
        ))
    );
}

#[test]
/// Verifies lists and closed list defaults cross reader and probe boundaries.
fn system_ordered_lists_cross_reader_and_probe() {
    let document = compile_reader(RECORD_LIST_DEFAULT);
    let summary = summarize(&document);
    assert_eq!(
        summary.declarations(),
        ["config: Config = {items: [{name: \"default\"}], labels: []}"]
    );
    assert!(summary.record_types()[0].contains("List<Item>"));
    assert!(summary.record_types()[0].contains("List<string> = []"));
}

#[test]
/// Verifies forward, transitive, nested, and nullable reuse lower final values.
fn conformance_stage5_immutable_reuse_positive_oracles() {
    let forward = compile_artifacts(FORWARD_TRANSITIVE_REUSE);
    let declarations = forward.logical_document().declarations();
    assert_eq!(declarations.len(), 3);
    assert!(
        declarations
            .iter()
            .all(|declaration| declaration.value().to_string() == "\"example.invalid/tool:1\"")
    );
    assert_eq!(forward.reuse_provenance().len(), 2);
    assert!(
        forward
            .reuse_provenance()
            .iter()
            .all(|record| record.value_path().is_empty())
    );
    assert_eq!(
        forward
            .provenance()
            .iter()
            .filter(|record| record.origin() == ValueOrigin::OrdinaryReuse)
            .count(),
        2
    );

    let nested = compile_artifacts(NESTED_REUSE);
    let config = nested
        .logical_document()
        .declarations()
        .iter()
        .find(|declaration| declaration.name() == "config")
        .expect("nested reuse fixture must contain config");
    assert_eq!(
        config.value().to_string(),
        "{image: \"example.invalid/tool:1\", labels: [\"portable\", \"portable\"]}"
    );
    assert_eq!(nested.reuse_provenance().len(), 4);

    let nullable = compile_artifacts(NULLABLE_REUSE);
    assert_eq!(
        nullable
            .logical_document()
            .declarations()
            .iter()
            .find(|declaration| declaration.name() == "optional_image")
            .expect("nullable fixture must contain optional_image")
            .value()
            .to_string(),
        "\"example.invalid/tool:1\""
    );

    let defaults = compile_artifacts(REUSE_DEFAULTS);
    let config = defaults
        .logical_document()
        .declarations()
        .iter()
        .find(|declaration| declaration.name() == "config")
        .expect("reuse/default fixture must contain config");
    assert_eq!(
        config.value().to_string(),
        "{image: \"example.invalid/tool:1\", labels: [], note: null}"
    );
    assert_eq!(defaults.reuse_provenance().len(), 2);
}

#[test]
/// Verifies unresolved names, wrong kinds, cycles, and covariance produce no IR.
fn conformance_stage5_immutable_reuse_negative_oracles() {
    let cases = [
        (
            include_bytes!("../../../../conformance/releases/v0.1.0/specs/fixtures/negative/reuse/unknown-value.neu")
                .as_slice(),
            diagnostics::UNKNOWN_VALUE,
        ),
        (
            include_bytes!("../../../../conformance/releases/v0.1.0/specs/fixtures/negative/reuse/wrong-kind.neu")
                .as_slice(),
            diagnostics::WRONG_DECLARATION_KIND,
        ),
        (
            include_bytes!("../../../../conformance/releases/v0.1.0/specs/fixtures/negative/reuse/direct-cycle.neu")
                .as_slice(),
            diagnostics::VALUE_CYCLE,
        ),
        (
            include_bytes!("../../../../conformance/releases/v0.1.0/specs/fixtures/negative/reuse/indirect-cycle.neu")
                .as_slice(),
            diagnostics::VALUE_CYCLE,
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/reuse/generic-covariance.neu"
            )
            .as_slice(),
            diagnostics::TYPE_MISMATCH,
        ),
    ];
    for (source, expected_code) in cases {
        let failure = compile_failure(source);
        assert_eq!(failure.class(), ResultClass::Semantics);
        assert_eq!(failure.diagnostics()[0].code().as_str(), expected_code);
    }
    let direct = compile_failure(include_bytes!(
        "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/reuse/direct-cycle.neu"
    ));
    let indirect = compile_failure(include_bytes!(
        "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/reuse/indirect-cycle.neu"
    ));
    assert_eq!(direct.diagnostics()[0].related().len(), 1);
    assert_eq!(indirect.diagnostics()[0].related().len(), 3);
    assert_eq!(
        direct.diagnostics()[0]
            .related()
            .iter()
            .map(|location| span_pair(location.span()))
            .collect::<Vec<_>>(),
        [(92, 97)]
    );
    assert_eq!(
        indirect.diagnostics()[0]
            .related()
            .iter()
            .map(|location| span_pair(location.span()))
            .collect::<Vec<_>>(),
        [(94, 100), (117, 122), (138, 143)]
    );
}

#[test]
/// Verifies declaration order cannot change final reused values or fingerprints.
fn property_immutable_reuse_is_declaration_order_independent() {
    let forward = compile_artifacts(
        b"// SPDX-License-Identifier: Apache-2.0\nneu \"0.1\"\nmodule reuse_order\nstring copy = source\nstring source = \"value\"\n",
    );
    let reverse = compile_artifacts(
        b"// SPDX-License-Identifier: Apache-2.0\nneu \"0.1\"\nmodule reuse_order\nstring source = \"value\"\nstring copy = source\n",
    );
    assert_eq!(forward.logical_document(), reverse.logical_document());
}

#[test]
/// Verifies captured traversal bounds stop dependency-heavy source before IR.
fn security_immutable_reuse_chains_are_bounded() {
    let source = include_bytes!(
        "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/reuse/traversal-limit.neu"
    );
    let bounded = StructuralLimits::new(1_024, 16)
        .expect("reuse security limits must be valid")
        .with_traversal_nodes(3)
        .expect("reuse traversal bound must be valid");
    let request = CompilationRequest::new(source.to_vec(), bounded, CancellationToken::new());
    let CompilationResult::Failure(failure) =
        compile(request).expect("bounded reuse source should capture")
    else {
        panic!("bounded reuse source unexpectedly produced IR");
    };
    assert_eq!(failure.class(), ResultClass::Resource);
    assert_eq!(
        failure.diagnostics()[0].code().as_str(),
        diagnostics::LIST_LIMIT_EXCEEDED
    );
    assert_eq!(
        span_pair(failure.diagnostics()[0].primary().span()),
        (151, 158)
    );
}

#[test]
/// Verifies reader validation and probe output expose immutable reuse edges.
fn system_immutable_reuse_crosses_reader_and_probe() {
    let artifacts = compile_artifacts(FORWARD_TRANSITIVE_REUSE);
    let invalid = Arc::new(artifacts.as_ref().clone().with_reuse_provenance(Vec::new()));
    assert!(matches!(
        ValidatedDocument::from_compiler_output(invalid),
        Err(ReaderError::InvalidReuseProvenance)
    ));

    let document = ValidatedDocument::from_compiler_output(artifacts)
        .expect("compiler reuse provenance must validate");
    let summary = summarize(&document);
    assert_eq!(summary.reuse_provenance().len(), 2);
    assert!(
        summary
            .declarations()
            .iter()
            .all(|declaration| declaration.ends_with("= \"example.invalid/tool:1\""))
    );
}

#[test]
/// Verifies forward, nested, recursive, and combined typed reference fixtures.
fn conformance_stage5_typed_references_positive_oracles() {
    let forward = compile_artifacts(FORWARD_REFERENCE);
    let selected = forward
        .logical_document()
        .declarations()
        .iter()
        .find(|declaration| declaration.name() == "selected")
        .expect("forward reference fixture must contain selected");
    assert_eq!(selected.resolved_type().to_string(), "Ref<Config>");
    assert_eq!(selected.value().to_string(), "ref(#1)");
    assert_eq!(forward.reference_provenance().len(), 1);
    assert_eq!(
        forward.provenance()[1].origin(),
        ValueOrigin::IdentityReference
    );

    let recursive = compile_reader(RECURSIVE_REFERENCE);
    assert_eq!(recursive.declarations().len(), 2);
    assert_eq!(recursive.artifacts().reference_provenance().len(), 2);
    assert_eq!(
        recursive.record_types()[0].fields()[0]
            .resolved_type()
            .to_string(),
        "Ref<Node>?"
    );

    let fields = compile_artifacts(FIELD_NAME_REFERENCE);
    let edges = fields.reference_provenance();
    assert_eq!(edges.len(), 2);
    assert_eq!(edges[0].target_element_id(), edges[1].target_element_id());
    assert_eq!(edges[0].value_path(), ["dependency"]);
    assert_eq!(edges[1].value_path(), ["owner"]);

    let combined = compile_reader(COMBINED_REUSE_REFERENCE);
    assert_eq!(combined.artifacts().reuse_provenance().len(), 5);
    assert_eq!(combined.artifacts().reference_provenance().len(), 1);
}

#[test]
/// Verifies reference target name, kind, and exact type rejection boundaries.
fn conformance_stage5_typed_references_negative_oracles() {
    let cases = [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/references/unknown-target.neu"
            )
            .as_slice(),
            ResultClass::Reference,
            diagnostics::UNKNOWN_REFERENCE_TARGET,
            (103, 110),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/references/wrong-kind-target.neu"
            )
            .as_slice(),
            ResultClass::Reference,
            diagnostics::WRONG_REFERENCE_TARGET_KIND,
            (143, 149),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/references/wrong-target-type.neu"
            )
            .as_slice(),
            ResultClass::Reference,
            diagnostics::REFERENCE_TARGET_TYPE_MISMATCH,
            (143, 148),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/references/invariant-target-type.neu"
            )
            .as_slice(),
            ResultClass::Reference,
            diagnostics::REFERENCE_TARGET_TYPE_MISMATCH,
            (106, 111),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/references/missing-constructor.neu"
            )
            .as_slice(),
            ResultClass::Semantics,
            diagnostics::TYPE_MISMATCH,
            (126, 131),
        ),
    ];
    for (source, expected_class, expected_code, expected_span) in cases {
        let failure = compile_failure(source);
        assert_eq!(failure.class(), expected_class);
        assert_eq!(failure.diagnostics()[0].code().as_str(), expected_code);
        assert_eq!(
            span_pair(failure.diagnostics()[0].primary().span()),
            expected_span
        );
    }
}

#[test]
/// Verifies reference targets and fingerprints are independent of declaration order.
fn property_typed_reference_order_is_nonsemantic() {
    let forward = compile_artifacts(FORWARD_REFERENCE);
    let reordered = compile_artifacts(
        b"// SPDX-License-Identifier: Apache-2.0\nneu \"0.1\"\nmodule reference_forward\nrecord Config {\n string image,\n}\nConfig config = { image: \"example.invalid/tool:1\", }\nRef<Config> selected = ref(config)\n",
    );
    assert_eq!(forward.logical_document(), reordered.logical_document());
}

#[test]
/// Verifies ordinary reuse of a reference remains distinct and reader-valid.
fn property_reference_reuse_preserves_both_provenance_kinds() {
    let artifacts = compile_artifacts(
        b"// SPDX-License-Identifier: Apache-2.0\nneu \"0.1\"\nmodule reference_reuse\nRef<string> second = first\nRef<string> first = ref(value)\nstring value = \"value\"\n",
    );
    assert_eq!(artifacts.reuse_provenance().len(), 1);
    assert_eq!(artifacts.reference_provenance().len(), 2);
    ValidatedDocument::from_compiler_output(artifacts)
        .expect("reused references must retain complete edge provenance");
}

#[test]
/// Verifies reader and probe reject missing/dangling edges and traverse IDs.
fn system_typed_references_cross_reader_and_probe() {
    let artifacts = compile_artifacts(FORWARD_REFERENCE);
    let missing = Arc::new(
        artifacts
            .as_ref()
            .clone()
            .with_reference_provenance(Vec::new()),
    );
    assert!(matches!(
        ValidatedDocument::from_compiler_output(missing),
        Err(ReaderError::InvalidReferenceEdge)
    ));

    let owner = artifacts.reference_provenance()[0].element_id();
    let dangling = Arc::new(artifacts.as_ref().clone().with_reference_provenance(vec![
        ReferenceProvenanceRecord::new(owner, Vec::new(), owner),
    ]));
    assert!(matches!(
        ValidatedDocument::from_compiler_output(dangling),
        Err(ReaderError::InvalidReferenceEdge)
    ));

    let document = ValidatedDocument::from_compiler_output(artifacts)
        .expect("compiler identity edges must validate");
    let summary = summarize(&document);
    assert_eq!(summary.reference_provenance(), ["2::1"]);
    assert!(
        summary
            .declarations()
            .iter()
            .any(|value| value.ends_with("ref(#1)"))
    );
}

#[test]
/// Verifies duplicate IDs and stale fingerprints never produce validated views.
fn security_reader_rejects_invalid_graph_identity_states() {
    let artifacts = compile_artifacts(FORWARD_REFERENCE);
    let document = artifacts.logical_document();
    let mut declarations = document.declarations().to_vec();
    let duplicate_id = declarations[0].element_id();
    let original = declarations[1].clone();
    declarations[1] = Declaration::new(
        duplicate_id,
        original.symbol_identity().clone(),
        original.fingerprint(),
        original.name(),
        original.resolved_type().clone(),
        original.value().clone(),
    );
    let invalid_document = LogicalDocument::with_record_types(
        document.module().clone(),
        document.record_types().to_vec(),
        declarations,
    );
    let invalid = replace_logical_document(&artifacts, invalid_document);
    assert!(matches!(
        ValidatedDocument::from_compiler_output(Arc::new(invalid)),
        Err(ReaderError::DuplicateElementId)
    ));

    let mut declarations = document.declarations().to_vec();
    assert_ne!(declarations[0].fingerprint(), declarations[1].fingerprint());
    let original = declarations[1].clone();
    declarations[1] = Declaration::new(
        original.element_id(),
        original.symbol_identity().clone(),
        declarations[0].fingerprint(),
        original.name(),
        original.resolved_type().clone(),
        original.value().clone(),
    );
    let invalid_document = LogicalDocument::with_record_types(
        document.module().clone(),
        document.record_types().to_vec(),
        declarations,
    );
    let invalid = replace_logical_document(&artifacts, invalid_document);
    assert!(matches!(
        ValidatedDocument::from_compiler_output(Arc::new(invalid)),
        Err(ReaderError::InvalidDeclarationFingerprint)
    ));
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
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/strings/string-unknown-escape.neu"
            ),
            ResultClass::Syntax,
            diagnostics::INVALID_STRING_LITERAL,
            (93, 95),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/strings/string-invalid-surrogate.neu"
            ),
            ResultClass::Syntax,
            diagnostics::INVALID_STRING_LITERAL,
            (90, 98),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/strings/string-out-of-range.neu"
            ),
            ResultClass::Syntax,
            diagnostics::INVALID_STRING_LITERAL,
            (90, 100),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/strings/string-raw-control.neu"
            ),
            ResultClass::Syntax,
            diagnostics::INVALID_STRING_LITERAL,
            (93, 94),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/strings/string-unterminated.neu"
            ),
            ResultClass::Syntax,
            diagnostics::UNTERMINATED_STRING_LITERAL,
            (89, 102),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/strings/string-type-mismatch.neu"
            ),
            ResultClass::Semantics,
            diagnostics::TYPE_MISMATCH,
            (89, 93),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/booleans/invalid-boolean-literal.neu"
            ),
            ResultClass::Semantics,
            diagnostics::UNKNOWN_VALUE,
            (87, 91),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/syntax/version-escape.neu"
            ),
            ResultClass::Syntax,
            diagnostics::UNSUPPORTED_LANGUAGE_VERSION,
            (4, 14),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/syntax/version-leading-zero.neu"
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
    let source = include_bytes!(
        "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/strings/string-limit.neu"
    );
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

#[test]
/// Verifies qualified vocabulary data, defaults, derivation, reader, and probe end to end.
fn system_captured_vocabulary_crosses_reader_and_probe() {
    let CompilationResult::Success(artifacts) =
        compile_with_vocabulary(MINIMAL_VOCABULARY, VOCABULARY_BUNDLE)
    else {
        panic!("exact captured vocabulary fixture should compile");
    };
    let document = ValidatedDocument::from_compiler_output(artifacts)
        .expect("captured vocabulary artifacts should pass reader validation");
    let vocabulary = document
        .vocabulary()
        .expect("reader should expose exact vocabulary");
    assert_eq!(
        vocabulary.identity().identity(),
        FIXTURE_VOCABULARY_IDENTITY
    );
    assert_eq!(vocabulary.identity().version(), FIXTURE_VOCABULARY_VERSION);
    assert_eq!(
        vocabulary.identity().schema_version(),
        VOCABULARY_SCHEMA_VERSION
    );
    assert_eq!(
        vocabulary.identity().encoding_version(),
        VOCABULARY_ENCODING_VERSION
    );
    assert_eq!(
        vocabulary.identity().content_digest(),
        VocabularyContentDigest::from_bytes(VOCABULARY_BUNDLE)
    );
    let metadata = document
        .declaration_by_name("metadata")
        .expect("metadata should exist");
    assert_eq!(metadata.resolved_type().to_string(), "Fixture::Metadata");
    let defaults = document
        .artifacts()
        .field_provenance()
        .iter()
        .filter(|record| record.origin() == ValueOrigin::VocabularyDefault)
        .count();
    assert_eq!(defaults, 5);
    assert_eq!(
        document.artifacts().derivation().vocabulary(),
        Some(vocabulary.identity())
    );
    let summary = summarize(&document);
    let expected_summary_prefix =
        format!("{FIXTURE_VOCABULARY_IDENTITY}@{FIXTURE_VOCABULARY_VERSION}");
    assert!(
        summary
            .vocabulary()
            .is_some_and(|value| value.starts_with(&expected_summary_prefix))
    );
    assert!(
        summary
            .vocabulary_types()
            .iter()
            .any(|value| value.starts_with("record Fixture::Metadata"))
    );
}

#[test]
/// Verifies the frozen minimal source and exact bundle match their accepted oracle.
fn conformance_stage6_minimal_vocabulary_oracle() {
    let CompilationResult::Success(artifacts) =
        compile_with_vocabulary(MINIMAL_VOCABULARY, VOCABULARY_BUNDLE)
    else {
        panic!("minimal vocabulary oracle should accept");
    };
    assert_eq!(
        artifacts
            .logical_document()
            .vocabulary()
            .expect("oracle requires vocabulary")
            .identity()
            .content_digest(),
        VocabularyContentDigest::from_bytes(VOCABULARY_BUNDLE),
    );
}

#[test]
/// Verifies missing capture and unknown qualified types fail with frozen diagnostics.
fn conformance_stage6_vocabulary_resolution_failures() {
    let missing = compile_failure(MISSING_VOCABULARY_CAPTURE);
    assert_eq!(missing.class(), ResultClass::Vocabulary);
    assert_eq!(
        missing.diagnostics()[0].code().as_str(),
        diagnostics::MISSING_VOCABULARY
    );
    let CompilationResult::Failure(unknown) =
        compile_with_vocabulary(UNKNOWN_VOCABULARY_TYPE, VOCABULARY_BUNDLE)
    else {
        panic!("unknown qualified type must fail");
    };
    assert_eq!(
        unknown.diagnostics()[0].code().as_str(),
        diagnostics::UNKNOWN_VOCABULARY_TYPE
    );
}

#[test]
/// Verifies vocabulary payload fields use distinct closed-schema diagnostics.
fn conformance_stage6_vocabulary_payload_failures() {
    for (source, code) in [
        (
            UNKNOWN_VOCABULARY_FIELD,
            diagnostics::UNKNOWN_VOCABULARY_FIELD,
        ),
        (
            WRONG_VOCABULARY_FIELD_TYPE,
            diagnostics::VOCABULARY_FIELD_TYPE_MISMATCH,
        ),
        (
            MISSING_VOCABULARY_FIELD,
            diagnostics::MISSING_VOCABULARY_FIELD,
        ),
        (
            DUPLICATE_VOCABULARY_FIELD,
            diagnostics::DUPLICATE_VOCABULARY_FIELD,
        ),
        (
            VOCABULARY_NAME_COLLISION,
            diagnostics::VOCABULARY_NAME_COLLISION,
        ),
    ] {
        let CompilationResult::Failure(failure) =
            compile_with_vocabulary(source, VOCABULARY_BUNDLE)
        else {
            panic!("invalid vocabulary payload must fail");
        };
        assert_eq!(failure.class(), ResultClass::Vocabulary);
        assert_eq!(failure.diagnostics()[0].code().as_str(), code);
    }
}

#[test]
/// Verifies captured executable shapes fail before source payload validation.
fn security_vocabulary_executable_shape_precedes_payloads() {
    let CompilationResult::Failure(failure) =
        compile_with_vocabulary(WRONG_VOCABULARY_FIELD_TYPE, EXECUTABLE_VOCABULARY_BUNDLE)
    else {
        panic!("executable vocabulary content must fail");
    };
    assert_eq!(failure.class(), ResultClass::Vocabulary);
    assert_eq!(
        failure.diagnostics()[0].code().as_str(),
        diagnostics::EXECUTABLE_VOCABULARY_MEMBER
    );
}

#[test]
/// Verifies an unknown locked structural feature fails with its stable class.
fn security_vocabulary_unknown_feature_fails_closed() {
    let lock = VocabularyLock::new(
        FIXTURE_VOCABULARY_IDENTITY,
        FIXTURE_VOCABULARY_VERSION,
        VOCABULARY_ENCODING_VERSION,
        VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(UNKNOWN_FEATURE_VOCABULARY_BUNDLE),
        Vec::new(),
    )
    .expect("unknown feature lock should be structurally valid");
    let request = CompilationRequest::new(
        MINIMAL_VOCABULARY.to_vec(),
        StructuralLimits::new(16_384, 16).expect("vocabulary limits should be valid"),
        CancellationToken::new(),
    )
    .with_captured_vocabulary(UNKNOWN_FEATURE_VOCABULARY_BUNDLE.to_vec(), lock);
    let CompilationResult::Failure(failure) = compile(request).expect("source should capture")
    else {
        panic!("unknown vocabulary feature must fail");
    };
    assert_eq!(
        failure.diagnostics()[0].code().as_str(),
        diagnostics::UNKNOWN_VOCABULARY_FEATURE
    );
}

#[test]
/// Verifies source vocabulary requirements never perform implicit acquisition.
fn property_vocabulary_use_requires_identical_host_capture() {
    let first = compile_failure(MISSING_VOCABULARY_CAPTURE);
    let second = compile_failure(MISSING_VOCABULARY_CAPTURE);
    assert_eq!(first, second);
    assert_eq!(first.detail(), CompilationFailureDetail::VocabularyRejected);
}

#[test]
/// Verifies JSON member order changes captured facts but not logical meaning.
fn property_vocabulary_bundle_formatting_is_nonsemantic() {
    let CompilationResult::Success(canonical) =
        compile_with_vocabulary(MINIMAL_VOCABULARY, VOCABULARY_BUNDLE)
    else {
        panic!("canonical bundle should compile");
    };
    let CompilationResult::Success(reordered) =
        compile_with_vocabulary(MINIMAL_VOCABULARY, REORDERED_VOCABULARY_BUNDLE)
    else {
        panic!("reordered bundle should compile");
    };
    assert!(canonical.logically_equivalent(&reordered));
    assert_ne!(
        canonical.derivation().vocabulary(),
        reordered.derivation().vocabulary()
    );
}

#[test]
/// Verifies repeated exact vocabulary compilation is deterministic.
fn property_qualified_vocabulary_compilation_is_deterministic() {
    let first = compile_with_vocabulary(MINIMAL_VOCABULARY, VOCABULARY_BUNDLE);
    let second = compile_with_vocabulary(MINIMAL_VOCABULARY, VOCABULARY_BUNDLE);
    assert_eq!(first, second);
}

#[test]
/// Verifies omitted vocabulary fields become ordinary final values with companion provenance.
fn property_vocabulary_defaults_remain_ordinary_values() {
    let CompilationResult::Success(artifacts) =
        compile_with_vocabulary(MINIMAL_VOCABULARY, VOCABULARY_BUNDLE)
    else {
        panic!("minimal vocabulary should compile");
    };
    let metadata = artifacts
        .logical_document()
        .declarations()
        .iter()
        .find(|declaration| declaration.name() == "metadata")
        .expect("metadata declaration should exist");
    let neutral_ir::LogicalValue::VocabularyRecord(value) = metadata.value() else {
        panic!("metadata should retain its qualified contextual type");
    };
    assert_eq!(value.fields().len(), 6);
    assert_eq!(
        artifacts
            .field_provenance()
            .iter()
            .filter(|record| record.origin() == ValueOrigin::VocabularyDefault)
            .count(),
        5
    );
}

#[test]
/// Verifies an exact lock digest mismatch fails before source parsing.
fn security_vocabulary_lock_mismatch_precedes_source_parsing() {
    let lock = VocabularyLock::new(
        FIXTURE_VOCABULARY_IDENTITY,
        FIXTURE_VOCABULARY_VERSION,
        VOCABULARY_ENCODING_VERSION,
        VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(b"different"),
        Vec::new(),
    )
    .expect("mismatched lock should still be structurally valid");
    let request = CompilationRequest::new(
        b"not Neutral source".to_vec(),
        StructuralLimits::new(16_384, 16).expect("vocabulary limits should be valid"),
        CancellationToken::new(),
    )
    .with_captured_vocabulary(VOCABULARY_BUNDLE.to_vec(), lock);
    let CompilationResult::Failure(failure) = compile(request).expect("source should capture")
    else {
        panic!("lock mismatch must fail");
    };
    assert_eq!(
        failure.diagnostics()[0].code().as_str(),
        diagnostics::VOCABULARY_LOCK_MISMATCH
    );
}

#[test]
/// Verifies the reader fails closed when qualified data loses its exact contract.
fn security_reader_rejects_missing_vocabulary_contract() {
    let CompilationResult::Success(artifacts) =
        compile_with_vocabulary(MINIMAL_VOCABULARY, VOCABULARY_BUNDLE)
    else {
        panic!("exact captured vocabulary fixture should compile");
    };
    let logical = LogicalDocument::with_record_types(
        artifacts.logical_document().module().clone(),
        artifacts.logical_document().record_types().to_vec(),
        artifacts.logical_document().declarations().to_vec(),
    );
    let hostile = Arc::new(replace_logical_document(&artifacts, logical));
    assert_eq!(
        ValidatedDocument::from_compiler_output(hostile).unwrap_err(),
        ReaderError::InvalidVocabularyContract,
    );
}

#[test]
/// Verifies the external encoder emits the fixed frame and all five sections.
fn integration_stage7_encoder_emits_complete_fixed_frame() {
    let document = compile_reader(MINIMAL_SOURCE);
    let encoded = encode(&document, &ProducerInfo::new("test", TEST_PRODUCER_VERSION))
        .expect("validated fixture should encode");
    assert_eq!(
        &encoded.as_bytes()[..encoding::MAGIC.len()],
        &encoding::MAGIC
    );
    for kind in [
        SectionKind::Envelope,
        SectionKind::LogicalPayload,
        SectionKind::SourceMap,
        SectionKind::Provenance,
        SectionKind::Derivation,
    ] {
        assert!(!encoded.section_bytes(kind).is_empty());
    }
}

#[test]
/// Verifies an external artifact reconstructs exact immutable reader views.
fn system_stage7_encoded_artifact_reconstructs_reader_views() {
    let original = compile_reader(COMBINED_REUSE_REFERENCE);
    let encoded = encode(
        &original,
        &ProducerInfo::new("system-test", TEST_PRODUCER_VERSION),
    )
    .expect("system fixture should encode");
    let decoded = decode(
        encoded.as_bytes(),
        DecodeLimits::hard(),
        &CancellationToken::new(),
    )
    .expect("system artifact should decode");
    assert!(original.logically_equivalent(&decoded));
    assert_eq!(original.artifacts().as_ref(), decoded.artifacts().as_ref());
    assert_eq!(summarize(&original), summarize(&decoded));
}

#[test]
/// Verifies producer changes remain isolated from logical and companion sections.
fn property_stage7_producer_changes_are_envelope_only() {
    let document = compile_reader(MINIMAL_SOURCE);
    let first =
        encode(&document, &ProducerInfo::new("one", "1")).expect("first producer should encode");
    let second = encode(
        &document,
        &ProducerInfo::new("two", "2").with_build("build"),
    )
    .expect("second producer should encode");
    assert_ne!(
        first.section_bytes(SectionKind::Envelope),
        second.section_bytes(SectionKind::Envelope)
    );
    for kind in [
        SectionKind::LogicalPayload,
        SectionKind::SourceMap,
        SectionKind::Provenance,
        SectionKind::Derivation,
    ] {
        assert_eq!(first.section_bytes(kind), second.section_bytes(kind));
    }
}

#[test]
/// Verifies encoding is deterministic implementation behavior and never mutates input.
fn property_stage7_encoding_is_deterministic_and_nonmutating() {
    let document = compile_reader(COMBINED_REUSE_REFERENCE);
    let before = document.artifacts().as_ref().clone();
    let producer = ProducerInfo::new("test", TEST_PRODUCER_VERSION);
    assert_eq!(encode(&document, &producer), encode(&document, &producer));
    assert_eq!(document.artifacts().as_ref(), &before);
}

#[test]
/// Verifies every positive source fixture reaches bounded external encoding.
fn conformance_stage7_every_positive_fixture_encodes() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("test-suite crate should be inside the workspace");
    let mut fixtures = Vec::new();
    collect_positive_sources(
        &workspace.join("conformance/releases/v0.1.0/specs/fixtures/positive"),
        &mut fixtures,
    );
    assert!(!fixtures.is_empty());
    for fixture in fixtures {
        let source = fs::read(&fixture).expect("positive source should be readable");
        let document = if fixture
            .components()
            .any(|component| component.as_os_str() == "vocabulary")
        {
            let CompilationResult::Success(artifacts) =
                compile_with_vocabulary(&source, VOCABULARY_BUNDLE)
            else {
                panic!("positive vocabulary source should compile");
            };
            ValidatedDocument::from_compiler_output(artifacts)
                .expect("vocabulary output should validate")
        } else {
            compile_reader(&source)
        };
        let encoded = encode(
            &document,
            &ProducerInfo::new("fixture-test", TEST_PRODUCER_VERSION),
        )
        .expect("positive validated fixture should encode");
        assert!(encoded.as_bytes().len() <= encoding::MAXIMUM_ARTIFACT_BYTES);
        let decoded = decode(
            encoded.as_bytes(),
            DecodeLimits::hard(),
            &CancellationToken::new(),
        )
        .expect("positive encoded fixture should decode");
        assert_eq!(document.artifacts().as_ref(), decoded.artifacts().as_ref());
    }
}

#[test]
/// Verifies the exact canonical header, indentation, field, list, and spacing style.
fn conformance_stage8_formatter_emits_canonical_layout() {
    let source = b"/* license */ neu \"0.1\"\r\nmodule style\r\nrecord Item{string name,}\r\nList<Item>items=[{name:\"x\",},]\r\n";
    let expected = b"/* license */\nneu \"0.1\"\nmodule style\n\nrecord Item {\n    string name,\n}\n\nList<Item> items = [\n    {\n        name: \"x\",\n    },\n]\n";
    assert_eq!(format_fixture(source, false), expected);
}

#[test]
/// Verifies the published complete language example compiles with its captured bundle.
fn conformance_stage8_documentation_showcase_compiles() {
    let source = LANGUAGE_SHOWCASE
        .split_once("```neu\n")
        .and_then(|(_, remainder)| remainder.split_once("\n```"))
        .map(|(source, _)| source.as_bytes())
        .expect("showcase must contain one complete Neutral example first");
    assert!(matches!(
        compile_with_vocabulary(source, VOCABULARY_BUNDLE),
        CompilationResult::Success(_)
    ));
}

#[test]
/// Verifies formatting every positive source fixture is exactly idempotent.
fn property_stage8_formatter_is_idempotent_across_positive_corpus() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("test-suite crate should be inside the workspace");
    let mut fixtures = Vec::new();
    collect_positive_sources(
        &workspace.join("conformance/releases/v0.1.0/specs/fixtures/positive"),
        &mut fixtures,
    );
    assert!(!fixtures.is_empty());
    for fixture in fixtures {
        let source = fs::read(&fixture).expect("positive source should be readable");
        let uses_vocabulary = fixture
            .components()
            .any(|component| component.as_os_str() == "vocabulary");
        let once = format_fixture(&source, uses_vocabulary);
        let twice = format_fixture(&once, uses_vocabulary);
        assert_eq!(once, twice, "formatter was not idempotent: {fixture:?}");
    }
}

#[test]
/// Verifies formatting preserves logical IR and every accepted provenance category.
fn property_stage8_formatter_preserves_logic_and_provenance() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("test-suite crate should be inside the workspace");
    let mut fixtures = Vec::new();
    collect_positive_sources(
        &workspace.join("conformance/releases/v0.1.0/specs/fixtures/positive"),
        &mut fixtures,
    );
    for fixture in fixtures {
        let source = fs::read(&fixture).expect("positive source should be readable");
        let uses_vocabulary = fixture
            .components()
            .any(|component| component.as_os_str() == "vocabulary");
        let formatted = format_fixture(&source, uses_vocabulary);
        let before = compile_fixture(&source, uses_vocabulary);
        let after = compile_fixture(&formatted, uses_vocabulary);
        assert!(
            before.logically_equivalent(&after),
            "formatter changed logical IR: {fixture:?}"
        );
        assert_eq!(before.provenance(), after.provenance());
        assert_eq!(before.field_provenance(), after.field_provenance());
        assert_eq!(before.reuse_provenance(), after.reuse_provenance());
        assert_eq!(before.reference_provenance(), after.reference_provenance());
    }
}

#[test]
/// Verifies comment relocation is deterministic and remains logically irrelevant.
fn property_stage8_formatter_places_comments_deterministically() {
    let before_name = b"neu \"0.1\"\nmodule comments\nnum /* retained */ answer = 42\n";
    let before_type = b"neu \"0.1\"\nmodule comments\n/* retained */ num answer = 42\n";
    let first = format_fixture(before_name, false);
    let second = format_fixture(before_type, false);
    assert_eq!(first, second);
    assert!(
        std::str::from_utf8(&first)
            .expect("formatted source must be UTF-8")
            .contains("/* retained */\nnum answer = 42")
    );
    assert!(
        compile_fixture(before_name, false).logically_equivalent(&compile_fixture(&first, false))
    );
}

#[test]
/// Verifies formatted bytes acquire a distinct source identity without changing logic.
fn property_stage8_formatted_bytes_are_not_artifact_identity() {
    let source = b"neu \"0.1\"\r\nmodule identity\r\nnum\tanswer=00042\r\n";
    let formatted = format_fixture(source, false);
    assert_ne!(source.as_slice(), formatted);
    assert_ne!(
        neutral_core::SourceContentDigest::from_bytes(source),
        neutral_core::SourceContentDigest::from_bytes(&formatted)
    );
    assert!(
        compile_fixture(source, false).logically_equivalent(&compile_fixture(&formatted, false))
    );
}

#[test]
/// Verifies representative hostile framing, integrity, capability, and limit classes.
fn security_stage7_hostile_encoded_inputs_fail_boundedly() {
    let document = compile_reader(MINIMAL_SOURCE);
    let encoded = encode(
        &document,
        &ProducerInfo::new("security-test", TEST_PRODUCER_VERSION),
    )
    .expect("security fixture should encode");
    let cancellation = CancellationToken::new();
    assert_eq!(
        decode(
            &encoded.as_bytes()[..encoding::HEADER_BYTES - 1],
            DecodeLimits::hard(),
            &cancellation,
        )
        .expect_err("truncated frame must fail")
        .class(),
        DecodeErrorClass::MalformedFrame
    );
    let mut unknown_capability = encoded.as_bytes().to_vec();
    unknown_capability[40] = 1;
    assert_eq!(
        decode(&unknown_capability, DecodeLimits::hard(), &cancellation,)
            .expect_err("unknown capability must fail")
            .class(),
        DecodeErrorClass::UnsupportedCapability
    );
    let mut corrupt = encoded.as_bytes().to_vec();
    let last = corrupt.len() - 1;
    corrupt[last] ^= 1;
    assert_eq!(
        decode(&corrupt, DecodeLimits::hard(), &cancellation)
            .expect_err("corrupt section must fail")
            .class(),
        DecodeErrorClass::IntegrityMismatch
    );
    assert_eq!(
        decode(
            encoded.as_bytes(),
            DecodeLimits::hard().with_artifact_bytes(encoded.as_bytes().len() - 1),
            &cancellation,
        )
        .expect_err("host limit must fail")
        .class(),
        DecodeErrorClass::EncodedSizeLimit
    );
}

#[test]
/// Verifies arbitrary single-byte encoded mutations terminate without partial views.
fn fuzz_smoke_stage7_single_byte_mutations_terminate() {
    let document = compile_reader(MINIMAL_SOURCE);
    let encoded = encode(
        &document,
        &ProducerInfo::new("fuzz-test", TEST_PRODUCER_VERSION),
    )
    .expect("fuzz seed should encode");
    for index in 0..encoded.as_bytes().len() {
        let mut mutation = encoded.as_bytes().to_vec();
        mutation[index] ^= 1;
        if let Ok(decoded) = decode(&mutation, DecodeLimits::hard(), &CancellationToken::new()) {
            assert!(!decoded.module_name().is_empty());
        }
    }
}

#[test]
/// Fuzzes every truncation boundary and requires bounded fail-closed decoding.
fn fuzz_decoder_all_truncation_boundaries_fail_boundedly() {
    let document = compile_reader(MINIMAL_SOURCE);
    let encoded = encode(
        &document,
        &ProducerInfo::new("fuzz-truncation", TEST_PRODUCER_VERSION),
    )
    .expect("fuzz seed should encode");
    for end in 0..encoded.as_bytes().len() {
        assert!(
            decode(
                &encoded.as_bytes()[..end],
                DecodeLimits::hard(),
                &CancellationToken::new(),
            )
            .is_err(),
            "truncated input at byte {end} unexpectedly decoded"
        );
    }
}

#[test]
/// Fuzzes reproducible multi-byte mutations without permitting partial views.
fn fuzz_decoder_structured_mutation_campaign_terminates() {
    let document = compile_reader(MINIMAL_SOURCE);
    let encoded = encode(
        &document,
        &ProducerInfo::new("fuzz-structured", TEST_PRODUCER_VERSION),
    )
    .expect("fuzz seed should encode");
    let mut state = DECODER_FUZZ_SEED;
    for _ in 0..DECODER_MUTATION_CASES {
        let mut mutation = encoded.as_bytes().to_vec();
        let changes = 1 + fuzz_index(&mut state, DECODER_MAX_CHANGES);
        for _ in 0..changes {
            let index = fuzz_index(&mut state, mutation.len());
            mutation[index] ^= fuzz_nonzero_byte(&mut state);
        }
        if let Ok(decoded) = decode(&mutation, DecodeLimits::hard(), &CancellationToken::new()) {
            assert!(!decoded.module_name().is_empty());
        }
    }
}

#[test]
/// Fuzzes arbitrary deterministic byte sequences under the hard decoder ceiling.
fn fuzz_decoder_arbitrary_byte_campaign_terminates() {
    let mut state = DECODER_FUZZ_SEED.rotate_left(7);
    for _ in 0..DECODER_ARBITRARY_CASES {
        let length = fuzz_index(&mut state, DECODER_MAX_ARBITRARY_BYTES + 1);
        let mut bytes = vec![0_u8; length];
        for byte in &mut bytes {
            *byte = fuzz_word(&mut state).to_le_bytes()[0];
        }
        if let Ok(decoded) = decode(&bytes, DecodeLimits::hard(), &CancellationToken::new()) {
            assert!(!decoded.module_name().is_empty());
        }
    }
}

#[test]
/// Verifies an oversized envelope string fails before a complete frame exists.
fn security_stage7_oversized_producer_text_fails_boundedly() {
    let document = compile_reader(MINIMAL_SOURCE);
    let oversized = "x".repeat(encoding::MAXIMUM_TEXT_BYTES + 1);
    assert_eq!(
        encode(
            &document,
            &ProducerInfo::new(oversized, TEST_PRODUCER_VERSION),
        ),
        Err(EncodingError::EncodedSizeLimit)
    );
}

/// Recursively collects positive `.neu` files in deterministic path order.
fn collect_positive_sources(directory: &Path, files: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(directory)
        .expect("positive fixture directory should be readable")
        .map(|entry| {
            entry
                .expect("positive fixture entry should be readable")
                .path()
        })
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_positive_sources(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "neu") {
            files.push(path);
        }
    }
}

/// Advances the reproducible xorshift generator used by decoder campaigns.
fn fuzz_word(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

/// Selects a reproducible index strictly below a nonzero upper bound.
fn fuzz_index(state: &mut u64, upper_bound: usize) -> usize {
    debug_assert!(upper_bound > 0);
    usize::try_from(fuzz_word(state) % u64::try_from(upper_bound).unwrap_or(u64::MAX)).unwrap_or(0)
}

/// Selects a reproducible nonzero byte for an effective mutation.
fn fuzz_nonzero_byte(state: &mut u64) -> u8 {
    let candidate = fuzz_word(state).to_le_bytes()[0];
    candidate.max(1)
}

/// Converts a checked byte span into a compact assertion pair.
fn span_pair(span: neutral_core::ByteSpan) -> (u64, u64) {
    (span.start(), span.end())
}
