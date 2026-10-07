// SPDX-License-Identifier: Apache-2.0

//! Successor real-cache execution, isolated failures and independent consumer boundary sweeps.

use super::*;
use neutral_compiler::{CompositionCompilationCache, ProjectCacheLimits};
use neutral_ir::composition::project::CompositionPolicy;

/// Creates finite explicit syntax retention controls, independent of semantic controls.
fn cache() -> CompositionCompilationCache {
    CompositionCompilationCache::new(ProjectCacheLimits {
        source_units: 64,
        source_bytes: 65_536,
    })
    .unwrap()
}
/// Checks full producer companions and independent wire output, not just logical equality.
fn compare(
    cache: &mut CompositionCompilationCache,
    captured: &neutral_compiler::CapturedCompositionProject,
) -> neutral_compiler::ProjectCacheStats {
    let cancel = CancellationToken::new();
    let clean = compile_composition_project(captured, &cancel).unwrap();
    let (incremental, stats) = cache.compile(captured, &cancel).unwrap();
    assert_eq!(clean, incremental);
    let open = |ir: Arc<neutral_ir::composition::project::CompositionProjectIr>| {
        ValidatedCompositionProject::from_ir(
            Arc::clone(&ir),
            ir.limits,
            captured.composition_limits(),
            &cancel,
        )
        .unwrap()
    };
    assert_eq!(
        encode_composition_project(&open(clean), &cancel).unwrap(),
        encode_composition_project(&open(incremental), &cancel).unwrap()
    );
    stats
}

/// Exact syntax hits rebuild host facts; changed bytes/units and failed generations cannot contaminate results.
#[test]
fn integration_composition_real_cache_matches_clean_changes_and_failures() {
    let text = include_str!("source-defaults.neu");
    let mut case = source_case(text);
    case["capture"]["sources"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "module":"unused", "source_id":"source:unused",
            "source_utf8":"neu \"1.0\"\nmodule unused\nnum value = 1\n"
        }));
    let mut cache = cache();
    let captured = capture_composition_project(request(&case)).unwrap();
    assert_eq!(compare(&mut cache, &captured).parsed_units, 2);
    let warm = compare(&mut cache, &captured);
    assert_eq!((warm.parsed_units, warm.reused_units), (0, 2));
    case["capture"]["sources"][0]["source_id"] = json!("changed-host-id");
    let remapped = capture_composition_project(request(&case)).unwrap();
    assert_eq!(compare(&mut cache, &remapped).reused_units, 2);
    case["capture"]["sources"][0]["source_utf8"] = json!(text.replace("42", "43"));
    let changed = capture_composition_project(request(&case)).unwrap();
    let stats = compare(&mut cache, &changed);
    assert_eq!(
        (
            stats.parsed_units,
            stats.reused_units,
            stats.rejected_entries
        ),
        (1, 1, 1)
    );
    let before = cache.retained_source_bytes();
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert!(cache.compile(&changed, &cancel).is_err());
    assert_eq!(cache.retained_source_bytes(), before);
    let bad = capture_composition_project(request(&source_case(
        "neu \"1.0\"\nmodule example\npublic num wrong = false\n",
    )))
    .unwrap();
    assert_eq!(
        cache.compile(&bad, &CancellationToken::new()).unwrap_err(),
        compile_composition_project(&bad, &CancellationToken::new()).unwrap_err()
    );
    assert_eq!(cache.retained_source_bytes(), before);
    assert_eq!(compare(&mut cache, &changed).reused_units, 2);
    case["capture"]["sources"].as_array_mut().unwrap().pop();
    let removed = capture_composition_project(request(&case)).unwrap();
    assert_eq!(compare(&mut cache, &removed).reused_units, 1);
    assert_eq!(cache.retained_units(), 1);
    let mut controls = limits_for(&case);
    controls.work = 64;
    let lower = capture_composition_project(request_with_limits(&case, controls)).unwrap();
    assert_eq!(
        cache
            .compile(&lower, &CancellationToken::new())
            .unwrap_err(),
        compile_composition_project(&lower, &CancellationToken::new()).unwrap_err()
    );
    assert_eq!(compare(&mut cache, &removed).reused_units, 1);
}

