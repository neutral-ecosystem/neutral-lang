// SPDX-License-Identifier: Apache-2.0

//! Exact boundary tests for private frame and logical-schema validation.

use super::{DecodeErrorClass, DecodeLimits, SchemaBudget, constants, decode_frame, decode_type};
use crate::decoder::{CborValue, LocatedValue};
use neutral_core::{CancellationToken, StructuralLimits};
use neutral_ir::ResolvedType;

/// Total-length field offset in the frozen frame header.
const TOTAL_LENGTH_OFFSET: usize = 16;
/// Directory-offset field offset in the frozen frame header.
const DIRECTORY_OFFSET_OFFSET: usize = 24;
/// Directory-count field offset in the frozen frame header.
const DIRECTORY_COUNT_OFFSET: usize = 32;
/// Directory-entry-width field offset in the frozen frame header.
const DIRECTORY_WIDTH_OFFSET: usize = 36;
/// Header reserved-word offset in the frozen frame header.
const HEADER_RESERVED_OFFSET: usize = 38;
/// First directory-entry reserved field offset.
const ENTRY_RESERVED_OFFSET: usize = 4;
/// First directory-entry section-offset field offset.
const ENTRY_SECTION_OFFSET: usize = 8;
/// First directory-entry section-length field offset.
const ENTRY_SECTION_LENGTH: usize = 16;

/// Builds a structurally valid frame with five one-byte opaque sections.
fn valid_frame() -> Vec<u8> {
    let directory_bytes = constants::DIRECTORY_ENTRY_BYTES * constants::SECTION_COUNT;
    let first_section = constants::HEADER_BYTES + directory_bytes;
    let total = first_section + constants::SECTION_COUNT;
    let mut bytes = vec![0_u8; total];
    bytes[..constants::MAGIC.len()].copy_from_slice(&constants::MAGIC);
    bytes[8..10].copy_from_slice(&constants::FRAMING_REVISION.to_be_bytes());
    bytes[10..12].copy_from_slice(
        &u16::try_from(constants::HEADER_BYTES)
            .expect("header width should fit")
            .to_be_bytes(),
    );
    bytes[TOTAL_LENGTH_OFFSET..TOTAL_LENGTH_OFFSET + 8].copy_from_slice(
        &u64::try_from(total)
            .expect("frame length should fit")
            .to_be_bytes(),
    );
    bytes[DIRECTORY_OFFSET_OFFSET..DIRECTORY_OFFSET_OFFSET + 8].copy_from_slice(
        &u64::try_from(constants::HEADER_BYTES)
            .expect("header width should fit")
            .to_be_bytes(),
    );
    bytes[DIRECTORY_COUNT_OFFSET..DIRECTORY_COUNT_OFFSET + 4].copy_from_slice(
        &u32::try_from(constants::SECTION_COUNT)
            .expect("section count should fit")
            .to_be_bytes(),
    );
    bytes[DIRECTORY_WIDTH_OFFSET..DIRECTORY_WIDTH_OFFSET + 2].copy_from_slice(
        &u16::try_from(constants::DIRECTORY_ENTRY_BYTES)
            .expect("entry width should fit")
            .to_be_bytes(),
    );
    for index in 0..constants::SECTION_COUNT {
        let entry = constants::HEADER_BYTES + index * constants::DIRECTORY_ENTRY_BYTES;
        bytes[entry..entry + 2].copy_from_slice(
            &u16::try_from(index + 1)
                .expect("section kind should fit")
                .to_be_bytes(),
        );
        bytes[entry + 2..entry + 4]
            .copy_from_slice(&constants::SECTION_SCHEMA_REVISION.to_be_bytes());
        bytes[entry + ENTRY_SECTION_OFFSET..entry + ENTRY_SECTION_OFFSET + 8].copy_from_slice(
            &u64::try_from(first_section + index)
                .expect("section offset should fit")
                .to_be_bytes(),
        );
        bytes[entry + ENTRY_SECTION_LENGTH..entry + ENTRY_SECTION_LENGTH + 8]
            .copy_from_slice(&1_u64.to_be_bytes());
    }
    bytes
}

/// Replaces one big-endian frame field with an invalid nonzero value.
fn invalidate(bytes: &mut [u8], offset: usize, width: usize) {
    bytes[offset..offset + width].fill(0xff);
}

/// Returns the class of an expected frame failure without requiring `Frame: Debug`.
fn frame_error(bytes: &[u8], limits: DecodeLimits) -> DecodeErrorClass {
    match decode_frame(bytes, limits) {
        Ok(_) => panic!("frame was expected to fail"),
        Err(error) => error.class(),
    }
}

