// SPDX-License-Identifier: Apache-2.0

//! Encoded artifact cross-package regressions.

use super::*;

#[test]
/// Verifies the external encoder emits the fixed frame and all five sections.
fn integration_encoder_emits_complete_fixed_frame() {
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
        assert_ne!(encoded.section_bytes(kind).len(), 0);
    }
}

#[test]
/// Verifies an external artifact reconstructs exact immutable reader views.
fn system_encoded_artifact_reconstructs_reader_views() {
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
fn property_producer_changes_are_envelope_only() {
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
fn property_encoding_is_deterministic_and_nonmutating() {
    let document = compile_reader(COMBINED_REUSE_REFERENCE);
    let before = document.artifacts().as_ref().clone();
    let producer = ProducerInfo::new("test", TEST_PRODUCER_VERSION);
    assert_eq!(encode(&document, &producer), encode(&document, &producer));
    assert_eq!(document.artifacts().as_ref(), &before);
}

#[test]
/// Verifies every positive source fixture reaches bounded external encoding.
fn conformance_every_positive_fixture_encodes() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("test-suite crate should be inside the workspace");
    let mut fixtures = Vec::new();
    collect_positive_sources(
        &workspace.join("conformance/releases/v0.1.0/specs/fixtures/positive"),
        &mut fixtures,
    );
    assert_ne!(fixtures.len(), 0);
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
