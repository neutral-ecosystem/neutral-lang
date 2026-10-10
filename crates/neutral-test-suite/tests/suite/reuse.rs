// SPDX-License-Identifier: Apache-2.0

//! reuse cross-package regressions.

use super::*;

#[test]
/// Verifies forward, transitive, nested, and nullable reuse lower final values.
fn conformance_immutable_reuse_positive_oracles() {
    let forward = compile_artifacts(FORWARD_TRANSITIVE_REUSE);
    let declarations = forward.logical_document().declarations();
    assert_eq!(declarations.len(), 3);
    assert!(
        declarations
            .iter()
            .all(|declaration| declaration.value().to_string() == "\"example.invalid/tool:1\"")
    );
    assert_eq!(forward.reuse_provenance().len(), 2);
    assert!(
        forward
            .reuse_provenance()
            .iter()
            .all(|record| record.value_path().is_empty())
    );
    assert_eq!(
        forward
            .provenance()
            .iter()
            .filter(|record| record.origin() == ValueOrigin::OrdinaryReuse)
            .count(),
        2
    );

    let nested = compile_artifacts(NESTED_REUSE);
    let config = nested
        .logical_document()
        .declarations()
        .iter()
        .find(|declaration| declaration.name() == "config")
        .expect("nested reuse fixture must contain config");
    assert_eq!(
        config.value().to_string(),
        "{image: \"example.invalid/tool:1\", labels: [\"portable\", \"portable\"]}"
    );
    assert_eq!(nested.reuse_provenance().len(), 4);

    let nullable = compile_artifacts(NULLABLE_REUSE);
    assert_eq!(
        nullable
            .logical_document()
            .declarations()
            .iter()
            .find(|declaration| declaration.name() == "optional_image")
            .expect("nullable fixture must contain optional_image")
            .value()
            .to_string(),
        "\"example.invalid/tool:1\""
    );

    let defaults = compile_artifacts(REUSE_DEFAULTS);
    let config = defaults
        .logical_document()
        .declarations()
        .iter()
        .find(|declaration| declaration.name() == "config")
        .expect("reuse/default fixture must contain config");
    assert_eq!(
        config.value().to_string(),
        "{image: \"example.invalid/tool:1\", labels: [], note: null}"
    );
    assert_eq!(defaults.reuse_provenance().len(), 2);
}

#[test]
/// Verifies unresolved names, wrong kinds, cycles, and covariance produce no IR.
fn conformance_immutable_reuse_negative_oracles() {
    let cases = [
        (
            include_bytes!("../../../../conformance/releases/v0.1.0/specs/fixtures/negative/reuse/unknown-value.neu")
                .as_slice(),
            diagnostics::UNKNOWN_VALUE,
        ),
        (
            include_bytes!("../../../../conformance/releases/v0.1.0/specs/fixtures/negative/reuse/wrong-kind.neu")
                .as_slice(),
            diagnostics::WRONG_DECLARATION_KIND,
        ),
        (
            include_bytes!("../../../../conformance/releases/v0.1.0/specs/fixtures/negative/reuse/direct-cycle.neu")
                .as_slice(),
            diagnostics::VALUE_CYCLE,
        ),
        (
            include_bytes!("../../../../conformance/releases/v0.1.0/specs/fixtures/negative/reuse/indirect-cycle.neu")
                .as_slice(),
            diagnostics::VALUE_CYCLE,
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/reuse/generic-covariance.neu"
            )
            .as_slice(),
            diagnostics::TYPE_MISMATCH,
        ),
    ];
    for (source, expected_code) in cases {
        let failure = compile_failure(source);
        assert_eq!(failure.class(), ResultClass::Semantics);
        assert_eq!(failure.diagnostics()[0].code().as_str(), expected_code);
    }
    let direct = compile_failure(include_bytes!(
        "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/reuse/direct-cycle.neu"
    ));
    let indirect = compile_failure(include_bytes!(
        "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/reuse/indirect-cycle.neu"
    ));
    assert_eq!(direct.diagnostics()[0].related().len(), 1);
    assert_eq!(indirect.diagnostics()[0].related().len(), 3);
    assert_eq!(
        direct.diagnostics()[0]
            .related()
            .iter()
            .map(|location| span_pair(location.span()))
            .collect::<Vec<_>>(),
        [(92, 97)]
    );
    assert_eq!(
        indirect.diagnostics()[0]
            .related()
            .iter()
            .map(|location| span_pair(location.span()))
            .collect::<Vec<_>>(),
        [(94, 100), (117, 122), (138, 143)]
    );
}

#[test]
/// Verifies declaration order cannot change final reused values or fingerprints.
fn property_immutable_reuse_is_declaration_order_independent() {
    let forward = compile_artifacts(
        b"// SPDX-License-Identifier: Apache-2.0\nneu \"0.1\"\nmodule reuse_order\nstring copy = source\nstring source = \"value\"\n",
    );
    let reverse = compile_artifacts(
        b"// SPDX-License-Identifier: Apache-2.0\nneu \"0.1\"\nmodule reuse_order\nstring source = \"value\"\nstring copy = source\n",
    );
    assert_eq!(forward.logical_document(), reverse.logical_document());
}

#[test]
/// Verifies captured traversal bounds stop dependency-heavy source before IR.
fn security_immutable_reuse_chains_are_bounded() {
    let source = include_bytes!(
        "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/reuse/traversal-limit.neu"
    );
    let bounded = StructuralLimits::new(1_024, 16)
        .expect("reuse security limits must be valid")
        .with_traversal_nodes(3)
        .expect("reuse traversal bound must be valid");
    let request = CompilationRequest::new(source.to_vec(), bounded, CancellationToken::new());
    let CompilationResult::Failure(failure) =
        compile(request).expect("bounded reuse source should capture")
    else {
        panic!("bounded reuse source unexpectedly produced IR");
    };
    assert_eq!(failure.class(), ResultClass::Resource);
    assert_eq!(
        failure.diagnostics()[0].code().as_str(),
        diagnostics::LIST_LIMIT_EXCEEDED
    );
    assert_eq!(
        span_pair(failure.diagnostics()[0].primary().span()),
        (151, 158)
    );
}

#[test]
/// Verifies reader validation and probe output expose immutable reuse edges.
fn system_immutable_reuse_crosses_reader_and_probe() {
    let artifacts = compile_artifacts(FORWARD_TRANSITIVE_REUSE);
    let invalid = Arc::new(artifacts.as_ref().clone().with_reuse_provenance(Vec::new()));
    assert!(matches!(
        ValidatedDocument::from_compiler_output(invalid),
        Err(ReaderError::InvalidReuseProvenance)
    ));

    let document = ValidatedDocument::from_compiler_output(artifacts)
        .expect("compiler reuse provenance must validate");
    let summary = summarize(&document);
    assert_eq!(summary.reuse_provenance().len(), 2);
    assert!(
        summary
            .declarations()
            .iter()
            .all(|declaration| declaration.ends_with("= \"example.invalid/tool:1\""))
    );
}
