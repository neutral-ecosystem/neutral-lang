// SPDX-License-Identifier: Apache-2.0

//! lists cross-package regressions.

use super::*;

#[test]
/// Verifies ordered, empty, nested, nullable, and defaulted list values.
fn conformance_ordered_lists_positive_oracles() {
    let ordered = compile_artifacts(ORDERED_STRINGS);
    assert_eq!(
        ordered.logical_document().declarations()[0]
            .resolved_type()
            .to_string(),
        "List<string>"
    );
    assert_eq!(
        ordered.logical_document().declarations()[0]
            .value()
            .to_string(),
        "[\"first\", \"second\", \"third\"]"
    );
    assert_eq!(
        compile_artifacts(EMPTY_LIST)
            .logical_document()
            .declarations()[0]
            .value()
            .to_string(),
        "[]"
    );
    assert_eq!(
        compile_artifacts(NESTED_NULLABLE_LIST)
            .logical_document()
            .declarations()[0]
            .value()
            .to_string(),
        "[[\"first\", null], [], [\"last\"]]"
    );
    let combined = compile_artifacts(RECORD_LIST_DEFAULT);
    assert_eq!(
        combined.logical_document().declarations()[0]
            .value()
            .to_string(),
        "{items: [{name: \"default\"}], labels: []}"
    );
}

#[test]
/// Verifies malformed and heterogeneous list items have stable ownership.
fn conformance_ordered_lists_negative_oracles() {
    let cases: [FailureOracle<'_>; 2] = [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/lists/wrong-item-type.neu"
            ),
            ResultClass::Semantics,
            diagnostics::TYPE_MISMATCH,
            (105, 109),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/lists/missing-comma.neu"
            ),
            ResultClass::Syntax,
            diagnostics::MALFORMED_BOUNDARY,
            (107, 115),
        ),
    ];
    for (source, class, code, expected_span) in cases {
        let failure = compile_failure(source);
        assert_eq!(failure.class(), class);
        assert_eq!(failure.diagnostics()[0].code().as_str(), code);
        assert_eq!(
            span_pair(failure.diagnostics()[0].primary().span()),
            expected_span
        );
    }
}

#[test]
/// Verifies item, nesting, and traversal limits fail before excess growth.
fn security_list_limits_fail_before_proportional_allocation() {
    let cases = [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/lists/item-limit.neu"
            ) as &[u8],
            limits()
                .with_list_items(2)
                .expect("list item limit should be valid"),
            (100, 101),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/lists/nesting-limit.neu"
            ),
            limits()
                .with_nesting_depth(1)
                .expect("list depth limit should be valid"),
            (103, 104),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/lists/traversal-limit.neu"
            ),
            limits()
                .with_traversal_nodes(2)
                .expect("traversal limit should be valid"),
            (102, 103),
        ),
    ];
    for (source, limits, expected_span) in cases {
        let result = compile(CompilationRequest::new(
            source.to_vec(),
            limits,
            CancellationToken::new(),
        ))
        .expect("bounded list fixture should capture");
        let CompilationResult::Failure(failure) = result else {
            panic!("over-limit list must produce no IR");
        };
        assert_eq!(failure.class(), ResultClass::Resource);
        assert_eq!(
            failure.diagnostics()[0].code().as_str(),
            diagnostics::LIST_LIMIT_EXCEEDED
        );
        assert_eq!(
            span_pair(failure.diagnostics()[0].primary().span()),
            expected_span
        );
    }
}

#[test]
/// Verifies list order is logical and generic arguments are invariant.
fn property_list_order_and_invariant_types_are_logical() {
    let first =
        compile_artifacts(b"neu \"0.1\"\nmodule order\nList<string> values = [\"a\", \"b\"]\n");
    let second =
        compile_artifacts(b"neu \"0.1\"\nmodule order\nList<string> values = [\"b\", \"a\"]\n");
    assert_ne!(
        first.logical_document().declarations()[0].fingerprint(),
        second.logical_document().declarations()[0].fingerprint()
    );
    assert_ne!(
        neutral_ir::ResolvedType::list(neutral_ir::ResolvedType::String),
        neutral_ir::ResolvedType::list(neutral_ir::ResolvedType::nullable(
            neutral_ir::ResolvedType::String,
        ))
    );
}

#[test]
/// Verifies lists and closed list defaults cross reader and probe boundaries.
fn system_ordered_lists_cross_reader_and_probe() {
    let document = compile_reader(RECORD_LIST_DEFAULT);
    let summary = summarize(&document);
    assert_eq!(
        summary.declarations(),
        ["config: Config = {items: [{name: \"default\"}], labels: []}"]
    );
    assert!(summary.record_types()[0].contains("List<Item>"));
    assert!(summary.record_types()[0].contains("List<string> = []"));
}
