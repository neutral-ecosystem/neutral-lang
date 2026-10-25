// SPDX-License-Identifier: Apache-2.0
//! Current expression-project wire cross-package regressions.
use super::hostile_decoder::artifact;
use neutral_core::CancellationToken;
use neutral_encoding::{
    DecodeLimits,
    expression::{decode_expression_project, encode_expression_project},
};

#[test]
fn integration_current_source_roundtrips_through_independent_reader() {
    let (bytes, limits) = artifact();
    assert!(bytes.starts_with(b"NEUP3\r\n\x1a"));
    let decoded = decode_expression_project(
        &bytes,
        DecodeLimits::hard(),
        limits,
        &CancellationToken::new(),
    )
    .unwrap();
    let encoded_again = encode_expression_project(&decoded, &CancellationToken::new()).unwrap();
    assert_eq!(encoded_again, bytes);
}

#[test]
fn property_current_expression_encoding_is_deterministic() {
    let (bytes, limits) = artifact();
    let reader = decode_expression_project(
        &bytes,
        DecodeLimits::hard(),
        limits,
        &CancellationToken::new(),
    )
    .unwrap();
    let first = encode_expression_project(&reader, &CancellationToken::new()).unwrap();
    let second = encode_expression_project(&reader, &CancellationToken::new()).unwrap();
    assert_eq!(first, second);
}

#[test]
fn security_current_wire_does_not_accept_old_project_magic() {
    let (_, limits) = artifact();
    for old in [
        b"NIR-PROJECT-CBOR/1".as_slice(),
        b"NIR-PROJECT-CBOR/2".as_slice(),
    ] {
        assert!(
            decode_expression_project(old, DecodeLimits::hard(), limits, &CancellationToken::new())
                .is_err()
        );
    }
}
