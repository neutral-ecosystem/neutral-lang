// SPDX-License-Identifier: Apache-2.0

//! Missing hostile-schema cases in `src/decode.rs`, after coalescing LLVM instances.

/// Covers `decode_span`'s reversed-span rejection at source line 944.
#[test]
#[ignore = "UNIMPLEMENTED: reversed source spans have no dedicated decoder regression"]
fn reversed_source_span_reports_invalid_source_map() {
    // TODO: Give decode_span a closed {start: 4, end: 3} map at offset 37 with source_length 8; assert InvalidSourceMap and Some(37), plus accept {start: 3, end: 3}.
    todo!("UNIMPLEMENTED");
}

/// Covers the unknown spelling fallback in `decode_origin` at source line 1109.
#[test]
#[ignore = "UNIMPLEMENTED: unknown value-origin spellings have no dedicated decoder regression"]
fn unknown_value_origin_reports_schema_error_at_its_offset() {
    // TODO: Pass a LocatedValue::Text("unknown-origin") at offset 37 to decode_origin; assert InvalidEncodedSchema and Some(37), and verify the six frozen origin spellings still decode.
    todo!("UNIMPLEMENTED");
}

/// Covers the unknown spelling fallback in `decode_normalization` at source line 1127.
#[test]
#[ignore = "UNIMPLEMENTED: unknown normalization spellings have no dedicated decoder regression"]
fn unknown_normalization_reports_schema_error_at_its_offset() {
    // TODO: Pass a LocatedValue::Text("unknown-normalization") at offset 37 to decode_normalization; assert InvalidEncodedSchema and Some(37), and verify the eight frozen normalization spellings still decode.
    todo!("UNIMPLEMENTED");
}

/// Covers diagnostic narrowing and rejected limit builders in `decode_acceptance` at lines 1221 and 1231.
#[test]
#[ignore = "UNIMPLEMENTED: captured acceptance overflow and invalid limit values lack decoder regressions"]
fn invalid_captured_acceptance_limits_report_invalid_derivation() {
    // TODO: Build one valid exact ten-key acceptance map at offset 37, then independently set diagnostics to u32::MAX + 1 and every builder-rejected scalar to an invalid value; assert InvalidDerivation and Some(37) while the unchanged map succeeds.
    todo!("UNIMPLEMENTED");
}

/// Covers feature syntax and strict ordering in `decode_vocabulary_identity` at lines 645 and 1418.
#[test]
#[ignore = "UNIMPLEMENTED: malformed captured vocabulary feature IDs and ordering lack decoder regressions"]
fn captured_vocabulary_features_require_valid_unique_sorted_ids() {
    // TODO: Supply an otherwise valid exact vocabulary identity map at offset 37 with required_features containing an invalid ID, a duplicate pair, or a descending pair; assert InvalidEncodedSchema and Some(37), and accept the sorted valid control.
    todo!("UNIMPLEMENTED");
}
