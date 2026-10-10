// SPDX-License-Identifier: Apache-2.0

//! hostile decoder cross-package regressions.

use super::*;

#[test]
/// Verifies representative hostile framing, integrity, capability, and limit classes.
fn security_hostile_encoded_inputs_fail_boundedly() {
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
fn fuzz_smoke_single_byte_mutations_terminate() {
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
            assert_ne!(decoded.module_name(), "");
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
            assert_ne!(decoded.module_name(), "");
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
            assert_ne!(decoded.module_name(), "");
        }
    }
}

#[test]
/// Verifies an oversized envelope string fails before a complete frame exists.
fn security_oversized_producer_text_fails_boundedly() {
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
