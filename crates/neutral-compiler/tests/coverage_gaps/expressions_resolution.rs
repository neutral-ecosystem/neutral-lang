// SPDX-License-Identifier: Apache-2.0

//! Context-free constructors and exact variant field contracts in expression resolution.

/// An inferred empty list cannot fabricate an element type from a later operator operand.
#[test]
#[ignore = "UNIMPLEMENTED: empty list without contextual element type rejects before publication"]
fn inferred_empty_list_requires_element_type() {
    // TODO: Compile "bool item = [] == [1]" and an untyped empty matrix axis; assert
    // NEU-EXP-004 at list's line 583. Compare accepted "List<num> item = [] ++ [1]" and a matrix
    // axis referencing a declared List<num> empty binding to distinguish contextual typing.
    todo!("UNIMPLEMENTED");
}

/// A nominal constructor must have a contextual nominal type before field resolution.
#[test]
#[ignore = "UNIMPLEMENTED: context-free record constructor rejects without inferring a nominal owner"]
fn context_free_record_constructor_requires_nominal_type() {
    // TODO: Compile "bool item = { count: 1 } == { count: 1 }" with a record declaration present;
    // assert NEU-EXP-004 at record's missing-expected-type site (line 612), with no IR.
    // Contrast a typed record binding; merely matching field names must not select a nominal type.
    todo!("UNIMPLEMENTED");
}

/// Exactly two supplied variant fields must still be the required tag/payload pair.
#[test]
#[ignore = "UNIMPLEMENTED: variant constructors reject two-field shapes missing tag or payload"]
fn two_field_variant_constructor_requires_tag_and_payload() {
    // TODO: For variant Choice { string ready, num failed }, compile Choice bindings containing
    // { other: "ready", payload: "ok" } and { tag: "ready", other: "ok" }.
    // Assert NEU-EXP-004 at lines 647 and 656 and no IR. Existing tests only cover one-field
    // constructors, invalid tag values, and computed tags; include the valid two-field control.
    todo!("UNIMPLEMENTED");
}
