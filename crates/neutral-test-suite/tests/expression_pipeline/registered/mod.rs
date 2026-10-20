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

/// Compares every registered core case with the approved current oracle revision.
#[test]
fn integration_registered_source_cases_match_independent_literal_oracles() {
    let registry: J = serde_json::from_str(&input("cases.json")).unwrap();
    let oracles: J = serde_json::from_str(&input("results-v2.json")).unwrap();
    let mut failures = Vec::new();
    for case in registry["cases"].as_array().unwrap() {
        // Authoring is a separately gated service, not source compilation or artifact reading.
        if case.get("authoring_profile").is_some() {
            continue;
        }
        let expected = oracles["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == case["id"])
            .unwrap();
        let observed = std::panic::catch_unwind(|| check(case, expected, &registry));
        if let Err(failure) = observed {
            let message = failure.downcast_ref::<String>().map_or_else(
                || {
                    failure
                        .downcast_ref::<&str>()
                        .copied()
                        .unwrap_or("unknown failure")
                        .to_owned()
                },
                Clone::clone,
            );
            failures.push(format!("{}: {message}", case["id"]));
        }
    }
    assert_eq!(failures, [] as [String; 0], "{}", failures.join("\n"));
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
    if case.get("reader_profile").is_some() {
        assert!(
            neutral_encoding::composition::decode_composition_project(
                &bytes,
                DecodeLimits::hard(),
                project.ir().limits,
                composition_limits(),
                &cancel
            )
            .is_err()
        );
        assert_eq!(expected["code"], "Unsupported");
        return;
    }
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
