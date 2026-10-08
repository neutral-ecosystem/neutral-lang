// SPDX-License-Identifier: Apache-2.0

//! Allocation-boundary sweeps through real successor producers and independent consumers.

use super::*;
use neutral_compiler::{CompositionCompilationCache, ProjectCacheLimits};
use neutral_core::allocation::testing::{observe, observe_cancellation};
use neutral_ir::project_identity::{IdentityLimits, canonical_composition_project};
use neutral_probe::composition::inspect_composition_encoded;
use neutral_reader::composition::CompositionViewRequest;

/// Borrowed operation under an independently supplied cancellation signal.
type CancellableOperation<'a> = (&'a str, &'a dyn Fn(&CancellationToken) -> bool);

/// Fails every observed reservation independently, requiring atomic failure and subsequent recovery.
fn sweep<T, E: std::fmt::Debug>(
    label: &str,
    mut attempt: impl FnMut(Option<usize>) -> (Result<T, E>, usize),
) -> usize {
    let (baseline, count) = attempt(None);
    baseline.unwrap_or_else(|error| panic!("{label}: baseline {error:?}"));
    assert_ne!(count, 0, "{label}: no observed retention");
    for index in 0..count {
        let (result, observed) = attempt(Some(index));
        assert!(
            result.is_err(),
            "{label}: ignored allocation failure {index}/{count}"
        );
        assert!(observed > index, "{label}: failpoint was not reached");
    }
    let (recovered, recovered_count) = attempt(None);
    recovered.unwrap_or_else(|error| panic!("{label}: recovery {error:?}"));
    assert_eq!(
        recovered_count, count,
        "{label}: failed requests contaminated retention"
    );
    eprintln!("[TEST] {label}: {count} reservation failures rejected; recovery passed");
    count
}

/// Exercises exact-source identity/index copies, repeated aliases, transitive locks and adapted leaf graphs.
#[test]
fn security_composition_capture_allocation_sweep_is_atomic() {
    let mut cases = vec![
        source_case(include_str!("source-defaults.neu")),
        leaf_case(),
    ];
    for text in REQUESTS {
        let inputs: Value = serde_json::from_str(text).unwrap();
        for case in inputs["cases"].as_array().unwrap() {
            if capture_composition_project(request(case)).is_ok() {
                cases.push(case.clone());
            }
        }
    }
    for case in cases {
        sweep("capture", |failure| {
            // Fixture construction belongs to the caller, outside the observed operation.
            let request = request(&case);
            observe(failure, || capture_composition_project(request))
        });
    }
}

/// Creates a caller-owned compatible leaf bundle with real nominal embedding edges and repeated aliases.
fn leaf_case() -> Value {
    let mut case =
        source_case("neu \"1.0\"\nmodule example\nuse Leaf as first\nuse Leaf as second\n");
    let bundle = json!({
        "format": "neutral-vocabulary-bundle",
        "encoding_version": neutral_vocabulary::PROJECT_VOCABULARY_ENCODING_VERSION,
        "schema_version": neutral_vocabulary::PROJECT_VOCABULARY_SCHEMA_VERSION,
        "identity": "Leaf", "version": "1.0.0", "required_features": [],
        "types": [
            {"name":"Inner", "public":true, "fields":[{"name":"value", "type":"num"}]},
            {"name":"Outer", "public":true, "fields":[{"name":"inner", "type":"Inner"}]}
        ]
    });
    let bytes = serde_json::to_string_pretty(&bundle).unwrap();
    let digest = VocabularyContentDigest::from_bytes(bytes.as_bytes()).to_string();
    case["capture"]["vocabularies"] = json!([{
        "identity":"Leaf", "version":"1.0.0", "encoding_version":bundle["encoding_version"],
        "schema_version":bundle["schema_version"], "features":[], "bundle_utf8":bytes,
        "digest":digest.strip_prefix("sha256:").unwrap()
    }]);
    case
}

