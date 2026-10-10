// SPDX-License-Identifier: Apache-2.0

//! Cross-package tests for released syntax, values, artifacts, and host boundaries.

mod determinism;
mod formatter;
mod hostile_decoder;
mod lists;
mod nullable;
mod numbers;
mod records;
mod references;
mod reuse;
mod scalars;
mod syntax;
mod vocabulary;
mod wire;

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
