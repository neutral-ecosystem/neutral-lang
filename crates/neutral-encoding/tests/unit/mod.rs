// SPDX-License-Identifier: Apache-2.0

//! Unit tests for this package.

use super::{DecodeErrorClass, DecodeLimits, ProducerInfo, SectionKind, constants, decode, encode};
use neutral_compiler::{CompilationRequest, CompilationResult, capture, compile_captured};
use neutral_core::{
    CancellationToken, EncodedSectionDigest, StructuralLimits, VocabularyContentDigest,
};
use neutral_reader::ValidatedDocument;
use neutral_vocabulary::{VOCABULARY_ENCODING_VERSION, VOCABULARY_SCHEMA_VERSION, VocabularyLock};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Package-metadata version used for nonsemantic test producer envelopes.
const TEST_PRODUCER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Builds one validated scalar document for encoder tests.
fn validated(source: &[u8]) -> ValidatedDocument {
    let limits = StructuralLimits::new(16_384, 16).expect("test limits must be valid");
    let captured = capture(CompilationRequest::new(
        source.to_vec(),
        limits,
        CancellationToken::new(),
    ))
    .expect("fixture must capture");
    let CompilationResult::Success(artifacts) = compile_captured(&captured) else {
        panic!("fixture must compile");
    };
    ValidatedDocument::from_compiler_output(artifacts).expect("compiler output must validate")
}

/// Builds one validated document with an exact captured vocabulary bundle.
fn validated_with_vocabulary(source: &[u8], bundle: &[u8]) -> ValidatedDocument {
    let limits = StructuralLimits::new(16_384, 16).expect("test limits must be valid");
    let lock = VocabularyLock::new(
        "Fixture",
        "0.1.0",
        VOCABULARY_ENCODING_VERSION,
        VOCABULARY_SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(bundle),
        Vec::new(),
    )
    .expect("fixture vocabulary lock must be valid");
    let captured = capture(
        CompilationRequest::new(source.to_vec(), limits, CancellationToken::new())
            .with_captured_vocabulary(bundle.to_vec(), lock),
    )
    .expect("vocabulary fixture must capture");
    let CompilationResult::Success(artifacts) = compile_captured(&captured) else {
        panic!("vocabulary fixture must compile");
    };
    ValidatedDocument::from_compiler_output(artifacts)
        .expect("vocabulary compiler output must validate")
}

/// Recursively collects positive source fixture paths in stable order.
fn collect_neu_files(directory: &Path, files: &mut Vec<PathBuf>) {
    let mut entries = fs::read_dir(directory)
        .expect("fixture directory must be readable")
        .map(|entry| entry.expect("fixture entry must be readable").path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_neu_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "neu") {
            files.push(path);
        }
    }
}

/// Returns the first byte offset of one exact subsequence.
fn find_bytes(haystack: &[u8], needle: &[u8]) -> usize {
    haystack
        .windows(needle.len())
        .position(|candidate| candidate == needle)
        .expect("expected encoded subsequence must exist")
}

/// Mutates one section in place and refreshes its envelope integrity digest.
fn mutate_and_rehash(
    encoded: &super::EncodedArtifact,
    kind: SectionKind,
    mutate: impl FnOnce(&mut [u8]),
) -> Vec<u8> {
    let mut bytes = encoded.as_bytes().to_vec();
    let range = encoded.sections[kind.index()].clone();
    let old_digest = EncodedSectionDigest::from_bytes(&bytes[range.clone()]).as_bytes();
    mutate(&mut bytes[range.clone()]);
    let new_digest = EncodedSectionDigest::from_bytes(&bytes[range]).as_bytes();
    let envelope = encoded.sections[SectionKind::Envelope.index()].clone();
    let digest_offset = envelope.start + find_bytes(&bytes[envelope.clone()], &old_digest);
    bytes[digest_offset..digest_offset + old_digest.len()].copy_from_slice(&new_digest);
    bytes
}

/// Changes the scalar immediately following one text map key.
fn mutate_scalar_after_key(section: &mut [u8], key: &str, replacement: u8) {
    let offset = find_bytes(section, key.as_bytes()) + key.len();
    section[offset] = replacement;
}