/// Cancellation at each retained capture/leaf reservation prevents publication through real token checks.
#[test]
fn security_composition_capture_reservation_cancellation_is_atomic() {
    let case = leaf_case();
    let request = request(&case);
    let (_, count) = observe(None, || capture_composition_project(request));
    for index in 0..count {
        let signal = CancellationToken::new();
        let request =
            request_with_policies(&case, capture_limits(), limits_for(&case), signal.clone());
        let (result, observed) =
            observe_cancellation(index, &signal, || capture_composition_project(request));
        assert!(
            signal.is_cancelled(),
            "capture checkpoint {index} was not reached ({observed})"
        );
        assert!(
            result.is_err(),
            "capture published after cancellation at {index}/{count}"
        );
    }
}

/// Caller count/byte/identity controls accept exact retention and reject one-over with unchanged inputs.
#[test]
fn security_composition_capture_retention_bounds_are_independent() {
    let mut case = leaf_case();
    let mut other = case["capture"]["vocabularies"][0].clone();
    let mut bundle: Value = serde_json::from_str(other["bundle_utf8"].as_str().unwrap()).unwrap();
    bundle["identity"] = json!("Other");
    let other_bytes = serde_json::to_string_pretty(&bundle).unwrap();
    other["identity"] = json!("Other");
    other["digest"] = json!(
        VocabularyContentDigest::from_bytes(other_bytes.as_bytes())
            .to_string()
            .strip_prefix("sha256:")
            .unwrap()
    );
    other["bundle_utf8"] = json!(other_bytes);
    case["capture"]["vocabularies"]
        .as_array_mut()
        .unwrap()
        .push(other);
    case["capture"]["sources"][0]["source_utf8"] = json!(
        "neu \"1.0\"\nmodule example\nuse Leaf as first\nuse Leaf as second\nuse Other as other\n"
    );
    case["capture"]["sources"].as_array_mut().unwrap().push(json!({
        "module":"other", "source_id":"source:other", "source_utf8":"neu \"1.0\"\nmodule other\n"
    }));
    let source = &case["capture"]["sources"][0];
    let sizes: Vec<_> = case["capture"]["vocabularies"]
        .as_array()
        .unwrap()
        .iter()
        .map(|input| input["bundle_utf8"].as_str().unwrap().len() as u64)
        .collect();
    let source_bytes = source["source_utf8"].as_str().unwrap().len() as u64;
    let total = source_bytes
        + case["capture"]["sources"][1]["source_utf8"]
            .as_str()
            .unwrap()
            .len() as u64;
    let values = [
        2,
        source_bytes,
        total,
        source["source_id"].as_str().unwrap().len() as u64,
        source["module"].as_str().unwrap().len() as u64,
        *sizes.iter().max().unwrap(),
        sizes.iter().sum(),
        2,
    ];
    for (index, required) in values.into_iter().enumerate() {
        for limit in [required - 1, required, required + 1] {
            let mut bounds = capture_limits().values();
            *match index {
                0 => &mut bounds.source_units,
                1 => &mut bounds.source_bytes_per_unit,
                2 => &mut bounds.total_source_bytes,
                3 => &mut bounds.source_id_bytes,
                4 => &mut bounds.module_id_bytes,
                5 => &mut bounds.vocabulary_bytes_per_unit,
                6 => &mut bounds.total_vocabulary_bytes,
                _ => &mut bounds.vocabulary_units,
            } = limit;
            let request = request_with_policies(
                &case,
                ProjectCaptureLimits::new(bounds),
                limits_for(&case),
                CancellationToken::new(),
            );
            let (result, allocations) = observe(None, || capture_composition_project(request));
            assert_eq!(
                result.is_ok(),
                limit >= required,
                "capture control {index}, limit {limit}"
            );
            if matches!(index, 0 | 7) && limit < required {
                assert_eq!(allocations, 0);
            }
        }
    }
}