/// Builds one located closed CBOR map for direct schema-decoder tests.
fn map(members: Vec<(&str, CborValue)>) -> LocatedValue {
    LocatedValue {
        offset: 0,
        value: CborValue::Map(
            members
                .into_iter()
                .map(|(key, value)| (key.to_owned(), LocatedValue { offset: 0, value }))
                .collect(),
        ),
    }
}

/// Creates a mutable schema budget with caller-selected nesting capacity.
fn type_budget(cancellation: &CancellationToken, nesting_depth: u64) -> SchemaBudget<'_> {
    SchemaBudget {
        limits: StructuralLimits::new(64, 4)
            .expect("base limits should be valid")
            .with_nesting_depth(nesting_depth)
            .expect("nesting depth should be valid"),
        nodes: 0,
        cancellation,
    }
}

#[test]
/// Verifies closed recursive type decoding rejects unknown kinds and enforces depth.
fn recursive_type_schema_is_closed_and_depth_bounded() {
    let cancellation = CancellationToken::new();
    let unknown = map(vec![
        (constants::key::KIND, CborValue::Text("unknown".to_owned())),
        (
            constants::key::INNER,
            CborValue::Map(vec![(
                constants::key::KIND.to_owned(),
                LocatedValue {
                    offset: 0,
                    value: CborValue::Text(constants::kind::NUM.to_owned()),
                },
            )]),
        ),
    ]);
    assert_eq!(
        decode_type(&unknown, &mut type_budget(&cancellation, 4), 1)
            .expect_err("unknown recursive type kind must fail")
            .class(),
        DecodeErrorClass::InvalidEncodedSchema
    );

    let list = map(vec![
        (
            constants::key::KIND,
            CborValue::Text(constants::kind::LIST.to_owned()),
        ),
        (
            constants::key::INNER,
            CborValue::Map(vec![(
                constants::key::KIND.to_owned(),
                LocatedValue {
                    offset: 0,
                    value: CborValue::Text(constants::kind::NUM.to_owned()),
                },
            )]),
        ),
    ]);
    assert_eq!(
        decode_type(&list, &mut type_budget(&cancellation, 2), 1)
            .expect("nested type at the exact depth must decode"),
        ResolvedType::list(ResolvedType::Num)
    );
    assert_eq!(
        decode_type(&list, &mut type_budget(&cancellation, 1), 1)
            .expect_err("nested type one over the depth must fail")
            .class(),
        DecodeErrorClass::EncodedSizeLimit
    );
}

#[test]
/// Verifies logical depth, traversal, and cancellation at and over boundaries.
fn schema_budget_enforces_exact_boundaries() {
    let cancellation = CancellationToken::new();
    let limits = StructuralLimits::new(64, 4)
        .expect("base limits should be valid")
        .with_nesting_depth(2)
        .expect("depth should be valid")
        .with_traversal_nodes(2)
        .expect("traversal should be valid");
    let mut budget = SchemaBudget {
        limits,
        nodes: 0,
        cancellation: &cancellation,
    };
    assert!(budget.visit(2).is_ok());
    assert!(budget.visit(2).is_ok());
    assert_eq!(
        budget
            .visit(2)
            .expect_err("one over traversal must fail")
            .class(),
        DecodeErrorClass::EncodedSizeLimit
    );
    let mut depth_budget = SchemaBudget {
        limits,
        nodes: 0,
        cancellation: &cancellation,
    };
    assert_eq!(
        depth_budget
            .visit(3)
            .expect_err("one over depth must fail")
            .class(),
        DecodeErrorClass::EncodedSizeLimit
    );
    cancellation.cancel();
    assert_eq!(
        depth_budget
            .visit(0)
            .expect_err("cancellation must fail")
            .class(),
        DecodeErrorClass::Cancelled
    );
}