/// Verifies the fixed frame and all five nonempty sections.
#[test]
fn encoder_emits_fixed_frame_and_sections() {
    let document = validated(b"neu \"0.1\"\nmodule sample\n\nnum answer = 42\n");
    let encoded = encode(&document, &ProducerInfo::new("test", TEST_PRODUCER_VERSION))
        .expect("validated document must encode");
    assert_eq!(&encoded.as_bytes()[..8], &constants::MAGIC);
    assert_eq!(
        u64::try_from(encoded.as_bytes().len()).expect("encoded length must fit u64"),
        u64::from_be_bytes(
            encoded.as_bytes()[16..24]
                .try_into()
                .expect("fixed total length field")
        )
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

/// Verifies one encoded artifact reconstructs an equivalent immutable reader.
#[test]
fn valid_artifact_decodes_to_an_equivalent_reader() {
    let document = validated(b"neu \"0.1\"\nmodule sample\n\nnum answer = 42\n");
    let encoded = encode(&document, &ProducerInfo::new("test", TEST_PRODUCER_VERSION))
        .expect("validated document must encode");
    let decoded = decode(
        encoded.as_bytes(),
        DecodeLimits::hard(),
        &CancellationToken::new(),
    )
    .expect("encoded document must decode");
    assert!(document.logically_equivalent(&decoded));
    assert_eq!(document.artifacts().as_ref(), decoded.artifacts().as_ref());
}

/// Verifies fixed-frame truncation and header corruption fail before CBOR parsing.
#[test]
fn hostile_frame_failures_are_classified() {
    let document = validated(b"neu \"0.1\"\nmodule sample\n\nnum answer = 42\n");
    let encoded = encode(&document, &ProducerInfo::new("test", TEST_PRODUCER_VERSION))
        .expect("fixture must encode");
    let cancellation = CancellationToken::new();
    for (bytes, class) in [
        (
            encoded.as_bytes()[..constants::HEADER_BYTES - 1].to_vec(),
            DecodeErrorClass::MalformedFrame,
        ),
        (
            {
                let mut bytes = encoded.as_bytes().to_vec();
                bytes[0] ^= 1;
                bytes
            },
            DecodeErrorClass::MalformedFrame,
        ),
        (
            {
                let mut bytes = encoded.as_bytes().to_vec();
                bytes[9] = 2;
                bytes
            },
            DecodeErrorClass::UnsupportedVersion,
        ),
        (
            {
                let mut bytes = encoded.as_bytes().to_vec();
                bytes[40] = 1;
                bytes
            },
            DecodeErrorClass::UnsupportedCapability,
        ),
    ] {
        assert_eq!(
            decode(&bytes, DecodeLimits::hard(), &cancellation)
                .expect_err("hostile frame must fail")
                .class(),
            class
        );
    }
}

/// Verifies envelope lexical, schema, version, and integrity failures stay distinct.
#[test]
fn hostile_envelope_failures_are_classified() {
    let document = validated(b"neu \"0.1\"\nmodule sample\n\nnum answer = 42\n");
    let encoded = encode(&document, &ProducerInfo::new("test", TEST_PRODUCER_VERSION))
        .expect("fixture must encode");
    let cancellation = CancellationToken::new();
    let mut malformed = encoded.as_bytes().to_vec();
    malformed[encoded.sections[0].start] = 0xfa;
    assert_eq!(
        decode(&malformed, DecodeLimits::hard(), &cancellation)
            .expect_err("float envelope must fail")
            .class(),
        DecodeErrorClass::MalformedCbor
    );

    let mut unknown = encoded.as_bytes().to_vec();
    let envelope = encoded.sections[0].clone();
    let producer = envelope.start + find_bytes(&unknown[envelope], b"producer");
    unknown[producer + b"producer".len() - 1] = b'x';
    assert_eq!(
        decode(&unknown, DecodeLimits::hard(), &cancellation)
            .expect_err("unknown envelope key must fail")
            .class(),
        DecodeErrorClass::InvalidEncodedSchema
    );

    let mut duplicate = encoded.as_bytes().to_vec();
    let envelope = encoded.sections[0].clone();
    let producer = envelope.start + find_bytes(&duplicate[envelope], b"producer");
    duplicate[producer..producer + b"producer".len()].copy_from_slice(b"versions");
    assert_eq!(
        decode(&duplicate, DecodeLimits::hard(), &cancellation)
            .expect_err("duplicate envelope key must fail")
            .class(),
        DecodeErrorClass::InvalidEncodedSchema
    );

    let mut version = encoded.as_bytes().to_vec();
    let envelope = encoded.sections[0].clone();
    let encoding = envelope.start + find_bytes(&version[envelope], constants::ENCODING.as_bytes());
    version[encoding + constants::ENCODING.len() - 1] = b'2';
    assert_eq!(
        decode(&version, DecodeLimits::hard(), &cancellation)
            .expect_err("unsupported encoding version must fail")
            .class(),
        DecodeErrorClass::UnsupportedVersion
    );

    let mut integrity = encoded.as_bytes().to_vec();
    let last = integrity.len() - 1;
    integrity[last] ^= 1;
    assert_eq!(
        decode(&integrity, DecodeLimits::hard(), &cancellation)
            .expect_err("section corruption must fail integrity")
            .class(),
        DecodeErrorClass::IntegrityMismatch
    );
}

/// Verifies authenticated hostile section states reach their owning validators.
#[test]
fn hostile_section_failures_are_classified() {
    let document = validated(b"neu \"0.1\"\nmodule sample\n\nnum answer = 42\n");
    let encoded = encode(&document, &ProducerInfo::new("test", TEST_PRODUCER_VERSION))
        .expect("fixture must encode");
    let cancellation = CancellationToken::new();

    let malformed = mutate_and_rehash(&encoded, SectionKind::LogicalPayload, |section| {
        section[0] = 0xfa;
    });
    assert_eq!(
        decode(&malformed, DecodeLimits::hard(), &cancellation)
            .expect_err("forbidden logical CBOR must fail")
            .class(),
        DecodeErrorClass::MalformedCbor
    );

    let schema = mutate_and_rehash(&encoded, SectionKind::LogicalPayload, |section| {
        let offset = find_bytes(section, b"record_types");
        section[offset + b"record_types".len() - 1] = b'z';
    });
    assert_eq!(
        decode(&schema, DecodeLimits::hard(), &cancellation)
            .expect_err("unknown logical key must fail")
            .class(),
        DecodeErrorClass::InvalidEncodedSchema
    );

    let fingerprint = document.declarations()[0].fingerprint().digest().as_bytes();
    let invalid_logical = mutate_and_rehash(&encoded, SectionKind::LogicalPayload, |section| {
        let offset = find_bytes(section, &fingerprint);
        section[offset] ^= 1;
    });
    assert_eq!(
        decode(&invalid_logical, DecodeLimits::hard(), &cancellation)
            .expect_err("invalid fingerprint must fail")
            .class(),
        DecodeErrorClass::InvalidLogicalIr
    );

    let invalid_name = mutate_and_rehash(&encoded, SectionKind::LogicalPayload, |section| {
        let offset = find_bytes(section, b"sample");
        section[offset] = b'S';
    });
    assert_eq!(
        decode(&invalid_name, DecodeLimits::hard(), &cancellation)
            .expect_err("invalid module name must fail")
            .class(),
        DecodeErrorClass::InvalidLogicalIr
    );

    let invalid_source = mutate_and_rehash(&encoded, SectionKind::SourceMap, |section| {
        mutate_scalar_after_key(section, constants::key::ELEMENT_ID, 2);
    });
    assert_eq!(
        decode(&invalid_source, DecodeLimits::hard(), &cancellation)
            .expect_err("source ownership mismatch must fail")
            .class(),
        DecodeErrorClass::InvalidSourceMap
    );

    let invalid_provenance = mutate_and_rehash(&encoded, SectionKind::Provenance, |section| {
        mutate_scalar_after_key(section, constants::key::ELEMENT_ID, 2);
    });
    assert_eq!(
        decode(&invalid_provenance, DecodeLimits::hard(), &cancellation)
            .expect_err("provenance ownership mismatch must fail")
            .class(),
        DecodeErrorClass::InvalidProvenance
    );

    let invalid_derivation = mutate_and_rehash(&encoded, SectionKind::Derivation, |section| {
        mutate_scalar_after_key(section, constants::key::SAFE_BOUNDED_OUTPUT, 0xf4);
    });
    assert_eq!(
        decode(&invalid_derivation, DecodeLimits::hard(), &cancellation)
            .expect_err("unsafe derivation policy must fail")
            .class(),
        DecodeErrorClass::InvalidDerivation
    );
}

/// Verifies mismatched captured vocabulary identity fails without external lookup.
#[test]
fn hostile_vocabulary_identity_mismatch_fails_closed() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("encoding crate must be inside the workspace");
    let source = fs::read(
        workspace.join("portable/specs/fixtures/positive/vocabulary/minimal-vocabulary.neu"),
    )
    .expect("vocabulary source fixture must be readable");
    let bundle = fs::read(
        workspace.join("portable/specs/fixtures/vocabulary/bundles/positive/comprehensive.json"),
    )
    .expect("vocabulary bundle fixture must be readable");
    let document = validated_with_vocabulary(&source, &bundle);
    let encoded = encode(&document, &ProducerInfo::new("test", TEST_PRODUCER_VERSION))
        .expect("vocabulary fixture must encode");
    let digest = document
        .artifacts()
        .logical_document()
        .vocabulary()
        .expect("fixture must retain vocabulary")
        .identity()
        .content_digest()
        .as_bytes();
    let mismatch = mutate_and_rehash(&encoded, SectionKind::Derivation, |section| {
        let offset = find_bytes(section, &digest);
        section[offset] ^= 1;
    });
    assert_eq!(
        decode(&mismatch, DecodeLimits::hard(), &CancellationToken::new(),)
            .expect_err("mismatched vocabulary identity must fail")
            .class(),
        DecodeErrorClass::InvalidDerivation
    );
}

/// Verifies host limits and cancellation prevent reader construction.
#[test]
fn hostile_limits_and_cancellation_fail_boundedly() {
    let document = validated(b"neu \"0.1\"\nmodule sample\n\nnum answer = 42\n");
    let encoded = encode(&document, &ProducerInfo::new("test", TEST_PRODUCER_VERSION))
        .expect("fixture must encode");
    assert_eq!(
        decode(
            encoded.as_bytes(),
            DecodeLimits::hard().with_artifact_bytes(encoded.as_bytes().len() - 1),
            &CancellationToken::new(),
        )
        .expect_err("host artifact limit must fail")
        .class(),
        DecodeErrorClass::EncodedSizeLimit
    );
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert_eq!(
        decode(encoded.as_bytes(), DecodeLimits::hard(), &cancelled)
            .expect_err("cancelled decode must fail")
            .class(),
        DecodeErrorClass::Cancelled
    );
}

/// Verifies producer changes affect only the envelope section.
#[test]
fn producer_changes_are_envelope_only() {
    let document = validated(b"neu \"0.1\"\nmodule sample\n\nnum answer = 42\n");
    let first = encode(&document, &ProducerInfo::new("one", "1")).expect("first encoding");
    let second = encode(
        &document,
        &ProducerInfo::new("two", "2").with_build("build"),
    )
    .expect("second encoding");
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

/// Documents deterministic output as a nonsemantic implementation property.
#[test]
fn repeated_encoding_is_byte_deterministic() {
    let document = validated(b"neu \"0.1\"\nmodule sample\n\nnum answer = 42\n");
    let producer = ProducerInfo::new("test", TEST_PRODUCER_VERSION);
    assert_eq!(encode(&document, &producer), encode(&document, &producer));
}

/// Verifies encoding borrows without changing validated compiler artifacts.
#[test]
fn encoding_does_not_mutate_validated_input() {
    let document = validated(b"neu \"0.1\"\nmodule sample\n\nnum answer = 42\n");
    let before = document.artifacts().as_ref().clone();
    let _encoded = encode(&document, &ProducerInfo::new("test", TEST_PRODUCER_VERSION))
        .expect("validated document must encode");
    assert_eq!(document.artifacts().as_ref(), &before);
}

/// Verifies every current positive in-memory source fixture encodes within limits.
#[test]
fn every_positive_fixture_encodes_within_limits() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("encoding crate must be inside the workspace");
    let positive = workspace.join("portable/specs/fixtures/positive");
    let mut fixtures = Vec::new();
    collect_neu_files(&positive, &mut fixtures);
    assert!(!fixtures.is_empty());
    let vocabulary_bundle = fs::read(
        workspace.join("portable/specs/fixtures/vocabulary/bundles/positive/comprehensive.json"),
    )
    .expect("vocabulary bundle fixture must be readable");
    for fixture in fixtures {
        let source = fs::read(&fixture).expect("positive fixture must be readable");
        let document = if fixture
            .components()
            .any(|part| part.as_os_str() == "vocabulary")
        {
            validated_with_vocabulary(&source, &vocabulary_bundle)
        } else {
            validated(&source)
        };
        let encoded = encode(
            &document,
            &ProducerInfo::new("fixture-test", TEST_PRODUCER_VERSION),
        )
        .expect("every positive validated fixture must encode");
        assert!(encoded.as_bytes().len() <= constants::MAXIMUM_ARTIFACT_BYTES);
        let decoded = decode(
            encoded.as_bytes(),
            DecodeLimits::hard(),
            &CancellationToken::new(),
        )
        .expect("every encoded positive fixture must decode");
        assert_eq!(document.artifacts().as_ref(), decoded.artifacts().as_ref());
    }
}

/// Verifies producer text is checked against the frozen text ceiling.
#[test]
fn oversized_producer_text_fails_before_frame_construction() {
    let document = validated(b"neu \"0.1\"\nmodule sample\n\nnum answer = 42\n");
    let oversized_name = "x".repeat(constants::MAXIMUM_TEXT_BYTES + 1);
    assert_eq!(
        encode(
            &document,
            &ProducerInfo::new(oversized_name, TEST_PRODUCER_VERSION),
        ),
        Err(super::EncodingError::EncodedSizeLimit)
    );
}

#[test]
/// Verifies every decoder ceiling builder and internal effective-limit accessor.
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
/// Verifies bounded decoder failures expose their stable code and offset policy.
fn decoder_error_contract_exposes_safe_diagnostics() {
    let error = decode(b"", DecodeLimits::hard(), &CancellationToken::new())
        .expect_err("empty frame must fail");
    assert_eq!(error.class(), DecodeErrorClass::MalformedFrame);
    assert_eq!(error.code(), DecodeErrorClass::MalformedFrame.code());
    assert_eq!(error.offset(), None);
}