/// Real parsing, resolver/default expansion, SCC assembly and complete companions reject every reservation failure.
#[test]
fn security_composition_compiler_allocation_sweep_is_atomic() {
    let mut case = source_case(include_str!("source-defaults.neu"));
    case["capture"]["sources"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "module":"other", "source_id":"source:other",
            "source_utf8":"neu \"1.0\"\nmodule other\nimport example as e\npublic num value = 1\n"
        }));
    let captured = capture_composition_project(request(&case)).unwrap();
    let cancel = CancellationToken::new();
    sweep("compiler", |failure| {
        observe(failure, || compile_composition_project(&captured, &cancel))
    });
    let mut cases = vec![
        source_case(include_str!("reference-cycle.neu")),
        leaf_case(),
    ];
    cases[1]["capture"]["sources"][0]["source_utf8"] = json!(
        "neu \"1.0\"\nmodule example\nuse Leaf as first\nuse Leaf as second\npublic first::Outer item = { inner: { value: 1 } }\n"
    );
    let fixtures: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    cases.extend(
        fixtures["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|case| {
                case["project_outcome"] == "accepted"
                    && !case["capture"]["sources"].as_array().unwrap().is_empty()
            })
            .cloned(),
    );
    for case in cases {
        let captured = capture_composition_project(request(&case)).unwrap();
        sweep("compiler-family", |failure| {
            observe(failure, || compile_composition_project(&captured, &cancel))
        });
    }
}

/// Independent complete validation, view closure, identities, wire encoding/decoding and probe fail atomically.
#[test]
fn security_composition_consumers_allocation_sweep_is_atomic() {
    let case = source_case(include_str!("source-defaults.neu"));
    sweep_consumers(&case);
}

/// Reference cycles and vocabulary defaults/constraints exercise additional consumer retention branches.
#[test]
fn security_composition_reference_and_vocabulary_consumer_allocation_sweeps() {
    sweep_consumers(&source_case(include_str!("reference-cycle.neu")));
    let fixtures: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    sweep_consumers(&fixtures["cases"][0]);
}

/// Sweeps complete consumer operations after fixture construction has finished outside observation.
fn sweep_consumers(case: &Value) {
    let project = compile(case);
    let ir = project.complete_ir();
    let cancel = CancellationToken::new();
    let composition = limits_for(case);
    sweep("reader", |failure| {
        observe(failure, || {
            ValidatedCompositionProject::from_ir(Arc::clone(ir), ir.limits, composition, &cancel)
        })
    });
    let selection = CompositionViewRequest {
        schema: profile::PROJECT_VIEW_SCHEMA.to_owned(),
        roots: ir
            .declarations
            .iter()
            .filter(|d| d.public)
            .map(|d| d.identity.clone())
            .collect(),
    };
    sweep("view", |failure| {
        observe(failure, || project.derive_view(&selection, &cancel))
    });
    sweep("identity", |failure| {
        observe(failure, || {
            canonical_composition_project(
                ir,
                IdentityLimits {
                    bytes: 1_048_576,
                    nodes: 65_536,
                },
                &cancel,
            )
        })
    });
    sweep("encode", |failure| {
        observe(failure, || encode_composition_project(&project, &cancel))
    });
    let bytes = encode_composition_project(&project, &cancel).unwrap();
    sweep("decode", |failure| {
        observe(failure, || {
            decode_composition_project(
                &bytes,
                DecodeLimits::hard(),
                ir.limits,
                composition,
                &cancel,
            )
        })
    });
    let roots: Vec<_> = selection
        .roots
        .iter()
        .map(|owner| {
            format!(
                "{}::{}",
                owner.module().module_name(),
                owner.declaration_name()
            )
        })
        .collect();
    for roots in [None, Some(roots)] {
        sweep("probe", |failure| {
            observe(failure, || {
                inspect_composition_encoded(
                    &bytes,
                    DecodeLimits::hard(),
                    ir.limits,
                    composition,
                    roots.as_deref(),
                    &cancel,
                )
            })
        });
    }
}

