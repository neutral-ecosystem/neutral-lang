// SPDX-License-Identifier: Apache-2.0

//! Executable Stage 4 captured semantic fixtures and their reviewed oracles.

use crate::stage2::{parse_fixture, request_fixture, required_string};
use neutral_compiler::{
    ProjectDependencyKind, analyze_project_semantics, build_module_graph, capture_project,
};
use neutral_core::CancellationToken;

/// Exact portable fixture and oracle bytes, with no crate-owned copy drift.
const CASES: &[&str] = &[
    include_str!("../../../../portable/specs/fixtures/stage4/negative/visibility-leak.toml"),
    include_str!("../../../../portable/specs/fixtures/stage4/negative/private-ref-target.toml"),
    include_str!("../../../../portable/specs/fixtures/stage4/negative/type-incompatible.toml"),
    include_str!("../../../../portable/specs/fixtures/stage4/positive/reuse-provenance.toml"),
    include_str!("../../../../portable/specs/fixtures/stage4/negative/cross-scc-value-cycle.toml"),
];

#[test]
/// Every registered Stage 4 fixture matches its reviewed semantic outcome.
fn conformance_stage4_semantic_cases_match_oracles() {
    let oracle = parse_fixture(include_str!(
        "../../../../portable/conformance/oracles/stage4/semantics.toml"
    ));
    let cases = oracle.arrays.get("case").expect("reviewed Stage 4 cases");
    assert_eq!(cases.len(), CASES.len());
    for text in CASES {
        let fixture = parse_fixture(text);
        let id = required_string(&fixture.root, "case_id");
        let expected = cases
            .iter()
            .find(|case| required_string(case, "id") == id)
            .expect("fixture must have oracle");
        let captured =
            capture_project(request_fixture(text)).expect("Stage 4 fixture must capture");
        let graph = build_module_graph(&captured, &CancellationToken::new())
            .expect("Stage 4 graph must be valid");
        let result = analyze_project_semantics(&captured, &graph, &CancellationToken::new());
        match required_string(expected, "expected_outcome").as_str() {
            "reject-semantics" => {
                let code = result
                    .expect_err("negative fixture must fail")
                    .diagnostics()[0]
                    .code();
                assert_eq!(code, required_string(expected, "diagnostic_code"), "{id}");
            }
            "accept-resolution" => {
                let model = result.expect("positive fixture must pass");
                let order = model
                    .value_order()
                    .iter()
                    .map(|identity| {
                        format!(
                            "{}::{}",
                            identity.module().module_name(),
                            identity.declaration_name()
                        )
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    format!("{order:?}"),
                    expected
                        .get("dependency_first_values")
                        .expect("reviewed value order")
                        .as_str(),
                    "{id}"
                );
                let edge = model
                    .dependencies()
                    .iter()
                    .find(|edge| {
                        edge.kind() == ProjectDependencyKind::Value
                            && edge.from().declaration_name() == "copied"
                    })
                    .expect("reviewed reuse edge");
                assert_eq!(
                    format!(
                        "{}::{} -> {}::{}",
                        edge.from().module().module_name(),
                        edge.from().declaration_name(),
                        edge.to().module().module_name(),
                        edge.to().declaration_name()
                    ),
                    required_string(expected, "value_edge"),
                    "{id}"
                );
                let reference = model
                    .dependencies()
                    .iter()
                    .find(|edge| {
                        edge.kind() == ProjectDependencyKind::Reference
                            && edge.from().declaration_name() == "pointer"
                    })
                    .expect("reviewed reference edge");
                assert_eq!(
                    format!(
                        "{}::{} -> {}::{}",
                        reference.from().module().module_name(),
                        reference.from().declaration_name(),
                        reference.to().module().module_name(),
                        reference.to().declaration_name()
                    ),
                    required_string(expected, "reference_edge"),
                    "{id}"
                );
            }
            other => panic!("unknown reviewed semantic outcome: {other}"),
        }
    }
}
