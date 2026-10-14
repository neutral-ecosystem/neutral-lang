// SPDX-License-Identifier: Apache-2.0

//! Remaining meaningful malformed-source cases in frontend/expressions/parser.rs.

/// Truncated expansion declarations return their source syntax class without indexing absent names.
#[test]
#[ignore = "UNIMPLEMENTED: template and matrix declarations require a name and template body"]
fn truncated_template_and_matrix_declarations_reject_safely() {
    // TODO: Exercise declarations "template", "expand", and "template Item() -> num" through
    // captured successor compilation (move to unit-test scope for the private parser if capture
    // rejects first).
    // Assert NEU-TPL-002 with no IR for missing names/body, covering lines 588, 654 and 697.
    todo!("UNIMPLEMENTED");
}

/// Primary operations requiring a name reject an empty operand independently of missing qualifiers.
#[test]
#[ignore = "UNIMPLEMENTED: expression name parser rejects empty ref and instantiate operands"]
fn named_primary_operations_reject_empty_name() {
    // TODO: Compile "Ref<num> item = ref()" and "num item = instantiate ()". Assert
    // NEU-EXP-002, no IR, and a safe diagnostic envelope; cover name's first-token error at 328.
    // Existing malformed-source tests cover "unknown::", which is a different lookup site.
    todo!("UNIMPLEMENTED");
}

/// A binding declaration requires the assignment delimiter before expression parsing begins.
#[test]
#[ignore = "UNIMPLEMENTED: binding declaration without assignment delimiter rejects safely"]
fn binding_declaration_requires_assignment_delimiter() {
    // TODO: Compile "num item" and "public num item"; assert NEU-EXP-002, a safe source
    // location and no IR at declaration's missing-'=' site (line 545). Compare accepted
    // "input num item"; existing "num item =" tests cover an empty RHS after a present delimiter.
    todo!("UNIMPLEMENTED");
}
