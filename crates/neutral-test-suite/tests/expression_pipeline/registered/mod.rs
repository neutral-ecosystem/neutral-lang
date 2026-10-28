// SPDX-License-Identifier: Apache-2.0

//! Immutable registered source/result oracles against compilation and independent decoding.

use super::*;
use neutral_ir::project_identity::{IdentityLimits, canonical_expression_project};
use neutral_ir::project_interface::ProjectPublicType as T;
use neutral_semantics::{
    Binding, BindingKey, BindingState, EvaluationRequest, EvaluationResult as R,
};
use serde_json::{Value as J, json};

mod evaluation;
mod requests;

/// Reads only test-owned literal inputs; normal tests do not need the active portable folder.
fn input(path: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/expression_pipeline/fixtures/registered")
            .join(path),
    )
    .unwrap()
}

/// Projects observed typed values, not producer/evaluator-generated expectations.
fn project_value(value: &V) -> J {
    match value {
        V::Number(n) => {
            let mut digits = n.coefficient().to_owned();
            if n.scale() >= 0 {
                digits.push_str(&"0".repeat(usize::try_from(n.scale()).unwrap()));
            } else {
                let places = usize::try_from(n.scale().unsigned_abs()).unwrap();
                if places >= digits.len() {
                    digits = format!("0.{}{}", "0".repeat(places - digits.len()), digits);
                } else {
                    digits.insert(digits.len() - places, '.');
                }
            }
            if n.is_negative() {
                digits.insert(0, '-');
            }
            J::String(digits)
        }
        V::String(s) | V::Url(s) | V::Path(s) => J::String(s.clone()),
        V::Bool(b) => J::Bool(*b),
        V::Null => J::Null,
        V::List(items) => J::Array(items.iter().map(project_value).collect()),
        V::Record(fields) => J::Object(
            fields
                .iter()
                .filter_map(|(n, v)| v.as_ref().map(|v| (n.clone(), project_value(v))))
                .collect(),
        ),
        V::Variant { tag, payload } => json!({"tag":tag,"payload":project_value(payload)}),
        V::Reference(owner) => json!([
            owner.module().language_behavior_version(),
            owner.module().module_name(),
            owner.declaration_name()
        ]),
    }
}

/// Every copied source/bundle byte remains pinned to its independently reviewed digest.
#[test]
fn integration_registered_source_inputs_match_reviewed_pins() {
    let registry: J = serde_json::from_str(&input("cases.json")).unwrap();
    for file in registry["files"].as_array().unwrap() {
        let path = file["path"]
            .as_str()
            .unwrap()
            .rsplit_once("/expressions/")
            .unwrap()
            .1;
        let digest = neutral_core::SourceContentDigest::from_bytes(input(path).as_bytes());
        assert_eq!(
            digest.to_string().strip_prefix("sha256:").unwrap(),
            file["sha256"].as_str().unwrap(),
            "{path}"
        );
    }
}

