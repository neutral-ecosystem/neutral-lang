// SPDX-License-Identifier: Apache-2.0

//! Missing nonempty expression-project fields in `src/expression/project.rs`.

/// Covers the module-import writer closure at source line 43.
#[test]
#[ignore = "UNIMPLEMENTED: expression wire fixtures contain no nonempty module import list"]
fn expression_module_imports_preserve_order_and_roundtrip() {
    // TODO: Construct raw ExpressionProjectIr modules with imports ["alpha", "beta"], write_ir and canonically parse/read_ir; inspect the exact two-text import tuple and assert module identities and import order are unchanged.
    todo!("UNIMPLEMENTED");
}

/// Covers nonempty vocabulary counting, catalogue, feature, visibility and dependency sites at lines 46, 62, 67, 74, 80, 96 and 100.
#[test]
#[ignore = "UNIMPLEMENTED: expression wire fixtures contain no vocabulary bundles or dependencies"]
fn expression_vocabulary_catalogues_preserve_features_visibility_and_dependencies() {
    // TODO: Build raw expression IR with one vocabulary bundle containing ordered required features, one public and one private definition, and a dependency; write/read the 22-position tuple, inspect positions 3/4/12, and assert bodies, feature order, public-name filtering and dependency identity/version survive.
    todo!("UNIMPLEMENTED");
}

/// Covers optional default-tree writer and reader closures at source lines 140 and 263.
#[test]
#[ignore = "UNIMPLEMENTED: existing template fixture exercises only absent parameter defaults"]
fn public_template_parameter_defaults_preserve_expression_trees() {
    // TODO: Write/read a raw expression project containing a public template with a static parameter default Some(literal Bool tree) and a deferred parameter without a default; assert names, phases, types, declaration order and both default states survive exactly.
    todo!("UNIMPLEMENTED");
}