/// Successful syntax hits must still resolve changed vocabulary contracts and restrictions.
#[test]
fn integration_composition_cache_revalidates_catalogues_and_concurrent_requests() {
    let requests: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let cases = requests["cases"].as_array().unwrap().iter().filter(|case| {
        case["project_outcome"] == "accepted"
            && !case["capture"]["sources"].as_array().unwrap().is_empty()
    });
    let mut cache = cache();
    for case in cases {
        let captured = capture_composition_project(request(case)).unwrap();
        compare(&mut cache, &captured);
        let stats = compare(&mut cache, &captured);
        assert_eq!(stats.parsed_units, 0);
        assert_eq!(stats.reused_units, captured.sources().len() as u64);
        std::thread::scope(|scope| {
            for _ in 0..4 {
                scope.spawn(|| {
                    let mut independent = self::cache();
                    compare(&mut independent, &captured);
                    assert_eq!(compare(&mut independent, &captured).parsed_units, 0);
                });
            }
        });
    }
    assert!(
        CompositionCompilationCache::new(ProjectCacheLimits {
            source_units: 0,
            source_bytes: 1,
        })
        .is_none()
    );
    let captured =
        capture_composition_project(request(&source_case(include_str!("source-defaults.neu"))))
            .unwrap();
    let mut tiny = CompositionCompilationCache::new(ProjectCacheLimits {
        source_units: 1,
        source_bytes: 1,
    })
    .unwrap();
    assert_eq!(compare(&mut tiny, &captured).parsed_units, 1);
    assert_eq!(tiny.retained_units(), 0);
    assert_eq!(compare(&mut tiny, &captured).parsed_units, 1);
}

/// Identical source must materialize changed locked defaults again, even when every syntax entry hits.
#[test]
fn integration_composition_cache_never_reuses_stale_vocabulary_defaults() {
    let requests: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let mut case = requests["cases"][0].clone();
    let mut cache = cache();
    let captured = capture_composition_project(request(&case)).unwrap();
    let original = compile_composition_project(&captured, &CancellationToken::new()).unwrap();
    compare(&mut cache, &captured);
    let input = &mut case["capture"]["vocabularies"][0];
    let mut bundle: Value = serde_json::from_str(input["bundle_utf8"].as_str().unwrap()).unwrap();
    bundle["types"][1]["fields"][0]["default"]["value"] = json!("1");
    let bytes = serde_json::to_string_pretty(&bundle).unwrap();
    input["digest"] = json!(
        VocabularyContentDigest::from_bytes(bytes.as_bytes())
            .to_string()
            .strip_prefix("sha256:")
            .unwrap()
    );
    input["bundle_utf8"] = json!(bytes);
    let changed = capture_composition_project(request(&case)).unwrap();
    let stats = compare(&mut cache, &changed);
    assert_eq!(stats.parsed_units, 0);
    assert_eq!(stats.reused_units, captured.sources().len() as u64);
    let revised = compile_composition_project(&changed, &CancellationToken::new()).unwrap();
    assert_ne!(original.declarations, revised.declarations);
    assert_ne!(original.interface_digest, revised.interface_digest);
}