#[test]
/// Verifies every independent fixed-header invariant fails closed.
fn frame_header_fields_are_independently_validated() {
    let valid = valid_frame();
    assert!(decode_frame(&valid, DecodeLimits::hard()).is_ok());
    assert!(
        decode_frame(
            &valid,
            DecodeLimits::hard().with_artifact_bytes(valid.len())
        )
        .is_ok()
    );
    assert_eq!(
        frame_error(
            &valid,
            DecodeLimits::hard().with_artifact_bytes(valid.len() - 1)
        ),
        DecodeErrorClass::EncodedSizeLimit
    );
    assert_eq!(
        frame_error(&valid[..constants::HEADER_BYTES - 1], DecodeLimits::hard()),
        DecodeErrorClass::MalformedFrame
    );
    // Buffer of exactly HEADER_BYTES must pass the slice-length check (< HEADER_BYTES)
    // and proceed to header validation, reporting invalid magic at offset 0.
    let exact_header = [0_u8; constants::HEADER_BYTES];
    match decode_frame(&exact_header, DecodeLimits::hard()) {
        Ok(_) => panic!("header with zero magic must fail"),
        Err(err) => {
            assert_eq!(err.class(), DecodeErrorClass::MalformedFrame);
            assert_eq!(
                err.offset(),
                Some(0),
                "exact HEADER_BYTES buffer must reach magic validation at offset 0"
            );
        }
    }
    for (offset, width) in [
        (10, 2),
        (12, 4),
        (DIRECTORY_OFFSET_OFFSET, 8),
        (DIRECTORY_COUNT_OFFSET, 4),
        (DIRECTORY_WIDTH_OFFSET, 2),
        (HEADER_RESERVED_OFFSET, 2),
    ] {
        let mut malformed = valid.clone();
        invalidate(&mut malformed, offset, width);
        assert_eq!(
            frame_error(&malformed, DecodeLimits::hard()),
            DecodeErrorClass::MalformedFrame
        );
    }
    // A frame truncated to exactly first_section bytes (with valid directory section lengths >= 1)
    // must pass line 226 (< first_section) and fail in section end-range check at offset 56.
    let directory_bytes = constants::DIRECTORY_ENTRY_BYTES * constants::SECTION_COUNT;
    let first_section = constants::HEADER_BYTES + directory_bytes;
    let mut truncated_at_first_section = valid.clone();
    truncated_at_first_section.truncate(first_section);
    let total_len = u64::try_from(first_section).unwrap().to_be_bytes();
    truncated_at_first_section[TOTAL_LENGTH_OFFSET..TOTAL_LENGTH_OFFSET + 8]
        .copy_from_slice(&total_len);
    match decode_frame(&truncated_at_first_section, DecodeLimits::hard()) {
        Ok(_) => panic!("truncated frame must fail"),
        Err(err) => {
            assert_eq!(err.class(), DecodeErrorClass::MalformedFrame);
            assert_eq!(
                err.offset(),
                Some(64),
                "truncated frame at first_section must reach section validation at offset 64"
            );
        }
    }
}

#[test]
/// Verifies directory offsets, lengths, kinds, and reserved fields exactly.
fn frame_directory_fields_are_independently_validated() {
    let valid = valid_frame();
    assert!(decode_frame(&valid, DecodeLimits::hard().with_section_bytes(1)).is_ok());
    assert_eq!(
        frame_error(&valid, DecodeLimits::hard().with_section_bytes(0)),
        DecodeErrorClass::EncodedSizeLimit
    );
    for (relative_offset, width) in [
        (0, 2),
        (ENTRY_RESERVED_OFFSET, 4),
        (ENTRY_SECTION_OFFSET, 8),
        (ENTRY_SECTION_LENGTH, 8),
    ] {
        let mut malformed = valid.clone();
        let offset = constants::HEADER_BYTES + relative_offset;
        invalidate(&mut malformed, offset, width);
        assert!(decode_frame(&malformed, DecodeLimits::hard()).is_err());
    }
    let mut trailing = valid.clone();
    trailing.push(0);
    let length = u64::try_from(trailing.len()).expect("length should fit");
    trailing[TOTAL_LENGTH_OFFSET..TOTAL_LENGTH_OFFSET + 8].copy_from_slice(&length.to_be_bytes());
    assert_eq!(
        frame_error(&trailing, DecodeLimits::hard()),
        DecodeErrorClass::MalformedFrame
    );
}

