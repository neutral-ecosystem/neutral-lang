// SPDX-License-Identifier: Apache-2.0

//! Unimplemented required-port ordering cases from LLVM's zero-count key comparator paths.
//!
//! Existing evaluation tests sort/deduplicate missing input keys, but do not compare
//! two missing output keys. These cases must use real validated models and literal
//! expected port order; ignored placeholders are not coverage or qualification evidence.

/// Covers the mixed input/output branches of `evaluation::keys::compare`.
#[test]
#[ignore = "UNIMPLEMENTED: canonical ordering of mixed required input and output ports"]
fn missing_input_and_output_ports_have_canonical_kind_and_symbol_order() {
    // TODO: Demand both kinds in permuted record fields; compare literal required keys and repeated-output deduplication.
    todo!("UNIMPLEMENTED");
}

/// Covers `compare`'s output tie-breakers and `ty`/`tag` for accepted nominal result contracts.
#[test]
#[ignore = "UNIMPLEMENTED: exact nominal result-contract ordering for required output ports"]
fn missing_outputs_from_one_target_order_by_exact_public_result_contract() {
    // TODO: Register source and vocabulary result contracts for one target, then assert alias-independent literal key order.
    todo!("UNIMPLEMENTED");
}

/// Covers `selectors`, `segment_tag` and their payload comparisons with valid typed paths.
#[test]
#[ignore = "UNIMPLEMENTED: typed selector tie-breakers for required output ports"]
fn missing_outputs_with_one_contract_order_by_selector_payload_and_prefix() {
    // TODO: Demand distinct field, numeric index, variant payload and prefix paths; assert literal order and equal-key deduplication.
    todo!("UNIMPLEMENTED");
}