/// Independent CBOR ceilings each reject immediately below their accepted threshold.
#[test]
fn security_composition_every_wire_control_has_an_exact_acceptance_boundary() {
    let case = source_case(include_str!("source-defaults.neu"));
    let project = compile(&case);
    let ir = project.complete_ir();
    let cancel = CancellationToken::new();
    let bytes = encode_composition_project(&project, &cancel).unwrap();
    let controls: [fn(DecodeLimits, usize) -> DecodeLimits; 7] = [
        DecodeLimits::with_artifact_bytes,
        DecodeLimits::with_section_bytes,
        DecodeLimits::with_nesting_depth,
        DecodeLimits::with_container_items,
        DecodeLimits::with_text_bytes,
        DecodeLimits::with_byte_string_bytes,
        DecodeLimits::with_traversal_nodes,
    ];
    for (index, control) in controls.into_iter().enumerate() {
        let accepts = |limit| {
            decode_composition_project(
                &bytes,
                control(DecodeLimits::hard(), limit),
                ir.limits,
                limits_for(&case),
                &cancel,
            )
            .is_ok()
        };
        let mut low = 0;
        let mut high = bytes.len();
        assert!(accepts(high), "wire control {index}");
        while low < high {
            let mid = low + (high - low) / 2;
            if accepts(mid) {
                high = mid;
            } else {
                low = mid + 1;
            }
        }
        assert!(accepts(low));
        assert!(low > 0);
        assert!(!accepts(low - 1), "wire control {index}");
        assert!(accepts(low + 1));
    }
}

/// Sweeps each independent successor policy through both raw reader and hostile wire boundaries.
#[test]
fn security_composition_every_consumer_policy_has_an_exact_acceptance_boundary() {
    let requests: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let case = &requests["cases"][0];
    let project = compile(case);
    let ir = project.complete_ir();
    let cancel = CancellationToken::new();
    let bytes = encode_composition_project(&project, &cancel).unwrap();
    let policy = limits_for(case);
    for index in 0..policy.policy().values().len() {
        let accepts = |limit| {
            let mut values = policy.policy().values();
            values[index] = limit;
            let limits =
                CompositionLimits::from_policy(policy.json, CompositionPolicy::from_values(values));
            let read =
                ValidatedCompositionProject::from_ir(Arc::clone(ir), ir.limits, limits, &cancel)
                    .is_ok();
            let decoded = decode_composition_project(
                &bytes,
                DecodeLimits::hard(),
                ir.limits,
                limits,
                &cancel,
            )
            .is_ok();
            assert_eq!(read, decoded, "policy position {index}, budget {limit}");
            read
        };
        let mut low = 1;
        let mut high = policy.policy().values()[index];
        assert!(accepts(high));
        while low < high {
            let mid = low + (high - low) / 2;
            if accepts(mid) {
                high = mid;
            } else {
                low = mid + 1;
            }
        }
        assert!(accepts(low));
        assert!(!accepts(low - 1));
        assert!(accepts(low.saturating_add(1)));
    }
}

/// Every byte position is mutated under complete decoding; any accepted result must independently revalidate.
#[test]
fn security_composition_mutated_artifacts_never_bypass_independent_validation() {
    let case = source_case(include_str!("source-defaults.neu"));
    let project = compile(&case);
    let ir = project.complete_ir();
    let cancel = CancellationToken::new();
    let bytes = encode_composition_project(&project, &cancel).unwrap();
    for index in 0..bytes.len() {
        for mask in [1, 128, 255] {
            let mut changed = bytes.clone();
            changed[index] ^= mask;
            if let Ok(decoded) = decode_composition_project(
                &changed,
                DecodeLimits::hard(),
                ir.limits,
                limits_for(&case),
                &cancel,
            ) {
                ValidatedCompositionProject::from_ir(
                    Arc::clone(decoded.complete_ir()),
                    ir.limits,
                    limits_for(&case),
                    &cancel,
                )
                .unwrap();
                encode_composition_project(&decoded, &cancel).unwrap();
            }
        }
    }
}

