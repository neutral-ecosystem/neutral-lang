// SPDX-License-Identifier: Apache-2.0

//! Frozen /2 design inputs; deliberately not compiler/codec activation evidence.

use neutral_core::{
    CancellationToken, SemanticDigest, SourceContentDigest, StructuralLimits,
    VocabularyContentDigest,
};
use neutral_vocabulary::{
    VocabularyLimits, VocabularyLock,
    composition::{CapturedCompositionBundle, CompositionLimits, validate_composition_closure},
};
use serde_json::{Value, json};
use std::{
    io::Write,
    path::Path,
    process::{Command, Stdio},
};

/// Runtime-owned immutable vector copy, not a generated measurement report.
const VECTORS: &str = include_str!("vectors.json");
/// Runtime-owned literal request families; compiler expectations remain frozen-only.
const REQUESTS: &[&str] = &[
    include_str!("positive.json"),
    include_str!("negative.json"),
    include_str!("boundary.json"),
    include_str!("migration.json"),
];
/// Containers in the frozen NHT grammar, independently walked rather than re-encoded.
const CONTAINERS: &[&str] = &[
    "neutral-nht-v1",
    "neutral/project-captured/v2",
    "neutral/project-logical/v2",
    "neutral/project-derivation/v2",
    "neutral/project-artifact/v2",
    "sources",
    "source",
    "vocabularies",
    "vocabulary",
    "features",
    "modules",
    "module",
    "imports",
    "declarations",
    "declaration",
    "signature",
    "binding",
    "record",
    "variant",
    "alternative",
    "field",
    "restrictions",
    "choices",
    "minimum",
    "maximum",
    "min-length",
    "max-length",
    "default",
    "value",
    "payload",
    "List",
    "Ref",
    "nullable",
    "nominal",
    "num",
    "vocabulary-types",
    "definition",
    "body",
    "vocabulary-catalogues",
    "public-types",
    "vocabulary-dependencies",
    "dependencies",
    "dependency",
    "public-edges",
    "edge",
    "from",
    "to",
    "capture-limits",
    "project-limits",
    "composition-limits",
    "roots",
    "root",
    "options",
    "option",
];

/// Parses fixed vectors without generating expected values at test time.
fn vectors() -> Value {
    serde_json::from_str(VECTORS).unwrap()
}

/// Executes the independent test oracle; missing Python is a failure, never a skip.
fn inspect(request: &Value) -> Value {
    let script =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/composition_contract/reference.py");
    let program =
        std::env::var("NEUTRAL_IDENTITY_REFERENCE_PYTHON").unwrap_or_else(|_| "python3".to_owned());
    let mut child = Command::new(program)
        .arg("-B")
        .arg(script)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("[error] Python 3 is required for frozen composition vectors");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(request).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "[error] composition oracle: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Decodes lowercase literal hex with no use of a production identity encoder.
fn bytes(value: &Value) -> Vec<u8> {
    let text = value.as_str().unwrap();
    assert_eq!(text.len() % 2, 0);
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).unwrap())
        .collect()
}

/// Independently walks every literal length prefix and nested frame boundary.
fn frame_count(mut input: &[u8], depth: usize, parent: &str) -> u64 {
    assert!(depth < 64);
    let mut count = 0;
    while !input.is_empty() {
        assert!(input.len() >= 10, "invalid frame within {parent}");
        let tag_len = usize::from(u16::from_be_bytes(input[..2].try_into().unwrap()));
        let header = 2 + tag_len + 8;
        assert!(input.len() >= header);
        let tag = std::str::from_utf8(&input[2..2 + tag_len]).unwrap();
        let length = usize::try_from(u64::from_be_bytes(
            input[2 + tag_len..header].try_into().unwrap(),
        ))
        .unwrap();
        let end = header.checked_add(length).unwrap();
        assert!(input.len() >= end);
        count += 1;
        // The frozen grammar is contextual: a symbol's `module` is text,
        // limit fields are integers, and an artifact option's `value` is text.
        let container = CONTAINERS.contains(&tag)
            && !parent.ends_with("-limits")
            && (tag != "module" || parent == "modules")
            && (tag != "declaration" || parent == "declarations")
            && (tag != "value" || parent != "option");
        if container {
            count += frame_count(&input[header..end], depth + 1, tag);
        }
        input = &input[end..];
    }
    count
}

/// Checks fixed bytes, digest, length and frame count through a second independent path.
fn literal(expected: &Value) {
    let transcript = bytes(&expected["transcript_hex"]);
    assert_eq!(
        SemanticDigest::from_transcript(&transcript).to_string(),
        expected["sha256"].as_str().unwrap()
    );
    assert_eq!(
        transcript.len() as u64,
        expected["transcript_bytes"].as_u64().unwrap()
    );
    assert_eq!(
        frame_count(&transcript, 0, ""),
        expected["frames"].as_u64().unwrap()
    );
}