#[test]
/// Verifies malformed directory fields report their exact offsets and size precedence.
fn directory_diagnostics_identify_the_offending_entry_field() {
    for index in 0..constants::SECTION_COUNT {
        let entry = constants::HEADER_BYTES + index * constants::DIRECTORY_ENTRY_BYTES;
        let mut bytes = valid_frame();
        invalidate(&mut bytes, entry + 2, 2);
        let failure = decode_frame(&bytes, DecodeLimits::hard())
            .err()
            .expect("bad revision");
        assert_eq!(failure.class(), DecodeErrorClass::UnsupportedVersion);
        assert_eq!(failure.offset(), Some(u64::try_from(entry + 2).unwrap()));

        for (length, limit, expected) in [
            (0_u64, 1, DecodeErrorClass::MalformedFrame),
            (1, 1, DecodeErrorClass::MalformedFrame),
            (2, 1, DecodeErrorClass::EncodedSizeLimit),
        ] {
            let mut bytes = valid_frame();
            bytes[entry + ENTRY_SECTION_OFFSET..entry + ENTRY_SECTION_OFFSET + 8]
                .copy_from_slice(&0_u64.to_be_bytes());
            bytes[entry + ENTRY_SECTION_LENGTH..entry + ENTRY_SECTION_LENGTH + 8]
                .copy_from_slice(&length.to_be_bytes());
            let failure = decode_frame(&bytes, DecodeLimits::hard().with_section_bytes(limit))
                .err()
                .expect("invalid offset must fail even at the exact length ceiling");
            assert_eq!(failure.class(), expected);
            assert_eq!(
                failure.offset(),
                Some(u64::try_from(entry + ENTRY_SECTION_OFFSET).unwrap())
            );
        }
    }
}

/// Builds a scalar null value for recursive schema boundary tests.
fn null_value() -> LocatedValue {
    map(vec![(
        constants::key::KIND,
        CborValue::Text(constants::kind::NULL.into()),
    )])
}

/// Builds a valid module-owned identity for a record or reference symbol.
fn owned_identity(name: &str) -> CborValue {
    map(vec![
        (
            constants::key::MODULE,
            map(vec![
                (
                    constants::key::LANGUAGE_BEHAVIOR_VERSION,
                    CborValue::Text(neutral_ir::LANGUAGE_BEHAVIOR_VERSION.into()),
                ),
                (constants::key::NAME, CborValue::Text("schema_test".into())),
            ])
            .value,
        ),
        (constants::key::NAME, CborValue::Text(name.into())),
    ])
    .value
}

/// Builds one reference-shaped value with a caller-selected discriminator.
fn reference_value(kind: &str) -> LocatedValue {
    map(vec![
        (constants::key::KIND, CborValue::Text(kind.into())),
        (constants::key::TARGET_ELEMENT_ID, CborValue::Unsigned(1)),
        (constants::key::TARGET_SYMBOL, owned_identity("target")),
        (
            constants::key::TARGET_TYPE,
            map(vec![(
                constants::key::KIND,
                CborValue::Text(constants::kind::NUM.into()),
            )])
            .value,
        ),
    ])
}

/// Builds a record with a single null field for exact recursive depth tests.
fn record_value(kind: &str) -> LocatedValue {
    let identity = if kind == constants::kind::RECORD {
        owned_identity("Entry")
    } else {
        map(vec![
            (
                constants::key::VOCABULARY,
                CborValue::Text("Fixture".into()),
            ),
            (constants::key::NAME, CborValue::Text("Entry".into())),
        ])
        .value
    };
    map(vec![
        (constants::key::KIND, CborValue::Text(kind.into())),
        (constants::key::IDENTITY, identity),
        (
            constants::key::FIELDS,
            CborValue::Array(vec![map(vec![
                (constants::key::NAME, CborValue::Text("value".into())),
                (constants::key::VALUE, null_value().value),
            ])]),
        ),
    ])
}

#[test]
/// Verifies every recursive value edge consumes depth and preserves closed discriminators.
fn recursive_values_account_for_each_child_depth() {
    let cancellation = CancellationToken::new();
    let list = map(vec![
        (
            constants::key::KIND,
            CborValue::Text(constants::kind::LIST.into()),
        ),
        (constants::key::ITEMS, CborValue::Array(vec![null_value()])),
    ]);
    for (value, required_depth) in [
        (list, 2),
        (reference_value(constants::kind::REF), 2),
        (record_value(constants::kind::RECORD), 3),
        (record_value(constants::kind::VOCABULARY_RECORD), 3),
    ] {
        assert!(
            super::decode_value(&value, &mut type_budget(&cancellation, required_depth), 1).is_ok()
        );
        assert_eq!(
            super::decode_value(
                &value,
                &mut type_budget(&cancellation, required_depth - 1),
                1
            )
            .expect_err("a child beyond the depth ceiling must fail")
            .class(),
            DecodeErrorClass::EncodedSizeLimit
        );
    }
    assert_eq!(
        super::decode_value(
            &reference_value("unknown"),
            &mut type_budget(&cancellation, 4),
            1
        )
        .expect_err("reference-shaped unknown kind must fail")
        .class(),
        DecodeErrorClass::InvalidEncodedSchema
    );
}
