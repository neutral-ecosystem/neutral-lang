// SPDX-License-Identifier: Apache-2.0

//! Independent complete vectors must agree with actual source compilation and fresh wire authority.

use super::*;
use neutral_ir::project_identity::{
    IdentityLimits, canonical_expression_project, expression_interface,
};
use serde_json::Value as J;

/// Decodes literal hexadecimal without consulting the production encoder.
fn hex(text: &str) -> Vec<u8> {
    assert!(text.len().is_multiple_of(2));
    text.as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

/// Exact test-owned source ownership matches the frozen independent captured transcript.
fn vector_capture(source: &[u8]) -> CapturedExpressionProject {
    capture_sources(
        vec![CapturedSourceInput::new(
            "source:identity-fixture",
            "example::expressions",
            source.to_vec(),
        )],
        vec![],
    )
}

/// Frozen expected hashes are compared to source-produced identities before and after transport.
#[test]
fn conformance_complete_identity_vectors_cross_source_and_independent_wire() {
    let baselines: J =
        serde_json::from_str(include_str!("fixtures/identity-supplement/baselines.json")).unwrap();
    let folder =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/expression_pipeline/fixtures");
    for metadata in baselines["projects"].as_array().unwrap() {
        let source = std::fs::read(folder.join(metadata["source"].as_str().unwrap())).unwrap();
        let expected = metadata;
        let captured = vector_capture(&source);
        let (_, project) = pipeline_from_capture(&captured);
        let reference = decode_expression_project(
            &hex(metadata["encoded_hex"].as_str().unwrap()),
            DecodeLimits::hard(),
            SemanticLimits {
                project: neutral_encoding::hard_project_limits(),
                composition: composition_limits(),
                expression: ExpressionLimits::hard(),
                expansion: ExpansionLimits::hard(),
            },
            &CancellationToken::new(),
        )
        .unwrap();
        for actual in [&project, &reference] {
            all_partitions(actual, &captured, expected, &reference);
        }
    }
}

/// Reads a frozen ordered control tuple, retaining its arity rather than guessing missing values.
fn control_values<const N: usize>(context: &J, key: &str) -> [u64; N] {
    let values = context["controls"][key].as_array().unwrap();
    assert_eq!(values.len(), N);
    std::array::from_fn(|index| values[index][1].as_u64().unwrap())
}

/// Every complete partition is compared to a literal independent transcript and digest.
fn all_partitions(
    project: &ValidatedExpressionProject,
    captured: &CapturedExpressionProject,
    expected: &J,
    reference: &ValidatedExpressionProject,
) {
    use neutral_ir::project_identity::{
        ArtifactIdentityInput, ArtifactKind, ExpressionDerivationContext,
        expression_artifact_identity, expression_derivation_identity,
    };
    let limits = IdentityLimits {
        bytes: 1_048_576,
        nodes: 100_000,
    };
    let signal = CancellationToken::new();
    let capture = captured.identity_transcript(limits, &signal).unwrap();
    let logical = canonical_expression_project(project.ir(), limits, &signal).unwrap();
    let interface = expression_interface(project.ir(), limits, &signal).unwrap();
    let context = &expected["context"];
    let derivation = expression_derivation_identity(
        logical.identity(),
        &ExpressionDerivationContext {
            captured: capture.identity(),
            producer: context["producer"].as_str().unwrap(),
            producer_version: context["producer_version"].as_str().unwrap(),
            capture_limits: control_values(context, "capture-limits"),
            project_limits: reference.ir().limits,
            composition_limits: reference.ir().composition_limits,
            semantic_profiles: &project.ir().semantic_profiles,
            expression_limits: reference.ir().expression_limits,
            expansion_limits: reference.ir().expansion_limits,
        },
        limits,
        &signal,
    )
    .unwrap();
    let artifact = expression_artifact_identity(
        derivation.identity(),
        &ArtifactIdentityInput {
            kind: ArtifactKind::Project,
            format: context["format"].as_str().unwrap(),
            roots: &[],
            options: &[],
        },
        limits,
        &signal,
    )
    .unwrap();
    for (name, transcript, digest) in [
        ("captured", capture.bytes(), capture.identity().as_bytes()),
        ("logical", logical.bytes(), logical.identity().as_bytes()),
        (
            "interface",
            interface.bytes(),
            interface.identity().as_bytes(),
        ),
        (
            "derivation",
            derivation.bytes(),
            derivation.identity().as_bytes(),
        ),
        ("artifact", artifact.bytes(), artifact.identity().as_bytes()),
    ] {
        assert_eq!(
            transcript,
            hex(expected["partitions"][name]["transcript_hex"]
                .as_str()
                .unwrap()),
            "{}/{name}/transcript",
            expected["id"]
        );
        assert_eq!(
            digest.as_slice(),
            hex(expected["partitions"][name]["sha256"].as_str().unwrap()),
            "{}/{name}/digest",
            expected["id"]
        );
    }
}

/// Independent complete frames and all five source-derived identity partitions must agree with results.
#[test]
fn conformance_supplemental_complete_projects_match_independent_identity_and_results() {
    use neutral_semantics::{
        Binding, BindingKey, BindingState, EvaluationRequest, EvaluationResult,
    };
    let vectors: J =
        serde_json::from_str(include_str!("fixtures/identity-supplement/vectors.json")).unwrap();
    let mut seen = std::collections::BTreeSet::new();
    for case in vectors["cases"].as_array().unwrap() {
        assert!(seen.insert(case["id"].as_str().unwrap()));
        let source = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/expression_pipeline/fixtures/identity-supplement")
                .join(case["source"].as_str().unwrap()),
        )
        .unwrap();
        let capture = vector_capture(&source);
        let (_, produced) = pipeline_from_capture(&capture);
        let literal = hex(case["encoded_hex"].as_str().unwrap());
        let independent = decode_expression_project(
            &literal,
            DecodeLimits::hard(),
            SemanticLimits {
                project: neutral_encoding::hard_project_limits(),
                composition: composition_limits(),
                expression: ExpressionLimits::hard(),
                expansion: ExpansionLimits::hard(),
            },
            &CancellationToken::new(),
        )
        .unwrap();
        assert_eq!(
            encode_expression_project(&independent, &CancellationToken::new()).unwrap(),
            literal
        );
        for project in [&produced, &independent] {
            all_partitions(project, &capture, case, &independent);
            let owner = |name: &str| {
                &project
                    .ir()
                    .declarations
                    .iter()
                    .find(|d| d.identity.declaration_name() == name)
                    .unwrap()
                    .identity
            };
            let boolean = V::Bool(case["bindings"]["enabled"].as_bool().unwrap());
            let count = V::Number(
                ExactNumber::from_source(case["bindings"]["count"].as_str().unwrap(), 128, 128)
                    .unwrap(),
            );
            let bindings = [
                Binding {
                    key: BindingKey::Input(owner("enabled")),
                    ty: &neutral_ir::project_interface::ProjectPublicType::Bool,
                    state: BindingState::Value(&boolean),
                },
                Binding {
                    key: BindingKey::Input(owner("count")),
                    ty: &neutral_ir::project_interface::ProjectPublicType::Num,
                    state: BindingState::Value(&count),
                },
            ];
            let outcome = neutral_semantics::evaluate(
                project.model(),
                &EvaluationRequest {
                    selected: owner("result"),
                    profiles: &project.ir().semantic_profiles,
                    bindings: &bindings,
                    associations: &[],
                    limits: ExpressionLimits::hard(),
                },
                &CancellationToken::new(),
            );
            let EvaluationResult::Value {
                value: V::Number(number),
                ..
            } = outcome
            else {
                panic!("literal complete numeric result: {outcome:?}")
            };
            assert_eq!(
                number.is_negative(),
                case["expected_number"]["negative"].as_bool().unwrap()
            );
            assert_eq!(
                number.coefficient(),
                case["expected_number"]["coefficient"].as_str().unwrap()
            );
            assert_eq!(
                number.scale(),
                case["expected_number"]["scale"].as_i64().unwrap()
            );
        }
    }
    assert_ne!(seen.len(), 0);
}

/// The reference implementation recomputes expectations from explicit tuples and rational inputs only.
#[test]
fn conformance_supplemental_vectors_have_an_independent_read_only_checker() {
    let python =
        std::env::var("NEUTRAL_IDENTITY_REFERENCE_PYTHON").unwrap_or_else(|_| "python3".into());
    let output = std::process::Command::new(python)
        .arg("-B")
        .arg(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/expression_pipeline/fixtures/identity-supplement/verify.py"),
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