/// Applies reviewed exact JSON pointers to a fresh request, without canonicalizing bad input.
fn variant(base: &Value, edits: &Value) -> Value {
    let mut request = base.clone();
    for edit in edits.as_array().unwrap() {
        let path = edit["path"].as_str().unwrap();
        if path == "/excluded" {
            request["excluded"] = edit["value"].clone();
        } else {
            *request.pointer_mut(path).unwrap() = edit["value"].clone();
        }
    }
    request
}

/// Five baseline partitions match literal bytes; all semantic/control variants are pinned.
#[test]
fn conformance_composition_contract_literal_identity_partitions() {
    let data = vectors();
    assert_eq!(data["identity_profile"], "neutral.project-identity/2");
    assert_eq!(inspect(&data["request"]), data["expected"]);
    for expected in data["expected"].as_object().unwrap().values() {
        literal(expected);
    }
    let baseline = &data["expected"]["logical"]["sha256"];
    for case in data["logical_variants"].as_array().unwrap() {
        let mut request = variant(&data["request"], &case["edits"]);
        request["operation"] = json!("logical");
        assert_eq!(inspect(&request), case["expected"], "{}", case["id"]);
        literal(&case["expected"]);
        assert_eq!(
            case["expected"]["sha256"] == *baseline,
            case["logical_equal_to_baseline"].as_bool().unwrap(),
            "{}",
            case["id"]
        );
    }
}

/// Root choice and occurrence/presentation controls change no complete upstream partition.
#[test]
fn property_composition_contract_roots_and_origins_are_not_meaning() {
    let data = vectors();
    let case = &data["logical_variants"][1];
    let actual = inspect(&variant(&data["request"], &case["edits"]));
    for layer in ["captured", "logical", "derivation", "artifact-project"] {
        assert_eq!(actual[layer], data["expected"][layer]);
    }
    assert_ne!(actual["artifact-view"], data["expected"]["artifact-view"]);
    let mut changed = data["request"].clone();
    changed["capture"]["sources"][0]["source_id"] = json!("source:renamed");
    let actual = inspect(&changed);
    assert_eq!(actual["logical"], data["expected"]["logical"]);
    assert_ne!(actual["captured"], data["expected"]["captured"]);
    assert_ne!(actual["derivation"], data["expected"]["derivation"]);
}

