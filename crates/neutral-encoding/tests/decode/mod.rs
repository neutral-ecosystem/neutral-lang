// SPDX-License-Identifier: Apache-2.0

//! Exact boundary tests for private frame and logical-schema validation.

use super::{DecodeErrorClass, DecodeLimits, SchemaBudget, constants, decode_frame};
use neutral_core::{CancellationToken, StructuralLimits};

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