/// Real token cancellation at every compiler/consumer reservation rejects final publication.
#[test]
fn security_composition_pipeline_reservation_cancellation_is_atomic() {
    let case = source_case(include_str!("source-defaults.neu"));
    let captured = capture_composition_project(request(&case)).unwrap();
    let project = compile(&case);
    let ir = project.complete_ir();
    let policy = limits_for(&case);
    let bytes = encode_composition_project(&project, &CancellationToken::new()).unwrap();
    let selection = CompositionViewRequest {
        schema: profile::PROJECT_VIEW_SCHEMA.to_owned(),
        roots: vec![
            ir.declarations
                .iter()
                .find(|d| d.identity.declaration_name() == "states")
                .unwrap()
                .identity
                .clone(),
        ],
    };
    let operations: [CancellableOperation<'_>; 7] = [
        ("compile", &|signal| {
            compile_composition_project(&captured, signal).is_ok()
        }),
        ("reader", &|signal| {
            ValidatedCompositionProject::from_ir(Arc::clone(ir), ir.limits, policy, signal).is_ok()
        }),
        ("view", &|signal| {
            project.derive_view(&selection, signal).is_ok()
        }),
        ("identity", &|signal| {
            canonical_composition_project(
                ir,
                IdentityLimits {
                    bytes: 1_048_576,
                    nodes: 65_536,
                },
                signal,
            )
            .is_ok()
        }),
        ("encode", &|signal| {
            encode_composition_project(&project, signal).is_ok()
        }),
        ("decode", &|signal| {
            decode_composition_project(&bytes, DecodeLimits::hard(), ir.limits, policy, signal)
                .is_ok()
        }),
        ("probe", &|signal| {
            inspect_composition_encoded(
                &bytes,
                DecodeLimits::hard(),
                ir.limits,
                policy,
                None,
                signal,
            )
            .is_ok()
        }),
    ];
    for (label, operation) in operations {
        let signal = CancellationToken::new();
        let (accepted, count) = observe(None, || operation(&signal));
        assert!(accepted);
        for index in 0..count {
            let signal = CancellationToken::new();
            let (accepted, _) = observe_cancellation(index, &signal, || operation(&signal));
            assert!(signal.is_cancelled(), "{label}: missed reservation {index}");
            assert!(
                !accepted,
                "{label}: published after cancellation at {index}/{count}"
            );
        }
    }
}

/// Failed changed-unit generations preserve warm syntax, byte accounting and the prior complete result.
#[test]
fn security_composition_cache_allocation_sweep_preserves_successful_generation() {
    let original_case = source_case("neu \"1.0\"\nmodule example\npublic num value = 1\n");
    let changed_case = source_case(
        "neu \"1.0\"\nmodule example\npublic record Item { num value = 2 }\npublic Item item = {}\n",
    );
    let original = capture_composition_project(request(&original_case)).unwrap();
    let changed = capture_composition_project(request(&changed_case)).unwrap();
    let cancel = CancellationToken::new();
    let make = || {
        CompositionCompilationCache::new(ProjectCacheLimits {
            source_units: 64,
            source_bytes: 65_536,
        })
        .unwrap()
    };
    for target in [&original, &changed] {
        let mut baseline = make();
        baseline.compile(&original, &cancel).unwrap();
        let (expected, count) = observe(None, || baseline.compile(target, &cancel));
        let expected = expected.unwrap().0;
        assert_ne!(count, 0);
        for index in 0..count {
            let mut cache = make();
            let prior = cache.compile(&original, &cancel).unwrap().0;
            let retained = (cache.retained_units(), cache.retained_source_bytes());
            let (failed, _) = observe(Some(index), || cache.compile(target, &cancel));
            assert!(failed.is_err(), "cache ignored failure {index}/{count}");
            assert_eq!(
                (cache.retained_units(), cache.retained_source_bytes()),
                retained
            );
            let (recovered, stats) = cache.compile(&original, &cancel).unwrap();
            assert_eq!(stats.reused_units, 1);
            assert_eq!(recovered, prior);
            assert_eq!(cache.compile(target, &cancel).unwrap().0, expected);
        }
        eprintln!(
            "[TEST] cache: {count} reservation failures rejected; prior generation preserved"
        );
    }
}
