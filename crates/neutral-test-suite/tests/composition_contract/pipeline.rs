// SPDX-License-Identifier: Apache-2.0

//! Production source → complete IR → independent reader → restricted successor wire checks.

use super::*;
use neutral_compiler::{
    CapturedCompositionProjectRequest, CapturedProjectRequest, CapturedSourceInput,
    CapturedVocabularyInput, ProjectCaptureControls, ProjectCaptureLimitValues,
    ProjectCaptureLimits, capture_composition_project, compile_composition_project,
};
use neutral_core::profile::LanguageProfile;
use neutral_encoding::{
    DecodeLimits,
    composition::{decode_composition_project, encode_composition_project},
};
use neutral_ir::{
    composition::{BindingValue, profile},
    project::{PROJECT_MAX_DEPTH, ProjectLimits},
};
use neutral_reader::composition::ValidatedCompositionProject;
use std::{fmt::Write as _, sync::Arc};

/// Finite request-local policy independent of ambient machine paths or package releases.
fn capture_limits() -> ProjectCaptureLimits {
    ProjectCaptureLimits::new(ProjectCaptureLimitValues {
        total_source_bytes: 65_536,
        source_bytes_per_unit: 16_384,
        source_units: 64,
        source_id_bytes: 128,
        module_id_bytes: 128,
        vocabulary_units: 64,
        vocabulary_bytes_per_unit: 65_536,
        total_vocabulary_bytes: 262_144,
        imports_per_module: 64,
        import_edges: 256,
        scc_units: 64,
        declarations: 256,
        diagnostics: 64,
        output_bytes: 1_048_576,
    })
}
/// Builds an explicit successor request from immutable literal runtime-owned capture facts.
fn request(case: &Value) -> CapturedCompositionProjectRequest {
    request_with_limits(case, limits_for(case))
}
/// Retains the same explicit capture while independently varying semantic resource controls.
fn request_with_limits(
    case: &Value,
    limits: CompositionLimits,
) -> CapturedCompositionProjectRequest {
    let sources = case["capture"]["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            CapturedSourceInput::new(
                s["source_id"].as_str().unwrap(),
                s["module"].as_str().unwrap(),
                s["source_utf8"].as_str().unwrap().as_bytes().to_vec(),
            )
        })
        .collect();
    let locks = locks(case);
    let vocabularies = case["capture"]["vocabularies"]
        .as_array()
        .unwrap()
        .iter()
        .zip(locks)
        .map(|(v, l)| {
            CapturedVocabularyInput::new(v["bundle_utf8"].as_str().unwrap().as_bytes().to_vec(), l)
        })
        .collect();
    CapturedCompositionProjectRequest::new(
        CapturedProjectRequest::new(
            profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            sources,
            vocabularies,
            ProjectCaptureControls::new(capture_limits(), CancellationToken::new()),
        ),
        profile::REQUIRED_FEATURES
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        limits,
    )
}
/// Wraps runtime-owned source bytes in an explicit no-vocabulary captured request.
fn source_case(text: &str) -> Value {
    json!({"id":"runtime-source", "capture":{"sources":[{"module":"example","source_id":"source:example","source_utf8":text}],"vocabularies":[]}})
}
/// Captures, compiles and independently reads a whole literal project before any encoding.
fn compile(case: &Value) -> ValidatedCompositionProject {
    let captured = capture_composition_project(request(case))
        .unwrap_or_else(|e| panic!("{}: capture {e:?}", case["id"]));
    let ir = compile_composition_project(&captured, &CancellationToken::new())
        .unwrap_or_else(|e| panic!("{}: compile {e:?}", case["id"]));
    let limits = ir.limits;
    ValidatedCompositionProject::from_ir(ir, limits, limits_for(case), &CancellationToken::new())
        .unwrap_or_else(|e| panic!("{}: read {e:?}", case["id"]))
}
/// Source defaults, nested materialization and reuse are ordinary immutable meaning, not execution.
#[test]
fn integration_composition_source_defaults_nested_reuse_and_heterogeneous_variants() {
    let project = compile(&source_case(include_str!("source-defaults.neu")));
    let ir = project.complete_ir();
    let original = ir
        .declarations
        .iter()
        .find(|d| d.identity.declaration_name() == "original")
        .unwrap();
    let copied = ir
        .declarations
        .iter()
        .find(|d| d.identity.declaration_name() == "copied")
        .unwrap();
    assert_eq!(original.value, copied.value);
    assert!(
        ir.origins
            .iter()
            .any(|o| o.kind == neutral_ir::composition::ValueOriginKind::Defaulted)
    );
    let bytes = encode_composition_project(&project, &CancellationToken::new()).unwrap();
    let decoded = decode_composition_project(
        &bytes,
        DecodeLimits::hard(),
        ir.limits,
        limits_for(&source_case("")),
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(decoded.complete_ir(), ir);
}
/// Identity references may cycle; ordinary value reuse cannot evaluate a cycle.
#[test]
fn conformance_composition_reference_cycles_are_not_reuse_cycles() {
    let project = compile(&source_case(include_str!("reference-cycle.neu")));
    assert_eq!(project.bindings().bindings().len(), 2);
    let captured =
        capture_composition_project(request(&source_case(include_str!("reuse-cycle.neu"))))
            .unwrap();
    assert_eq!(
        compile_composition_project(&captured, &CancellationToken::new())
            .unwrap_err()
            .code,
        neutral_compiler::composition_diagnostics::REUSE_CYCLE
    );
}
/// Unknown wrapper/arity, duplicate data and invisible reference targets fail before publication.
#[test]
fn security_composition_source_invalid_semantics_fail_closed() {
    for body in [
        "public variant State { num ready }\npublic State x = { tag: \"ready\", payload: 1, payload: 2 }\n",
        "public variant State { num ready }\npublic State x = { tag: \"ready\", payload: false }\n",
        "public record Node { Ref<Node>? next }\nNode secret = { next: null }\npublic Node shown = { next: ref(secret) }\n",
        "public record Node { num count }\npublic Node x = {}\n",
        "public record Node { Ref<Node> next = ref(x) }\npublic Node x = {}\n",
        "public variant State { List<State> recursive }\n",
        "public record Node { num count = false }\n",
        "public record Node { num count }\npublic Ref<List<Node>> x = null\n",
    ] {
        let captured = capture_composition_project(request(&source_case(&format!(
            "neu \"1.0\"\nmodule example\n{body}"
        ))))
        .unwrap();
        assert!(
            compile_composition_project(&captured, &CancellationToken::new()).is_err(),
            "{body}"
        );
    }
}
/// Compares actual complete logical bytes and digest with the frozen independent implementation.
#[test]
fn conformance_composition_compiler_logical_identity_matches_frozen_second_implementation() {
    let fixture: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let project = compile(&fixture["cases"][0]);
    let vector: Value = serde_json::from_str(VECTORS).unwrap();
    let transcript = neutral_ir::project_identity::canonical_composition_project(
        project.complete_ir(),
        neutral_ir::project_identity::IdentityLimits {
            bytes: 1_048_576,
            nodes: 65_536,
        },
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(
        transcript.identity().to_string(),
        vector["expected"]["logical"]["sha256"].as_str().unwrap()
    );
    let bytes = vector["expected"]["logical"]["transcript_hex"]
        .as_str()
        .unwrap()
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(transcript.bytes(), bytes);
}
/// Every accepted literal source family now traverses the actual compiler and independent codec.
#[test]
fn integration_composition_literal_positive_pipeline_roundtrips() {
    let fixture: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let mut tested = 0;
    for case in fixture["cases"].as_array().unwrap().iter().filter(|c| {
        c["project_outcome"] == "accepted"
            && !c["capture"]["sources"].as_array().unwrap().is_empty()
    }) {
        let project = compile(case);
        let cancel = CancellationToken::new();
        let bytes = encode_composition_project(&project, &cancel).unwrap();
        assert!(bytes.starts_with(&profile::MAGIC));
        let decoded = decode_composition_project(
            &bytes,
            DecodeLimits::hard(),
            project.complete_ir().limits,
            limits_for(case),
            &cancel,
        )
        .unwrap();
        assert_eq!(decoded.complete_ir(), project.complete_ir());
        assert!(
            neutral_encoding::project::decode_project(
                &bytes,
                DecodeLimits::hard(),
                project.complete_ir().limits,
                &cancel
            )
            .is_err()
        );
        tested += 1;
    }
    assert_ne!(tested, 0);
}
/// Checks every registered source-negative diagnostic without replacing the frozen oracle.
#[test]
fn conformance_composition_literal_source_negative_codes() {
    let fixture: Value = serde_json::from_str(REQUESTS[1]).unwrap();
    let mut count = 0;
    for case in fixture["cases"].as_array().unwrap().iter().filter(|c| {
        c["catalogue_outcome"] == "accepted"
            && !c["capture"]["sources"].as_array().unwrap().is_empty()
    }) {
        let captured = capture_composition_project(request(case)).unwrap();
        let error = compile_composition_project(&captured, &CancellationToken::new()).unwrap_err();
        assert_eq!(
            error.code,
            case["project_code"].as_str().unwrap(),
            "{}",
            case["id"]
        );
        count += 1;
    }
    assert_ne!(count, 0);
}
/// Truncation, trailing bytes, wrong magic, consumer bounds and cancellation never publish a prefix.
#[test]
fn security_composition_encoded_artifacts_reject_hostile_frames() {
    let fixture: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let case = &fixture["cases"][0];
    let project = compile(case);
    let cancel = CancellationToken::new();
    let bytes = encode_composition_project(&project, &cancel).unwrap();
    let limits = project.complete_ir().limits;
    for end in 0..bytes.len() {
        assert!(
            decode_composition_project(
                &bytes[..end],
                DecodeLimits::hard(),
                limits,
                limits_for(case),
                &cancel
            )
            .is_err(),
            "prefix {end}"
        );
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(
        decode_composition_project(
            &trailing,
            DecodeLimits::hard(),
            limits,
            limits_for(case),
            &cancel
        )
        .is_err()
    );
    let mut wrong = bytes.clone();
    wrong[0] = 0;
    assert!(
        decode_composition_project(
            &wrong,
            DecodeLimits::hard(),
            limits,
            limits_for(case),
            &cancel
        )
        .is_err()
    );
    let narrow = ProjectLimits {
        artifact_bytes: bytes.len() as u64 - 1,
        ..limits
    };
    assert!(
        decode_composition_project(
            &bytes,
            DecodeLimits::hard(),
            narrow,
            limits_for(case),
            &cancel
        )
        .is_err()
    );
    cancel.cancel();
    assert!(encode_composition_project(&project, &cancel).is_err());
    assert!(
        decode_composition_project(
            &bytes,
            DecodeLimits::hard(),
            limits,
            limits_for(case),
            &cancel
        )
        .is_err()
    );
}
/// Independent semantic validation rejects stale defaults, origin classifications and resource/interface claims.
#[test]
fn security_composition_reader_rechecks_materialized_contracts_and_companions() {
    let fixture: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let case = &fixture["cases"][0];
    let project = compile(case);
    let valid = project.complete_ir();
    let mut mutations = Vec::new();
    let mut ir = (**valid).clone();
    ir.interface_digest = SemanticDigest::from_raw_bytes([0; 32]);
    mutations.push(ir);
    let mut ir = (**valid).clone();
    ir.composition_resources.retained_value_nodes += 1;
    mutations.push(ir);
    let mut ir = (**valid).clone();
    ir.origins.pop();
    mutations.push(ir);
    let mut ir = (**valid).clone();
    ir.source_maps.pop();
    mutations.push(ir);
    let mut ir = (**valid).clone();
    ir.composition_limits.value_depth = PROJECT_MAX_DEPTH as u64 + 1;
    ir.composition_limits.work = 0;
    mutations.push(ir);
    let mut ir = (**valid).clone();
    let binding = ir
        .declarations
        .iter_mut()
        .find(|d| d.identity.declaration_name() == "request")
        .unwrap();
    let Some(BindingValue::Record(fields)) = &mut binding.value else {
        panic!("record")
    };
    fields.iter_mut().find(|(n, _)| n == "attempts").unwrap().1 = Some(BindingValue::Number(
        neutral_ir::ExactNumber::from_source("2", 64, 64).unwrap(),
    ));
    mutations.push(ir);
    for ir in mutations {
        assert!(
            ValidatedCompositionProject::from_ir(
                Arc::new(ir),
                valid.limits,
                limits_for(case),
                &CancellationToken::new()
            )
            .is_err()
        );
    }
}

/// Registered boundary and legacy-leaf captures now traverse the complete successor path.
#[test]
fn conformance_composition_boundary_and_migration_pipeline() {
    for document in [REQUESTS[2], REQUESTS[3]] {
        let fixture: Value = serde_json::from_str(document).unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            if case["catalogue_outcome"] != "accepted" {
                assert!(
                    capture_composition_project(request(case)).is_err(),
                    "{}",
                    case["id"]
                );
                continue;
            }
            if case["capture"]["sources"].as_array().unwrap().is_empty() {
                continue;
            }
            let project = compile(case);
            let bytes = encode_composition_project(&project, &CancellationToken::new()).unwrap();
            let decoded = decode_composition_project(
                &bytes,
                DecodeLimits::hard(),
                project.complete_ir().limits,
                limits_for(case),
                &CancellationToken::new(),
            )
            .unwrap();
            assert_eq!(decoded.complete_ir(), project.complete_ir());
        }
    }
}

/// Independent consumer and advertised producer budgets both constrain complete decoding.
#[test]
fn security_composition_reader_intersects_independent_bounds() {
    let fixture: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let case = &fixture["cases"][0];
    let project = compile(case);
    let valid = project.complete_ir();
    let cancel = CancellationToken::new();
    let bytes = encode_composition_project(&project, &cancel).unwrap();
    let policy = limits_for(case);
    for narrow in [
        CompositionLimits {
            captured_bytes: valid.resources.vocabulary_bytes - 1,
            ..policy
        },
        CompositionLimits {
            total_types: valid.composition_resources.types - 1,
            ..policy
        },
        CompositionLimits {
            total_fields: valid.composition_resources.fields - 1,
            ..policy
        },
        CompositionLimits {
            total_choices: valid.composition_resources.choices - 1,
            ..policy
        },
        CompositionLimits {
            value_nodes: valid.composition_resources.retained_value_nodes - 1,
            ..policy
        },
    ] {
        assert!(
            ValidatedCompositionProject::from_ir(Arc::clone(valid), valid.limits, narrow, &cancel)
                .is_err()
        );
        assert!(
            decode_composition_project(&bytes, DecodeLimits::hard(), valid.limits, narrow, &cancel)
                .is_err()
        );
    }
    let exact = CompositionLimits {
        captured_bytes: valid.resources.vocabulary_bytes,
        total_types: valid.composition_resources.types,
        total_fields: valid.composition_resources.fields,
        total_choices: valid.composition_resources.choices,
        ..policy
    };
    assert!(
        decode_composition_project(&bytes, DecodeLimits::hard(), valid.limits, exact, &cancel)
            .is_ok()
    );
    let exact_wire = ProjectLimits {
        artifact_bytes: bytes.len() as u64,
        ..valid.limits
    };
    assert!(
        decode_composition_project(&bytes, DecodeLimits::hard(), exact_wire, policy, &cancel)
            .is_ok()
    );
    cancel.cancel();
    assert!(
        ValidatedCompositionProject::from_ir(Arc::clone(valid), valid.limits, policy, &cancel)
            .is_err()
    );
    let captured = capture_composition_project(request(case)).unwrap();
    assert_eq!(
        compile_composition_project(&captured, &cancel)
            .unwrap_err()
            .code,
        neutral_compiler::composition_diagnostics::CANCELLED
    );
}

/// Unknown tags, wrong top-level arity and nonminimal CBOR must fail before reader authority.
#[test]
fn security_composition_wire_rejects_unknown_tags_and_nonminimal_array() {
    let fixture: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let case = &fixture["cases"][0];
    let project = compile(case);
    let cancel = CancellationToken::new();
    let bytes = encode_composition_project(&project, &cancel).unwrap();
    let mut hostile = Vec::new();
    let mut nonminimal = bytes[..profile::MAGIC.len()].to_vec();
    assert_eq!(bytes[profile::MAGIC.len()], 0x90);
    nonminimal.extend([0x98, 0x10]);
    nonminimal.extend(&bytes[profile::MAGIC.len() + 1..]);
    hostile.push(nonminimal);
    let mut arity = bytes.clone();
    arity[profile::MAGIC.len()] = 0x91;
    hostile.push(arity);
    for (known, unknown) in [
        (b"supplied".as_slice(), b"surprise".as_slice()),
        (b"defaulted".as_slice(), b"surprised".as_slice()),
        (b"omitted".as_slice(), b"unknown".as_slice()),
    ] {
        let mut changed = bytes.clone();
        let start = changed
            .windows(known.len())
            .position(|part| part == known)
            .unwrap();
        changed[start..start + known.len()].copy_from_slice(unknown);
        hostile.push(changed);
    }
    for bytes in hostile {
        assert!(
            decode_composition_project(
                &bytes,
                DecodeLimits::hard(),
                project.complete_ir().limits,
                limits_for(case),
                &cancel
            )
            .is_err()
        );
    }
}

/// Shuffled capture order and concurrent request execution retain identical complete logical meaning.
#[test]
fn property_composition_pipeline_ordering_and_concurrent_isolation() {
    let fixture: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let case = fixture["cases"][2].clone();
    let baseline = compile(&case);
    let mut shuffled = case.clone();
    shuffled["capture"]["sources"]
        .as_array_mut()
        .unwrap()
        .reverse();
    shuffled["capture"]["vocabularies"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let expected = encode_composition_project(&baseline, &CancellationToken::new()).unwrap();
    std::thread::scope(|threads| {
        let handles = (0..4)
            .map(|_| {
                threads.spawn(|| {
                    let project = compile(&shuffled);
                    encode_composition_project(&project, &CancellationToken::new()).unwrap()
                })
            })
            .collect::<Vec<_>>();
        for handle in handles {
            assert_eq!(handle.join().unwrap(), expected);
        }
    });
}

/// Small source can describe exponentially repeated reuse; request work must stop copies before expansion.
#[test]
fn security_composition_reuse_expansion_pays_before_recursive_copies() {
    let mut source = String::from(
        "neu \"1.0\"\nmodule example\nrecord Layer0 { string text }\nLayer0 value_0 = { text: \"abcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyzabcdefghijklmnopqrstuvwxyz\" }\n",
    );
    for level in 1..=20 {
        let previous = level - 1;
        write!(source, "record Layer{level} {{ Layer{previous} left, Layer{previous} right }}\nLayer{level} value_{level} = {{ left: value_{previous}, right: value_{previous} }}\n").unwrap();
    }
    let case = source_case(&source);
    let mut policy = limits_for(&case);
    policy.work = 100_000;
    let captured = capture_composition_project(request_with_limits(&case, policy)).unwrap();
    assert_eq!(
        compile_composition_project(&captured, &CancellationToken::new())
            .unwrap_err()
            .code,
        neutral_compiler::composition_diagnostics::LIMIT
    );
    // Failure is request-local; no partially memoized expansion can affect another request.
    assert_eq!(
        compile(&source_case("neu \"1.0\"\nmodule example\nnum value = 1\n"))
            .bindings()
            .bindings()
            .len(),
        1
    );
}
