// SPDX-License-Identifier: Apache-2.0

//! Missing source contracts in frontend/expansion/mod.rs; existing allocation sweeps are not repeated.

/// Qualified applications resolve the captured alias and retain definition-scoped nominal identities.
#[test]
#[ignore = "UNIMPLEMENTED: imported public templates resolve aliases at definition scope"]
fn imported_public_template_keeps_definition_scope() {
    // TODO: Capture caller/definition modules with different same-spelled record types and a public
    // template returning the definition's type; apply it through the caller's import alias (also
    // through a matrix). Assert exact returned type owners, values and definition/application spans.
    todo!("UNIMPLEMENTED");
}

/// Missing aliases and private imported definitions cannot publish expanded declarations.
#[test]
#[ignore = "UNIMPLEMENTED: qualified template lookup rejects unknown aliases and private definitions"]
fn imported_template_lookup_rejects_unknown_alias_and_private_definition() {
    // TODO: Compare instantiate missing::Item() with a captured import of a private Item template.
    // Assert NEU-TPL-003 for both and no successful IR; include a public Item positive control.
    // These exercise template_owner's uncovered alias lookup/error sites at lines 31 and 33.
    todo!("UNIMPLEMENTED");
}

/// Static classification traverses composite arguments while templates are still symbolic.
#[test]
#[ignore = "UNIMPLEMENTED: nested static template arguments classify lists records variants and interpolation"]
fn nested_static_composite_arguments_keep_static_phase() {
    // TODO: Type public outer templates that pass static parameters to inner static parameters
    // inside List<num>, a record, a variant, and $"${text(value)}". Include a match over a static
    // variant whose arms return constants. Instantiate each and assert exact folded values and
    // retained public template contracts; exercise phase_static's sites at lines 640, 643 and 647.
    todo!("UNIMPLEMENTED");
}

/// Composite containers cannot conceal deferred leaves from nested static arguments.
#[test]
#[ignore = "UNIMPLEMENTED: nested static composite arguments reject deferred members and match payload binders"]
fn nested_static_composite_arguments_reject_deferred_leaves() {
    // TODO: Replace one static leaf in each accepted list/record/variant/interpolation argument
    // with a deferred parameter. Also pass a match arm's payload binder to a nested static
    // parameter. Assert NEU-TPL-004 even in unused outer definitions or statically unselected arms.
    todo!("UNIMPLEMENTED");
}

/// Exclusions independently require static values even when their types and axis names match.
#[test]
#[ignore = "UNIMPLEMENTED: matrix exclusion tuples reject deferred values"]
fn matrix_exclusions_reject_deferred_tuple_members() {
    // TODO: Compile input num denied; template Item(static num value) -> num { value }; and
    // expand items = Item(value: axis) for (axis in [1]) exclude [{ axis: denied }].
    // Assert NEU-TPL-007 at exclusions' deferred-value site (line 580) and no IR; repeat with
    // an empty typed axis or false filter, and contrast a static exclusion that removes a tuple.
    // Existing axis/filter deferred-input cases do not exercise exclusion member evaluation.
    todo!("UNIMPLEMENTED");
}