/// Producer policy cannot advertise weaker acceptance controls and still gain reader authority.
#[test]
fn security_composition_every_producer_policy_has_an_exact_acceptance_boundary() {
    let requests: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let case = &requests["cases"][0];
    let project = compile(case);
    let ir = project.complete_ir();
    let cancel = CancellationToken::new();
    let controls = limits_for(case);
    for index in 0..ir.composition_limits.values().len() {
        let accepts = |budget| {
            let mut claimed = (**ir).clone();
            let mut values = claimed.composition_limits.values();
            values[index] = budget;
            claimed.composition_limits = CompositionPolicy::from_values(values);
            if let Ok(checked) =
                ValidatedCompositionProject::from_ir(shared(claimed), ir.limits, controls, &cancel)
            {
                let bytes = encode_composition_project(&checked, &cancel).unwrap();
                decode_composition_project(
                    &bytes,
                    DecodeLimits::hard(),
                    ir.limits,
                    controls,
                    &cancel,
                )
                .unwrap();
                true
            } else {
                false
            }
        };
        let mut low = 1;
        let mut high = ir.composition_limits.values()[index];
        assert!(accepts(high));
        while low < high {
            let mid = low + (high - low) / 2;
            if accepts(mid) {
                high = mid;
            } else {
                low = mid + 1;
            }
        }
        assert!(accepts(low));
        assert!(!accepts(low - 1), "producer control {index}");
        assert!(accepts(low + 1));
    }
}

/// Narrower shared scalar/shape control projected into strict composition JSON policy.
type StructuralControl =
    fn(StructuralLimits, u64) -> Result<StructuralLimits, neutral_core::CoreError>;
/// Independent JSON-specific override with positive finite policy requirements.
type JsonControl =
    fn(VocabularyLimits, u64) -> Result<VocabularyLimits, neutral_vocabulary::VocabularyError>;

/// Confirms the strict JSON policy boundary without changing unrelated composition controls.
fn json_boundary(case: &Value, label: &str, policy: impl Fn(u64) -> Option<VocabularyLimits>) {
    let limits = limits_for(case);
    let accepts = |budget| {
        policy(budget).is_some_and(|json| {
            capture_composition_project(request_with_limits(
                case,
                CompositionLimits { json, ..limits },
            ))
            .is_ok()
        })
    };
    let mut low = 1;
    let mut high = 65_536;
    assert!(accepts(high), "JSON {label}");
    while low < high {
        let mid = low + (high - low) / 2;
        if accepts(mid) {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    assert!(accepts(low));
    assert!(!accepts(low - 1), "JSON {label}");
    assert!(accepts(low + 1));
}

/// Every strict JSON/scalar control rejects immediately below its acceptance boundary.
#[test]
fn security_composition_json_scalar_and_shape_boundaries_are_independent() {
    let requests: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let case = &requests["cases"][0];
    let structural = StructuralLimits::new(65_536, 64).unwrap();
    let scalar: &[(&str, StructuralControl)] = &[
        ("string", StructuralLimits::with_string_bytes),
        ("digits", StructuralLimits::with_numeric_digits),
        ("scale", StructuralLimits::with_numeric_scale),
        ("declarations", StructuralLimits::with_declarations),
        ("fields", StructuralLimits::with_record_fields),
        ("depth", StructuralLimits::with_nesting_depth),
        ("items", StructuralLimits::with_list_items),
        ("nodes", StructuralLimits::with_traversal_nodes),
    ];
    for (label, control) in scalar {
        json_boundary(case, label, |budget| {
            control(structural, budget)
                .ok()
                .map(VocabularyLimits::from_structural)
                .and_then(|json| {
                    json.with_object_members(neutral_vocabulary::composition::MAX_OBJECT_MEMBERS)
                        .ok()
                })
        });
    }
    let json: &[(&str, JsonControl)] = &[
        ("bytes", VocabularyLimits::with_bundle_bytes),
        ("members", VocabularyLimits::with_object_members),
        ("array", VocabularyLimits::with_array_items),
        ("types", VocabularyLimits::with_types),
        ("features", VocabularyLimits::with_features),
    ];
    for (label, control) in json {
        json_boundary(case, label, |budget| {
            control(limits_for(case).json, budget).ok()
        });
    }
}