/// All four layers enforce independent exact bytes/frames and reject one below their needs.
#[test]
fn security_composition_contract_exact_identity_limits_and_cancellation() {
    let data = vectors();
    for (layer, expected) in data["expected"].as_object().unwrap() {
        for key in ["bytes", "nodes"] {
            let count = expected[if key == "bytes" {
                "transcript_bytes"
            } else {
                "frames"
            }]
            .as_u64()
            .unwrap();
            let mut request = data["request"].clone();
            // Isolate the selected identity so upstream budgets do not hide its exact boundary.
            request["upstream"] = json!({
                "captured":data["expected"]["captured"]["sha256"],
                "logical":data["expected"]["logical"]["sha256"],
                "derivation":data["expected"]["derivation"]["sha256"]
            });
            if layer.starts_with("artifact-") {
                request["operation"] = json!("artifact");
                request["selected_artifact"] = data["request"]["artifacts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|v| v["layer"] == *layer)
                    .unwrap()
                    .clone();
            } else {
                request["operation"] = json!(layer);
            }
            request["limits"][key] = json!(count);
            let exact = inspect(&request);
            assert_eq!(&exact, expected);
            request["limits"][key] = json!(count - 1);
            let over = inspect(&request);
            assert_eq!(over["error"], "Limit");
        }
    }
    let mut request = data["request"].clone();
    request["operation"] = json!("logical");
    request["cancelled"] = json!(true);
    assert_eq!(inspect(&request), json!({"error":"Cancelled"}));
    request["cancelled"] = json!(false);
    request["limits"]["nodes"] = json!(0);
    assert_eq!(inspect(&request), json!({"error":"Limit"}));
}

/// Canonical order, type wrapper and exact-number faults fail rather than silently normalize.
#[test]
fn security_composition_contract_rejects_noncanonical_resolved_facts() {
    let data = vectors();
    for case in data["rejections"].as_array().unwrap() {
        let mut request = variant(&data["request"], &case["edits"]);
        request["operation"] = case["operation"].clone();
        if case["cancelled"] == true {
            request["cancelled"] = json!(true);
        }
        assert_eq!(inspect(&request), case["expected"], "{}", case["id"]);
    }
}

/// Separate literal oracles agree with request outcomes and enumerate all identity vectors.
#[test]
fn conformance_composition_contract_oracle_inventory_is_complete() {
    let oracle: Value = serde_json::from_str(include_str!("oracle.json")).unwrap();
    for (family, raw) in ["positive", "negative", "boundary", "migration"]
        .iter()
        .zip(REQUESTS)
    {
        let data: Value = serde_json::from_str(raw).unwrap();
        assert_eq!(
            SemanticDigest::from_transcript(raw.as_bytes()).to_string(),
            oracle["fixture_digests"][family].as_str().unwrap()
        );
        let expected = oracle["families"][family].as_array().unwrap();
        let cases = data["cases"].as_array().unwrap();
        assert_eq!(cases.len(), expected.len());
        for (case, expected) in cases.iter().zip(expected) {
            for key in [
                "id",
                "catalogue_outcome",
                "project_outcome",
                "project_code",
                "composition_limits",
            ] {
                assert_eq!(case[key], expected[key], "{family}/{key}");
            }
        }
    }
    let data = vectors();
    assert_eq!(
        SemanticDigest::from_transcript(VECTORS.as_bytes()).to_string(),
        oracle["fixture_digests"]["vectors"].as_str().unwrap()
    );
    assert_eq!(data["identity_profile"], oracle["identity"]["profile"]);
    assert_eq!(
        data["logical_variants"].as_array().unwrap().len() as u64,
        oracle["identity"]["logical_variants"].as_u64().unwrap()
    );
    assert_eq!(
        data["rejections"].as_array().unwrap().len() as u64,
        oracle["identity"]["rejection_vectors"].as_u64().unwrap()
    );
}

/// Successor design never rewrites the accepted independent /1 literal bytes.
#[test]
fn compatibility_composition_contract_preserves_frozen_identity_vectors() {
    let digest =
        SourceContentDigest::from_bytes(include_bytes!("../project_identity/vectors.json"));
    assert_eq!(
        digest.to_string(),
        "sha256:a09a1e281274bd154d7add38aed96e1da938711b0ec1f85c419d533679c86af7"
    );
}

/// Every embedded source/bundle digest binds its literal bytes, without absolute host paths.
#[test]
fn conformance_composition_contract_capture_inputs_have_exact_byte_pins() {
    for raw in REQUESTS {
        let data: Value = serde_json::from_str(raw).unwrap();
        assert_eq!(
            data["scope"],
            "frozen-contract-inputs; project-not-activated"
        );
        for case in data["cases"].as_array().unwrap() {
            for source in case["capture"]["sources"].as_array().unwrap() {
                let bytes = source["source_utf8"].as_str().unwrap().as_bytes();
                assert_eq!(
                    SourceContentDigest::from_bytes(bytes).to_string(),
                    format!("sha256:{}", source["digest"].as_str().unwrap())
                );
                assert_eq!(bytes.len() as u64, source["byte_len"].as_u64().unwrap());
            }
            for bundle in case["capture"]["vocabularies"].as_array().unwrap() {
                let bytes = bundle["bundle_utf8"].as_str().unwrap().as_bytes();
                assert_eq!(
                    VocabularyContentDigest::from_bytes(bytes).to_string(),
                    format!("sha256:{}", bundle["digest"].as_str().unwrap())
                );
                assert_eq!(bytes.len() as u64, bundle["byte_len"].as_u64().unwrap());
            }
        }
    }
}

/// New literal dependency, schema, recursion and independent bound cases execute the catalogue API.
#[test]
fn conformance_composition_contract_catalogue_oracles_execute_without_portable() {
    for raw in REQUESTS {
        let data: Value = serde_json::from_str(raw).unwrap();
        for case in data["cases"].as_array().unwrap() {
            let inputs = case["capture"]["vocabularies"].as_array().unwrap();
            if inputs.is_empty() {
                continue;
            }
            let locks = inputs
                .iter()
                .map(|input| {
                    VocabularyLock::new(
                        input["identity"].as_str().unwrap(),
                        input["version"].as_str().unwrap(),
                        input["encoding_version"].as_str().unwrap(),
                        input["schema_version"].as_str().unwrap(),
                        VocabularyContentDigest::parse_text(&format!(
                            "sha256:{}",
                            input["digest"].as_str().unwrap()
                        ))
                        .unwrap(),
                        input["features"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|v| v.as_str().unwrap().to_owned())
                            .collect(),
                    )
                    .unwrap()
                })
                .collect::<Vec<_>>();
            let captured = inputs
                .iter()
                .zip(&locks)
                .map(|(input, lock)| CapturedCompositionBundle {
                    bytes: input["bundle_utf8"].as_str().unwrap().as_bytes(),
                    lock,
                })
                .collect::<Vec<_>>();
            let roots = case["roots"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| (v[0].as_str().unwrap(), v[1].as_str().unwrap()))
                .collect::<Vec<_>>();
            let mut limits = CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
                StructuralLimits::new(65_536, 64).unwrap(),
            ));
            if let Some(value) = case["composition_limits"]["alternatives_per_type"].as_u64() {
                limits.alternatives_per_type = value;
            }
            if let Some(value) = case["composition_limits"]["dependency_edges"].as_u64() {
                limits.dependency_edges = value;
            }
            let actual =
                validate_composition_closure(&captured, &roots, limits, &CancellationToken::new());
            let expected = case["catalogue_outcome"].as_str().unwrap();
            if expected == "accepted" {
                assert!(actual.is_ok(), "{}: {actual:?}", case["id"]);
            } else {
                assert_eq!(
                    actual.unwrap_err().diagnostic_code(),
                    expected,
                    "{}",
                    case["id"]
                );
            }
        }
    }
}
