// SPDX-License-Identifier: Apache-2.0

//! references cross-package regressions.

use super::*;

#[test]
/// Verifies forward, nested, recursive, and combined typed reference fixtures.
fn conformance_typed_references_positive_oracles() {
    let forward = compile_artifacts(FORWARD_REFERENCE);
    let selected = forward
        .logical_document()
        .declarations()
        .iter()
        .find(|declaration| declaration.name() == "selected")
        .expect("forward reference fixture must contain selected");
    assert_eq!(selected.resolved_type().to_string(), "Ref<Config>");
    assert_eq!(selected.value().to_string(), "ref(#1)");
    assert_eq!(forward.reference_provenance().len(), 1);
    assert_eq!(
        forward.provenance()[1].origin(),
        ValueOrigin::IdentityReference
    );

    let recursive = compile_reader(RECURSIVE_REFERENCE);
    assert_eq!(recursive.declarations().len(), 2);
    assert_eq!(recursive.artifacts().reference_provenance().len(), 2);
    assert_eq!(
        recursive.record_types()[0].fields()[0]
            .resolved_type()
            .to_string(),
        "Ref<Node>?"
    );

    let fields = compile_artifacts(FIELD_NAME_REFERENCE);
    let edges = fields.reference_provenance();
    assert_eq!(edges.len(), 2);
    assert_eq!(edges[0].target_element_id(), edges[1].target_element_id());
    assert_eq!(edges[0].value_path(), ["dependency"]);
    assert_eq!(edges[1].value_path(), ["owner"]);

    let combined = compile_reader(COMBINED_REUSE_REFERENCE);
    assert_eq!(combined.artifacts().reuse_provenance().len(), 5);
    assert_eq!(combined.artifacts().reference_provenance().len(), 1);
}

#[test]
/// Verifies reference target name, kind, and exact type rejection boundaries.
fn conformance_typed_references_negative_oracles() {
    let cases = [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/references/unknown-target.neu"
            )
            .as_slice(),
            ResultClass::Reference,
            diagnostics::UNKNOWN_REFERENCE_TARGET,
            (103, 110),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/references/wrong-kind-target.neu"
            )
            .as_slice(),
            ResultClass::Reference,
            diagnostics::WRONG_REFERENCE_TARGET_KIND,
            (143, 149),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/references/wrong-target-type.neu"
            )
            .as_slice(),
            ResultClass::Reference,
            diagnostics::REFERENCE_TARGET_TYPE_MISMATCH,
            (143, 148),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/references/invariant-target-type.neu"
            )
            .as_slice(),
            ResultClass::Reference,
            diagnostics::REFERENCE_TARGET_TYPE_MISMATCH,
            (106, 111),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/references/missing-constructor.neu"
            )
            .as_slice(),
            ResultClass::Semantics,
            diagnostics::TYPE_MISMATCH,
            (126, 131),
        ),
    ];
    for (source, expected_class, expected_code, expected_span) in cases {
        let failure = compile_failure(source);
        assert_eq!(failure.class(), expected_class);
        assert_eq!(failure.diagnostics()[0].code().as_str(), expected_code);
        assert_eq!(
            span_pair(failure.diagnostics()[0].primary().span()),
            expected_span
        );
    }
}

#[test]
/// Verifies reference targets and fingerprints are independent of declaration order.
fn property_typed_reference_order_is_nonsemantic() {
    let forward = compile_artifacts(FORWARD_REFERENCE);
    let reordered = compile_artifacts(
        b"// SPDX-License-Identifier: Apache-2.0\nneu \"0.1\"\nmodule reference_forward\nrecord Config {\n string image,\n}\nConfig config = { image: \"example.invalid/tool:1\", }\nRef<Config> selected = ref(config)\n",
    );
    assert_eq!(forward.logical_document(), reordered.logical_document());
}

#[test]
/// Verifies ordinary reuse of a reference remains distinct and reader-valid.
fn property_reference_reuse_preserves_both_provenance_kinds() {
    let artifacts = compile_artifacts(
        b"// SPDX-License-Identifier: Apache-2.0\nneu \"0.1\"\nmodule reference_reuse\nRef<string> second = first\nRef<string> first = ref(value)\nstring value = \"value\"\n",
    );
    assert_eq!(artifacts.reuse_provenance().len(), 1);
    assert_eq!(artifacts.reference_provenance().len(), 2);
    ValidatedDocument::from_compiler_output(artifacts)
        .expect("reused references must retain complete edge provenance");
}

#[test]
/// Verifies reader and probe reject missing/dangling edges and traverse IDs.
fn system_typed_references_cross_reader_and_probe() {
    let artifacts = compile_artifacts(FORWARD_REFERENCE);
    let missing = Arc::new(
        artifacts
            .as_ref()
            .clone()
            .with_reference_provenance(Vec::new()),
    );
    assert!(matches!(
        ValidatedDocument::from_compiler_output(missing),
        Err(ReaderError::InvalidReferenceEdge)
    ));

    let owner = artifacts.reference_provenance()[0].element_id();
    let dangling = Arc::new(artifacts.as_ref().clone().with_reference_provenance(vec![
        ReferenceProvenanceRecord::new(owner, Vec::new(), owner),
    ]));
    assert!(matches!(
        ValidatedDocument::from_compiler_output(dangling),
        Err(ReaderError::InvalidReferenceEdge)
    ));

    let document = ValidatedDocument::from_compiler_output(artifacts)
        .expect("compiler identity edges must validate");
    let summary = summarize(&document);
    assert_eq!(summary.reference_provenance(), ["2::1"]);
    assert!(
        summary
            .declarations()
            .iter()
            .any(|value| value.ends_with("ref(#1)"))
    );
}

#[test]
/// Verifies duplicate IDs and stale fingerprints never produce validated views.
fn security_reader_rejects_invalid_graph_identity_states() {
    let artifacts = compile_artifacts(FORWARD_REFERENCE);
    let document = artifacts.logical_document();
    let mut declarations = document.declarations().to_vec();
    let duplicate_id = declarations[0].element_id();
    let original = declarations[1].clone();
    declarations[1] = Declaration::new(
        duplicate_id,
        original.symbol_identity().clone(),
        original.fingerprint(),
        original.name(),
        original.resolved_type().clone(),
        original.value().clone(),
    );
    let invalid_document = LogicalDocument::with_record_types(
        document.module().clone(),
        document.record_types().to_vec(),
        declarations,
    );
    let invalid = replace_logical_document(&artifacts, invalid_document);
    assert!(matches!(
        ValidatedDocument::from_compiler_output(Arc::new(invalid)),
        Err(ReaderError::DuplicateElementId)
    ));

    let mut declarations = document.declarations().to_vec();
    assert_ne!(declarations[0].fingerprint(), declarations[1].fingerprint());
    let original = declarations[1].clone();
    declarations[1] = Declaration::new(
        original.element_id(),
        original.symbol_identity().clone(),
        declarations[0].fingerprint(),
        original.name(),
        original.resolved_type().clone(),
        original.value().clone(),
    );
    let invalid_document = LogicalDocument::with_record_types(
        document.module().clone(),
        document.record_types().to_vec(),
        declarations,
    );
    let invalid = replace_logical_document(&artifacts, invalid_document);
    assert!(matches!(
        ValidatedDocument::from_compiler_output(Arc::new(invalid)),
        Err(ReaderError::InvalidDeclarationFingerprint)
    ));
}