/// Runs one registered source case against the approved current oracle revision.
fn run_registered_case(id: &str) {
    let registry: J = serde_json::from_str(&input("cases.json")).unwrap();
    let oracles: J = serde_json::from_str(&input("results-v2.json")).unwrap();
    let case = registry["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == id)
        .unwrap_or_else(|| panic!("unregistered case: {id}"));
    let expected = oracles["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|result| result["id"] == id)
        .unwrap_or_else(|| panic!("missing literal oracle: {id}"));
    check(case, expected, &registry);
}

macro_rules! registered_cases {
    ($($test_name:ident => $case_id:literal),+ $(,)?) => {
        const ACTIVE_CASE_IDS: &[&str] = &[$($case_id),+];

        $(
            #[test]
            fn $test_name() {
                run_registered_case($case_id);
            }
        )+
    };
}

// Each active literal source case is a separately named conformance test. The two retired
// implementation cases remain frozen records and are asserted as historical below.
registered_cases! {
    conformance_registered_arithmetic => "arithmetic",
    conformance_registered_selected_guards => "selected-guards",
    conformance_registered_typed_construction => "typed-construction",
    conformance_registered_exhaustive_match => "exhaustive-match",
    conformance_registered_template_default => "template-default",
    conformance_registered_template_deferred_guard => "template-deferred-guard",
    conformance_registered_matrix_order_filter => "matrix-order-filter",
    conformance_registered_matrix_explicit_equivalence => "matrix-explicit-equivalence",
    conformance_registered_duplicate_axis_positions => "duplicate-axis-positions",
    conformance_registered_empty_axis => "empty-axis",
    conformance_registered_vocabulary_computed_restrictions => "vocabulary-computed-restrictions",
    conformance_registered_definition_scope_hygiene => "definition-scope-hygiene",
    conformance_registered_missing_guard => "missing-guard",
    conformance_registered_missing_selected_value => "missing-selected-value",
    conformance_registered_skipped_missing_value => "skipped-missing-value",
    conformance_registered_supplied_selected_value => "supplied-selected-value",
    conformance_registered_selected_unavailable => "selected-unavailable",
    conformance_registered_selected_failed => "selected-failed",
    conformance_registered_skipped_unavailable => "skipped-unavailable",
    conformance_registered_skipped_failed => "skipped-failed",
    conformance_registered_skipped_malformed => "skipped-malformed",
    conformance_registered_duplicate_binding => "duplicate-binding",
    conformance_registered_unknown_binding => "unknown-binding",
    conformance_registered_explicit_null => "explicit-null",
    conformance_registered_null_not_missing => "null-not-missing",
    conformance_registered_skipped_output => "skipped-output",
    conformance_registered_missing_output => "missing-output",
    conformance_registered_branch_type => "branch-type",
    conformance_registered_branch_name => "branch-name",
    conformance_registered_division_zero => "division-zero",
    conformance_registered_comparison_chain => "comparison-chain",
    conformance_registered_match_missing => "match-missing",
    conformance_registered_match_duplicate => "match-duplicate",
    conformance_registered_template_recursion => "template-recursion",
    conformance_registered_template_arguments => "template-arguments",
    conformance_registered_template_static => "template-static",
    conformance_registered_matrix_shape => "matrix-shape",
    conformance_registered_matrix_collision => "matrix-collision",
    conformance_registered_matrix_exclusion => "matrix-exclusion",
    conformance_registered_restrictions => "restrictions",
    conformance_registered_value_cycle => "value-cycle",
    conformance_registered_imperative => "imperative",
    conformance_registered_interpolation_type => "interpolation-type",
    conformance_registered_template_null => "template-null",
    conformance_registered_template_unused_static => "template-unused-static",
    conformance_registered_filtered_product_exact => "filtered-product-exact",
    conformance_registered_filtered_product_one_over => "filtered-product-one-over",
    conformance_registered_aggregate_tuples_exact => "aggregate-tuples-exact",
    conformance_registered_aggregate_tuples_one_over => "aggregate-tuples-one-over",
    conformance_registered_numeric_growth_exact => "numeric-growth-exact",
    conformance_registered_numeric_growth_one_over => "numeric-growth-one-over",
    conformance_registered_utf8_output_exact => "utf8-output-exact",
    conformance_registered_utf8_output_one_over => "utf8-output-one-over",
    conformance_registered_residual_depth_exact => "residual-depth-exact",
    conformance_registered_residual_depth_one_over => "residual-depth-one-over",
    conformance_registered_residual_nodes_exact => "residual-nodes-exact",
    conformance_registered_residual_nodes_one_over => "residual-nodes-one-over",
    conformance_registered_already_cancelled => "already-cancelled",
    conformance_registered_evaluator_cancelled => "evaluator-cancelled",
    conformance_registered_zero_control => "zero-control",
    conformance_registered_old_capture => "old-capture",
    conformance_registered_missing_feature => "missing-feature",
    conformance_registered_unknown_feature => "unknown-feature",
    conformance_registered_header_only => "header-only",
    conformance_registered_static_literal_equivalence => "static-literal-equivalence",
    conformance_registered_public_template_contract => "public-template-contract",
}

/// Keeps the individually named active tests in exact one-to-one correspondence with the corpus.
#[test]
fn integration_registered_source_inventory_has_no_omissions_or_duplicates() {
    let registry: J = serde_json::from_str(&input("cases.json")).unwrap();
    let dispositions: J = serde_json::from_str(&input("dispositions.json")).unwrap();
    let blocked: std::collections::BTreeSet<_> = dispositions["blocked_cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            assert_ne!(entry["reason"].as_str().unwrap(), "");
            entry["id"].as_str().unwrap()
        })
        .collect();
    assert_eq!(
        blocked.len(),
        dispositions["blocked_cases"].as_array().unwrap().len()
    );
    let registered_ids: std::collections::BTreeSet<_> = registry["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case["id"].as_str().unwrap())
        .collect();
    assert_eq!(
        registered_ids.len(),
        registry["cases"].as_array().unwrap().len()
    );
    assert!(blocked.is_subset(&registered_ids));
    let oracles: J = serde_json::from_str(&input("results-v2.json")).unwrap();
    let oracle_ids: std::collections::BTreeSet<_> = oracles["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case["id"].as_str().unwrap())
        .collect();
    assert_eq!(oracle_ids.len(), oracles["cases"].as_array().unwrap().len());
    assert_eq!(registered_ids, oracle_ids);
    let active: Vec<_> = registry["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| !blocked.contains(case["id"].as_str().unwrap()))
        .map(|case| case["id"].as_str().unwrap())
        .collect();
    assert_eq!(active.len(), ACTIVE_CASE_IDS.len());
    let mut listed = ACTIVE_CASE_IDS.to_vec();
    listed.sort_unstable();
    listed.dedup();
    assert_eq!(
        listed.len(),
        ACTIVE_CASE_IDS.len(),
        "duplicate active case ID"
    );
    let mut active = active;
    active.sort_unstable();
    assert_eq!(listed, active);
}

/// Retains frozen evidence for retired reader/authoring implementations without executing them.
#[test]
fn integration_registered_retired_implementation_cases_remain_historical_records() {
    let registry: J = serde_json::from_str(&input("cases.json")).unwrap();
    let oracles: J = serde_json::from_str(&input("results-v2.json")).unwrap();
    for (id, profile, phase, code) in [
        (
            "old-reader",
            "neutral.project-ir/2",
            "reader",
            "Unsupported",
        ),
        (
            "old-authoring",
            "neutral.authoring/1",
            "authoring",
            "NEU-AUTH-001",
        ),
    ] {
        let case = registry["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["id"] == id)
            .unwrap();
        let result = oracles["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|result| result["id"] == id)
            .unwrap();
        assert_eq!(case["id"], id);
        assert_eq!(
            case["reader_profile"]
                .as_str()
                .or(case["authoring_profile"].as_str()),
            Some(profile)
        );
        assert_eq!(result["status"], "failure");
        assert_eq!(result["phase"], phase);
        assert_eq!(result["code"], code);
        assert_eq!(result["partial_result"], false);
    }
}

/// The approved revision changes only the misclassified codes, not old snapshot bytes or values.
#[test]
fn integration_registered_oracle_revision_preserves_frozen_results() {
    let original = input("results.json");
    let mut historical: J = serde_json::from_str(&original).unwrap();
    let revised: J = serde_json::from_str(&input("results-v2.json")).unwrap();
    assert_eq!(
        neutral_core::SourceContentDigest::from_bytes(original.as_bytes())
            .to_string()
            .strip_prefix("sha256:")
            .unwrap(),
        revised["supersedes"]["sha256"].as_str().unwrap()
    );
    for case in historical["cases"].as_array_mut().unwrap() {
        if matches!(
            case["id"].as_str(),
            Some("residual-depth-one-over" | "residual-nodes-one-over")
        ) {
            assert_eq!(case["code"], "NEU-EXP-009");
            case["code"] = json!("NEU-EVAL-006");
        }
    }
    assert_eq!(historical["cases"], revised["cases"]);
}

/// Checks failure boundaries before inspecting any value, then crosses complete wire validation.
fn check(case: &J, expected: &J, registry: &J) {
    let operation = case["operation"].as_str().unwrap();
    let evaluator = operation == "evaluate-binding";
    let captured = match requests::capture(case, registry, evaluator) {
        Ok(captured) => captured,
        Err(code) => {
            assert_eq!(expected["status"], "failure");
            assert_eq!(code, expected["code"].as_str().unwrap());
            return;
        }
    };
    let signal = CancellationToken::new();
    if !evaluator && case["cancelled"] == true {
        signal.cancel();
    }
    let produced = match compile_expression_project(&captured, &signal) {
        Ok(ir) => ir,
        Err(error) => {
            assert_eq!(expected["status"], "failure", "{error:?}");
            assert_eq!(error.code, expected["code"].as_str().unwrap());
            return;
        }
    };
    let limits = SemanticLimits {
        project: produced.limits,
        composition: composition_limits(),
        expression: ExpressionLimits::hard(),
        expansion: ExpansionLimits::hard(),
    };
    let cancel = CancellationToken::new();
    let project = ValidatedExpressionProject::from_ir(produced, limits, &cancel).unwrap();
    let bytes = encode_expression_project(&project, &cancel).unwrap();
    let project = decode_expression_project(&bytes, DecodeLimits::hard(), limits, &cancel).unwrap();
    if evaluator {
        evaluation::check(&project, case, expected, registry);
        return;
    }
    if operation == "logical-compare" {
        compare_identities(&project, &captured, case, expected, registry, &limits);
        return;
    }
    assert_eq!(expected["status"], "value");
    if let Some(values) = expected["values"].as_object() {
        for (name, wanted) in values {
            let declaration = project
                .ir()
                .declarations
                .iter()
                .find(|d| d.identity.declaration_name() == name)
                .unwrap();
            let Some(ExpressionBody::Value(value)) = &declaration.body else {
                panic!("not materialized: {name}");
            };
            assert_eq!(project_value(value), *wanted, "{name}");
        }
    }
    if let Some(names) = expected["generated_symbols"].as_array() {
        let actual: Vec<_> = project
            .ir()
            .declarations
            .iter()
            .filter(|d| d.identity.declaration_name().contains("__"))
            .map(|d| json!(d.identity.declaration_name()))
            .collect();
        assert_eq!(actual, *names);
    }
}

/// Both sides cross independent wire validation; only captured evidence may differ.
fn compare_identities(
    project: &ValidatedExpressionProject,
    captured: &CapturedExpressionProject,
    case: &J,
    expected: &J,
    registry: &J,
    limits: &SemanticLimits,
) {
    let cancel = CancellationToken::new();
    let mut other = case.clone();
    other["source"] = case["other_source"].clone();
    let other_capture = requests::capture(&other, registry, false).unwrap();
    let identity_limits = IdentityLimits {
        bytes: neutral_ir::project_identity::MAX_TRANSCRIPT_BYTES,
        nodes: neutral_ir::project_identity::MAX_TRANSCRIPT_NODES,
    };
    assert_eq!(
        captured
            .identity_transcript(identity_limits, &cancel)
            .unwrap()
            .identity()
            == other_capture
                .identity_transcript(identity_limits, &cancel)
                .unwrap()
                .identity(),
        expected["captured_identity_equal"].as_bool().unwrap()
    );
    let other = compile_expression_project(&other_capture, &cancel).unwrap();
    let other = ValidatedExpressionProject::from_ir(other, *limits, &cancel).unwrap();
    let other_bytes = encode_expression_project(&other, &cancel).unwrap();
    let other =
        decode_expression_project(&other_bytes, DecodeLimits::hard(), *limits, &cancel).unwrap();
    assert_eq!(
        canonical_expression_project(project.ir(), identity_limits, &cancel)
            .unwrap()
            .identity()
            == canonical_expression_project(other.ir(), identity_limits, &cancel)
                .unwrap()
                .identity(),
        expected["logical_identity_equal"].as_bool().unwrap()
    );
}

/// Authoring vectors remain named blockers, not fake production passes from oracle self-checks.
#[test]
fn integration_registered_authoring_vectors_have_explicit_blocking_dispositions() {
    let vectors: J = serde_json::from_str(&input("authoring-vectors.json")).unwrap();
    let dispositions: J = serde_json::from_str(&input("dispositions.json")).unwrap();
    let mut expected = std::collections::BTreeSet::new();
    for family in ["valid", "invalid"] {
        for vector in vectors[family].as_array().unwrap() {
            assert!(expected.insert((family, vector["id"].as_str().unwrap())));
        }
    }
    let records = dispositions["blocked_authoring_vectors"]
        .as_array()
        .unwrap();
    let actual: std::collections::BTreeSet<_> = records
        .iter()
        .map(|record| {
            assert_ne!(record["reason"].as_str().unwrap(), "");
            (
                record["family"].as_str().unwrap(),
                record["id"].as_str().unwrap(),
            )
        })
        .collect();
    assert_eq!(actual.len(), records.len(), "duplicate blocker");
    assert_eq!(
        actual, expected,
        "every authoring case is explicitly blocked until its separate production gate"
    );
}
