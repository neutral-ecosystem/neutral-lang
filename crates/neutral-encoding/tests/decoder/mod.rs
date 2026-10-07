// SPDX-License-Identifier: Apache-2.0

//! Restricted CBOR lexical boundaries, independent of the artifact schema.

use super::*;

/// Returns a parser failure without requiring the untrusted tree to implement Debug.
fn failure(bytes: &[u8], base: usize, limits: DecodeLimits) -> DecodeError {
    match parse_section(bytes, base, limits, &CancellationToken::new()) {
        Ok(_) => panic!("hostile section must fail"),
        Err(error) => error,
    }
}

/// Every fixed-width argument is decoded exactly and every truncated prefix fails.
#[test]
fn integer_widths_and_truncations_are_checked() {
    for (bytes, expected) in [
        (vec![0x18, 24], 24),
        (vec![0x19, 1, 0], 256),
        (vec![0x1a, 0, 1, 0, 0], 65_536),
        (vec![0x1b, 0, 0, 0, 1, 0, 0, 0, 0], 4_294_967_296),
    ] {
        let parsed = parse_section(&bytes, 13, DecodeLimits::hard(), &CancellationToken::new())
            .expect("complete integer");
        assert_eq!(parsed.offset, 13);
        assert!(matches!(parsed.value, CborValue::Unsigned(value) if value == expected));
        for end in 0..bytes.len() {
            assert_eq!(
                failure(&bytes[..end], 13, DecodeLimits::hard()).class(),
                DecodeErrorClass::MalformedCbor
            );
        }
    }
    let parsed =
        parse_section(&[0x20], 0, DecodeLimits::hard(), &CancellationToken::new()).unwrap();
    assert!(matches!(parsed.value, CborValue::Negative(-1)));
    assert_eq!(
        failure(
            &[0x3b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
            9,
            DecodeLimits::hard()
        )
        .class(),
        DecodeErrorClass::InvalidEncodedSchema
    );
}

/// Definite lengths and UTF-8 are validated before accepting string data.
#[test]
fn string_and_container_limits_are_exact() {
    let token = CancellationToken::new();
    let limits = DecodeLimits::hard().with_byte_string_bytes(2);
    let value = parse_section(&[0x42, 1, 2], 0, limits, &token).unwrap();
    assert!(matches!(value.value, CborValue::Bytes(bytes) if bytes == [1, 2]));
    assert_eq!(
        failure(&[0x43, 1, 2, 3], 0, limits).class(),
        DecodeErrorClass::EncodedSizeLimit
    );
    assert_eq!(
        failure(&[0x43, 1], 0, DecodeLimits::hard()).class(),
        DecodeErrorClass::MalformedCbor
    );
    assert_eq!(
        failure(&[0x61, 0xff], 0, DecodeLimits::hard()).class(),
        DecodeErrorClass::MalformedCbor
    );
    let array = [0x82, 0xf4, 0xf6];
    assert!(
        parse_section(
            &array,
            0,
            DecodeLimits::hard()
                .with_container_items(2)
                .with_traversal_nodes(3)
                .with_nesting_depth(2),
            &token
        )
        .is_ok()
    );
    for limits in [
        DecodeLimits::hard().with_container_items(1),
        DecodeLimits::hard().with_traversal_nodes(2),
        DecodeLimits::hard().with_nesting_depth(1),
    ] {
        assert_eq!(
            failure(&array, 0, limits).class(),
            DecodeErrorClass::EncodedSizeLimit
        );
    }
    assert_eq!(
        failure(&[0x82, 0xf4], 0, DecodeLimits::hard()).class(),
        DecodeErrorClass::MalformedCbor
    );
}

/// Duplicate map entries remain visible for schema validation rather than being collapsed.
#[test]
fn maps_preserve_duplicates_and_reject_nontext_keys() {
    let bytes = [0xa2, 0x61, b'x', 0xf4, 0x61, b'x', 0xf5];
    let parsed = parse_section(&bytes, 7, DecodeLimits::hard(), &CancellationToken::new()).unwrap();
    let CborValue::Map(members) = parsed.value else {
        panic!("map");
    };
    assert_eq!(members.len(), 2);
    assert_eq!(members[0].0, members[1].0);
    assert!(matches!(members[0].1.value, CborValue::Boolean(false)));
    assert!(matches!(members[1].1.value, CborValue::Boolean(true)));
    assert_eq!(
        failure(&[0xa1, 0, 0], 7, DecodeLimits::hard()).class(),
        DecodeErrorClass::MalformedCbor
    );
}

/// Unsupported forms, trailing bytes, cancellation, and offset overflow fail boundedly.
#[test]
fn unsupported_forms_and_offset_overflow_fail_closed() {
    for bytes in [
        &[0x1c][..],
        &[0x1f],
        &[0x9f],
        &[0xbf],
        &[0xc0, 0],
        &[0xf7],
        &[0xf6, 0],
    ] {
        assert_eq!(
            failure(bytes, 0, DecodeLimits::hard()).class(),
            DecodeErrorClass::MalformedCbor
        );
    }
    assert_eq!(
        failure(&[0x81, 0], usize::MAX, DecodeLimits::hard()).class(),
        DecodeErrorClass::EncodedSizeLimit
    );
    let token = CancellationToken::new();
    token.cancel();
    assert!(
        matches!(parse_section(&[0xf6], 0, DecodeLimits::hard(), &token), Err(error) if error.class() == DecodeErrorClass::Cancelled)
    );
    let mut parser = Parser {
        bytes: &[0],
        position: usize::MAX,
        base_offset: 0,
        limits: DecodeLimits::hard(),
        nodes: 0,
        cancellation: &CancellationToken::new(),
        canonical: false,
    };
    assert_eq!(
        parser.read_array::<2>().unwrap_err().class(),
        DecodeErrorClass::MalformedCbor
    );
    parser.position = 0;
    parser.nodes = usize::MAX;
    assert!(
        matches!(parser.value(1), Err(error) if error.class() == DecodeErrorClass::EncodedSizeLimit)
    );
}

/// Explicit successor parsing rejects every nonminimal width without changing legacy acceptance.
#[test]
fn successor_minimal_widths_are_separate_from_legacy_selection() {
    let token = CancellationToken::new();
    for bytes in [
        &[0x18, 0][..],
        &[0x19, 0, 24][..],
        &[0x1a, 0, 0, 1, 0][..],
        &[0x1b, 0, 0, 0, 0, 0, 1, 0, 0][..],
        &[0x38, 0][..],
        &[0x98, 0][..],
        &[0x78, 1, b'x'][..],
        &[0x58, 1, 0][..],
    ] {
        assert!(parse_section(bytes, 0, DecodeLimits::hard(), &token).is_ok());
        assert!(
            matches!(parse_canonical_section(bytes, 0, DecodeLimits::hard(), &token), Err(e) if e.class() == DecodeErrorClass::MalformedCbor)
        );
    }
    for bytes in [
        &[0x00][..],
        &[0x18, 24][..],
        &[0x19, 1, 0][..],
        &[0x1a, 0, 1, 0, 0][..],
        &[0x1b, 0, 0, 0, 1, 0, 0, 0, 0][..],
        &[0x80][..],
        &[0x61, b'x'][..],
    ] {
        assert!(parse_canonical_section(bytes, 0, DecodeLimits::hard(), &token).is_ok());
    }
}
