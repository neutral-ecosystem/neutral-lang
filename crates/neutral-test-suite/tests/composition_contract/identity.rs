// SPDX-License-Identifier: Apache-2.0

//! Production identity partitions compared with immutable independent transcripts.

use super::*;
use neutral_ir::composition::project::CompositionPolicy;
use neutral_ir::project_identity::{
    ArtifactIdentityInput, ArtifactKind, CompositionDerivationContext, IdentityLimits,
    canonical_composition_project, composition_artifact_identity, composition_derivation_identity,
};
#[path = "identity_facts.rs"]
mod facts;

/// Uses the exact literal transcript limits, independently of production counting.
fn exact_limits(expected: &Value) -> IdentityLimits {
    IdentityLimits {
        bytes: expected["transcript_bytes"].as_u64().unwrap(),
        nodes: expected["frames"].as_u64().unwrap(),
    }
}
/// Exercises byte and frame rejection independently, including invalid zero controls.
fn insufficient_limits(exact: IdentityLimits) -> [IdentityLimits; 4] {
    [
        IdentityLimits {
            bytes: exact.bytes - 1,
            ..exact
        },
        IdentityLimits {
            nodes: exact.nodes - 1,
            ..exact
        },
        IdentityLimits { bytes: 0, ..exact },
        IdentityLimits { nodes: 0, ..exact },
    ]
}
/// Reads frozen root tuples without changing their selection or ordering.
fn roots(artifact: &Value) -> Vec<ModuleSymbolIdentity> {
    artifact["roots"]
        .as_array()
        .unwrap()
        .iter()
        .map(facts::symbol)
        .collect()
}
/// Reads explicit frozen format/kind selectors; these vector artifacts have no format options.
fn artifact_input<'a>(
    artifact: &'a Value,
    roots: &'a [ModuleSymbolIdentity],
) -> ArtifactIdentityInput<'a> {
    assert_eq!(artifact["options"].as_array().unwrap().len(), 0);
    ArtifactIdentityInput {
        kind: if artifact["kind"] == "project" {
            ArtifactKind::Project
        } else {
            ArtifactKind::View
        },
        format: artifact["format"].as_str().unwrap(),
        roots,
        options: &[],
    }
}
/// Reopens complete bytes independently under the same explicit caller controls.
fn roundtrip(project: &ValidatedCompositionProject, case: &Value) -> ValidatedCompositionProject {
    let cancel = CancellationToken::new();
    let encoded = encode_composition_project(project, &cancel).unwrap();
    decode_composition_project(
        &encoded,
        DecodeLimits::hard(),
        project.complete_ir().limits,
        limits_for(case),
        &cancel,
    )
    .unwrap()
}

/// Equivalent explicit/default values share meaning while omission/null and occurrence evidence differ.
#[test]
fn property_composition_source_default_omission_and_null_identity_partitions() {
    use neutral_ir::composition::{ValueOriginKind, ValuePathSegment};
    let fixtures: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let base_case = &fixtures["cases"][0];
    let base = compile(base_case);
    let cancel = CancellationToken::new();
    let policy = IdentityLimits {
        bytes: 1_048_576,
        nodes: 65_536,
    };
    let logical = canonical_composition_project(base.complete_ir(), policy, &cancel).unwrap();
    let mut explicit = base_case.clone();
    explicit["capture"]["sources"][0]["source_utf8"] = json!(base_case["capture"]["sources"][0]["source_utf8"].as_str().unwrap().replace("{ selected: ref(choice) }", "{ attempts: 3, outcomes: [{ tag: \"success\", payload: \"complete\" }], selected: ref(choice) }"));
    let supplied = compile(&explicit);
    assert_eq!(
        canonical_composition_project(supplied.complete_ir(), policy, &cancel).unwrap(),
        logical
    );
    assert_eq!(
        neutral_ir::project_identity::composition_interface(
            supplied.complete_ir(),
            policy,
            &cancel
        )
        .unwrap(),
        neutral_ir::project_identity::composition_interface(base.complete_ir(), policy, &cancel)
            .unwrap()
    );
    let path = [ValuePathSegment::Field("attempts".to_owned())];
    let origin = |project: &ValidatedCompositionProject| {
        project
            .complete_ir()
            .origins
            .iter()
            .find(|o| o.binding.declaration_name() == "request" && o.path == path)
            .unwrap()
            .clone()
    };
    assert_eq!(origin(&base).kind, ValueOriginKind::Defaulted);
    assert_eq!(origin(&supplied).kind, ValueOriginKind::Supplied);
    assert_ne!(origin(&base).attribution, origin(&supplied).attribution);
    let first = capture_composition_project(request(base_case)).unwrap();
    let second = capture_composition_project(request(&explicit)).unwrap();
    assert_ne!(
        first.identity_transcript(policy, &cancel).unwrap(),
        second.identity_transcript(policy, &cancel).unwrap()
    );
    for project in [&base, &supplied] {
        let decoded = roundtrip(project, base_case);
        assert_eq!(decoded.complete_ir().origins, project.complete_ir().origins);
    }
    let mut null = base_case.clone();
    null["capture"]["sources"][0]["source_utf8"] = json!(
        base_case["capture"]["sources"][0]["source_utf8"]
            .as_str()
            .unwrap()
            .replace(
                "{ selected: ref(choice) }",
                "{ label: null, selected: ref(choice) }"
            )
    );
    let null = compile(&null);
    assert_ne!(
        canonical_composition_project(null.complete_ir(), policy, &cancel).unwrap(),
        logical
    );
    let label = |project: &ValidatedCompositionProject| {
        project
            .complete_ir()
            .origins
            .iter()
            .find(|o| {
                o.binding.declaration_name() == "request"
                    && o.path == [ValuePathSegment::Field("label".to_owned())]
            })
            .unwrap()
            .kind
    };
    assert_eq!(label(&base), ValueOriginKind::OmittedOptional);
    assert_eq!(label(&null), ValueOriginKind::ExplicitNull);
    let mut changed_default = explicit;
    let mut bundle: Value = serde_json::from_str(
        changed_default["capture"]["vocabularies"][0]["bundle_utf8"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    bundle["types"][1]["fields"][0]["default"]["value"] = json!("1");
    super::graphs::replace_bundle(&mut changed_default["capture"]["vocabularies"][0], &bundle);
    let changed_default = compile(&changed_default);
    assert_eq!(
        changed_default.complete_ir().declarations,
        supplied.complete_ir().declarations
    );
    assert_ne!(
        canonical_composition_project(changed_default.complete_ir(), policy, &cancel).unwrap(),
        logical
    );
}

/// Compares every registered semantic variation, including exclusions, with literal frozen bytes.
#[test]
fn conformance_composition_all_logical_variations_match_frozen_vectors() {
    let data = vectors();
    let fixture: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let base = compile(&fixture["cases"][0]);
    let capture = capture_composition_project(request(&fixture["cases"][0])).unwrap();
    let policy = IdentityLimits {
        bytes: 1_048_576,
        nodes: 65_536,
    };
    let captured = capture
        .identity_transcript(policy, &CancellationToken::new())
        .unwrap();
    for case in data["logical_variants"].as_array().unwrap() {
        let request = variant(&data["request"], &case["edits"]);
        let ir = facts::project(base.complete_ir(), &request["logical"]);
        let expected = &case["expected"];
        let exact = exact_limits(expected);
        let actual = canonical_composition_project(&ir, exact, &CancellationToken::new())
            .unwrap_or_else(|e| panic!("{}: {e:?}", case["id"]));
        assert_eq!(
            actual.bytes(),
            bytes(&expected["transcript_hex"]),
            "{}",
            case["id"]
        );
        let mut interface_request = request.clone();
        interface_request["operation"] = json!("interface");
        interface_request["limits"] = json!({"bytes":1_048_576,"nodes":65_536});
        let expected_interface = inspect_interface(&interface_request);
        let actual_interface = neutral_ir::project_identity::composition_interface(
            &ir,
            policy,
            &CancellationToken::new(),
        )
        .unwrap();
        assert_eq!(
            actual_interface.bytes(),
            bytes(&expected_interface["transcript_hex"]),
            "{}: independent interface",
            case["id"]
        );
        assert_eq!(
            actual_interface.identity().to_string(),
            expected_interface["sha256"].as_str().unwrap()
        );
        assert_eq!(
            actual.identity().to_string(),
            expected["sha256"].as_str().unwrap(),
            "{}",
            case["id"]
        );
        let partitions = inspect(&request);
        let derivation = composition_derivation_identity(
            actual.identity(),
            &derivation_context(captured.identity(), &request),
            policy,
            &CancellationToken::new(),
        )
        .unwrap();
        assert_eq!(
            derivation.bytes(),
            bytes(&partitions["derivation"]["transcript_hex"]),
            "{}: independent derivation",
            case["id"]
        );
        for artifact in request["artifacts"].as_array().unwrap() {
            let roots = roots(artifact);
            let input = artifact_input(artifact, &roots);
            let actual = composition_artifact_identity(
                derivation.identity(),
                &input,
                policy,
                &CancellationToken::new(),
            )
            .unwrap();
            assert_eq!(
                actual.bytes(),
                bytes(&partitions[artifact["layer"].as_str().unwrap()]["transcript_hex"]),
                "{}: independent artifact",
                case["id"]
            );
        }
        for limited in [
            IdentityLimits {
                bytes: exact.bytes - 1,
                ..exact
            },
            IdentityLimits {
                nodes: exact.nodes - 1,
                ..exact
            },
        ] {
            assert_eq!(
                canonical_composition_project(&ir, limited, &CancellationToken::new()).unwrap_err(),
                neutral_ir::project_identity::IdentityError::Limit
            );
        }
    }
}
use neutral_ir::project_identity::{
    CapturedIdentityInput, CapturedIdentitySource, CapturedIdentityVocabulary, IdentityError,
};
use neutral_ir::{LogicalModuleIdentity, ModuleSymbolIdentity};
use neutral_reader::composition::{CompositionIdentityContext, CompositionIdentityReadError};

/// Projects exact captured source facts; test setup is outside observed consumer allocations.
fn source_facts(
    capture: &neutral_compiler::CapturedCompositionProject,
) -> Vec<CapturedIdentitySource<'_>> {
    capture
        .sources()
        .iter()
        .map(|s| CapturedIdentitySource {
            module: s.module_id(),
            source_id: s.source_id(),
            digest: s.digest(),
            byte_len: s.bytes().len() as u64,
        })
        .collect()
}
/// Projects exact locks without inferring selectors from complete logical meaning.
fn vocabulary_facts(
    capture: &neutral_compiler::CapturedCompositionProject,
) -> Vec<CapturedIdentityVocabulary<'_>> {
    capture
        .vocabularies()
        .iter()
        .map(|v| {
            let l = v.lock();
            CapturedIdentityVocabulary {
                identity: l.identity(),
                version: l.version(),
                encoding_version: l.encoding_version(),
                schema_version: l.schema_version(),
                digest: l.content_digest(),
                byte_len: v.bytes().len() as u64,
                required_features: l.required_features(),
            }
        })
        .collect()
}

/// Independently decoded identities bind all companions; selections cannot change upstream partitions.
#[test]
fn integration_composition_reader_identities_match_oracle_and_preserve_roots() {
    let data = vectors();
    let fixture: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let captured = capture_composition_project(request(&fixture["cases"][0])).unwrap();
    let original = compile(&fixture["cases"][0]);
    let cancel = CancellationToken::new();
    let encoded = encode_composition_project(&original, &cancel).unwrap();
    let reader = decode_composition_project(
        &encoded,
        DecodeLimits::hard(),
        original.complete_ir().limits,
        limits_for(&fixture["cases"][0]),
        &cancel,
    )
    .unwrap();
    let sources = source_facts(&captured);
    let mut vocabularies = vocabulary_facts(&captured);
    let mut context = CompositionIdentityContext {
        capture: CapturedIdentityInput {
            profile: neutral_core::profile::V1_SOURCE_PROFILE,
            sources: &sources,
            vocabularies: &vocabularies,
        },
        required_features: captured.required_features(),
        producer: data["request"]["producer"].as_str().unwrap(),
        producer_version: data["request"]["producer_version"].as_str().unwrap(),
        capture_limits: captured.identity_capture_limits(),
    };
    let policy = IdentityLimits {
        bytes: 1_048_576,
        nodes: 65_536,
    };
    let identities = reader.identities(&context, policy, &cancel).unwrap();
    let mut expected_input = data["request"].clone();
    let p = reader.complete_ir().limits;
    expected_input["capture_limits"] = json!(context.capture_limits);
    expected_input["project_limits"] = json!([
        p.modules,
        p.declarations,
        p.import_edges,
        p.nodes,
        p.text_bytes,
        p.artifact_bytes
    ]);
    expected_input["composition_limits"] = json!(reader.complete_ir().composition_limits.values());
    let expected = inspect(&expected_input);
    compare_reader_partitions(&reader, &identities, &expected);
    compare_view_roots(
        &reader,
        &context,
        &identities,
        &expected_input,
        policy,
        &cancel,
    );
    let empty = ArtifactIdentityInput {
        kind: ArtifactKind::Project,
        format: profile::ENCODING,
        roots: &[],
        options: &[],
    };
    assert_eq!(
        identities
            .artifact(&empty, policy, &cancel)
            .unwrap()
            .bytes(),
        bytes(&expected["artifact-project"]["transcript_hex"])
    );
    context.producer = "another-explicit-producer";
    let changed = reader.identities(&context, policy, &cancel).unwrap();
    assert_eq!(changed.captured(), identities.captured());
    assert_eq!(changed.logical(), identities.logical());
    assert_ne!(changed.derivation(), identities.derivation());
    let mut wrong_sources = source_facts(&captured);
    wrong_sources[0].source_id = "source:unrelated";
    context.capture.sources = &wrong_sources;
    assert_eq!(
        reader.identities(&context, policy, &cancel).unwrap_err(),
        CompositionIdentityReadError::CaptureMismatch
    );
    vocabularies[0].schema_version = "unrelated-selector";
    let context = CompositionIdentityContext {
        capture: CapturedIdentityInput {
            profile: neutral_core::profile::V1_SOURCE_PROFILE,
            sources: &sources,
            vocabularies: &vocabularies,
        },
        required_features: captured.required_features(),
        producer: "test",
        producer_version: "explicit",
        capture_limits: captured.identity_capture_limits(),
    };
    assert_eq!(
        reader.identities(&context, policy, &cancel).unwrap_err(),
        CompositionIdentityReadError::CaptureMismatch
    );
}

/// Checks independent partition bytes and the authoritative retained reader policy.
fn compare_reader_partitions(
    reader: &ValidatedCompositionProject,
    identities: &neutral_reader::composition::CompositionIdentities<'_, '_>,
    expected: &Value,
) {
    assert_eq!(
        identities.captured().bytes(),
        bytes(&expected["captured"]["transcript_hex"])
    );
    assert_eq!(
        identities.logical().bytes(),
        bytes(&expected["logical"]["transcript_hex"])
    );
    assert_eq!(
        identities.derivation().bytes(),
        bytes(&expected["derivation"]["transcript_hex"])
    );
    assert_eq!(
        identities.interface().identity(),
        reader.complete_ir().interface_digest
    );
    assert_eq!(
        identities.facts().composition_limits,
        reader.complete_ir().composition_limits
    );
}

/// Artifact identities cannot bypass independent public-root authority, even with well-framed tuples.
#[test]
fn security_composition_artifact_identity_rejects_invalid_public_roots() {
    use neutral_reader::composition::CompositionReadError;
    let case =
        source_case("neu \"1.0\"\nmodule example\nnum secret = 7\npublic num visible = secret\n");
    let captured = capture_composition_project(request(&case)).unwrap();
    let reader = roundtrip(&compile(&case), &case);
    let sources = source_facts(&captured);
    let vocabularies = vocabulary_facts(&captured);
    let context = CompositionIdentityContext {
        capture: CapturedIdentityInput {
            profile: neutral_core::profile::V1_SOURCE_PROFILE,
            sources: &sources,
            vocabularies: &vocabularies,
        },
        required_features: captured.required_features(),
        producer: env!("CARGO_PKG_NAME"),
        producer_version: env!("CARGO_PKG_VERSION"),
        capture_limits: captured.identity_capture_limits(),
    };
    let policy = IdentityLimits {
        bytes: 1_048_576,
        nodes: 65_536,
    };
    let cancel = CancellationToken::new();
    let identities = reader.identities(&context, policy, &cancel).unwrap();
    for (profile, name) in [
        (neutral_core::profile::V1_SOURCE_PROFILE, "secret"),
        (neutral_core::profile::V1_SOURCE_PROFILE, "absent"),
        (neutral_core::profile::V0_SOURCE_PROFILE, "visible"),
    ] {
        let roots = [ModuleSymbolIdentity::new(
            LogicalModuleIdentity::new(profile, "example"),
            name,
        )];
        let input = ArtifactIdentityInput {
            kind: ArtifactKind::View,
            format: profile::PROJECT_VIEW_SCHEMA,
            roots: &roots,
            options: &[],
        };
        assert_eq!(
            identities.artifact(&input, policy, &cancel).unwrap_err(),
            CompositionIdentityReadError::View(CompositionReadError::Semantic)
        );
    }
    let root = ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(neutral_core::profile::V1_SOURCE_PROFILE, "example"),
        "visible",
    );
    let roots = [root.clone(), root];
    let duplicate = ArtifactIdentityInput {
        kind: ArtifactKind::View,
        format: profile::PROJECT_VIEW_SCHEMA,
        roots: &roots,
        options: &[],
    };
    assert_eq!(
        identities
            .artifact(&duplicate, policy, &cancel)
            .unwrap_err(),
        CompositionIdentityReadError::Identity(IdentityError::InvalidInput)
    );
}

/// Compares independent selected artifact transcripts while complete reader identities remain unchanged.
fn compare_view_roots(
    reader: &ValidatedCompositionProject,
    context: &CompositionIdentityContext<'_>,
    identities: &neutral_reader::composition::CompositionIdentities<'_, '_>,
    expected_input: &Value,
    policy: IdentityLimits,
    cancel: &CancellationToken,
) {
    for name in ["choice", "request", "states", "status"] {
        let roots = [ModuleSymbolIdentity::new(
            LogicalModuleIdentity::new(neutral_core::profile::V1_SOURCE_PROFILE, "example"),
            name,
        )];
        let input = ArtifactIdentityInput {
            kind: ArtifactKind::View,
            format: profile::PROJECT_VIEW_SCHEMA,
            roots: &roots,
            options: &[],
        };
        let actual = identities.artifact(&input, policy, cancel).unwrap();
        let mut req = expected_input.clone();
        req["operation"] = json!("artifact");
        req["selected_artifact"] = json!({"kind":"view","format":profile::PROJECT_VIEW_SCHEMA,"roots":[[neutral_core::profile::V1_SOURCE_PROFILE,"example",name]],"options":[]});
        req["upstream"] = json!({"derivation":identities.derivation().identity().to_string()});
        assert_eq!(actual.bytes(), bytes(&inspect(&req)["transcript_hex"]));
        assert_eq!(
            reader
                .identities(context, policy, cancel)
                .unwrap()
                .logical(),
            identities.logical()
        );
    }
}

/// Every successor identity/view retention boundary rejects allocation failure and cancellation atomically.
#[test]
fn security_composition_reader_identity_allocation_failures_and_recovery() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    let data = vectors();
    let fixture: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let captured = capture_composition_project(request(&fixture["cases"][0])).unwrap();
    let reader = compile(&fixture["cases"][0]);
    let sources = source_facts(&captured);
    let vocabularies = vocabulary_facts(&captured);
    let context = CompositionIdentityContext {
        capture: CapturedIdentityInput {
            profile: neutral_core::profile::V1_SOURCE_PROFILE,
            sources: &sources,
            vocabularies: &vocabularies,
        },
        required_features: captured.required_features(),
        producer: data["request"]["producer"].as_str().unwrap(),
        producer_version: data["request"]["producer_version"].as_str().unwrap(),
        capture_limits: captured.identity_capture_limits(),
    };
    let policy = IdentityLimits {
        bytes: 1_048_576,
        nodes: 65_536,
    };
    let cancel = CancellationToken::new();
    let (result, count) = observe(None, || reader.identities(&context, policy, &cancel));
    let original = result.unwrap();
    for index in 0..count {
        assert!(
            observe(Some(index), || reader.identities(&context, policy, &cancel))
                .0
                .is_err(),
            "identity allocation {index}/{count}"
        );
        let signal = CancellationToken::new();
        assert!(
            observe_cancellation(index, &signal, || reader
                .identities(&context, policy, &signal))
            .0
            .is_err(),
            "identity cancellation {index}/{count}"
        );
    }
    let roots = [facts::symbol(&data["request"]["artifacts"][1]["roots"][0])];
    let input = ArtifactIdentityInput {
        kind: ArtifactKind::View,
        format: profile::PROJECT_VIEW_SCHEMA,
        roots: &roots,
        options: &[],
    };
    let (result, count) = observe(None, || original.artifact(&input, policy, &cancel));
    let artifact = result.unwrap();
    for index in 0..count {
        assert!(
            observe(Some(index), || original.artifact(&input, policy, &cancel))
                .0
                .is_err(),
            "artifact allocation {index}/{count}"
        );
        let signal = CancellationToken::new();
        assert!(
            observe_cancellation(index, &signal, || original
                .artifact(&input, policy, &signal))
            .0
            .is_err(),
            "artifact cancellation {index}/{count}"
        );
    }
    assert_eq!(
        original.artifact(&input, policy, &cancel).unwrap(),
        artifact
    );
    assert_eq!(
        reader
            .identities(&context, policy, &cancel)
            .unwrap()
            .derivation(),
        original.derivation()
    );
}

/// Canonical typed facts must reject the same malformed wrappers/order/schema as the frozen oracle.
#[test]
fn security_composition_identity_rejects_registered_raw_fact_faults() {
    let data = vectors();
    let fixture: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let base = compile(&fixture["cases"][0]);
    for case in data["rejections"].as_array().unwrap().iter().filter(|v| {
        !matches!(
            v["id"].as_str().unwrap(),
            "nested-absent-value" | "non-normalized-exact-number" | "unknown-feature"
        )
    }) {
        let request = variant(&data["request"], &case["edits"]);
        let ir = facts::project(base.complete_ir(), &request["logical"]);
        let cancel = CancellationToken::new();
        if case["cancelled"] == true {
            cancel.cancel();
        }
        let policy = IdentityLimits {
            bytes: request["limits"]["bytes"].as_u64().unwrap(),
            nodes: request["limits"]["nodes"].as_u64().unwrap(),
        };
        let expected = match case["expected"]["error"].as_str().unwrap() {
            "Limit" => IdentityError::Limit,
            "Cancelled" => IdentityError::Cancelled,
            "InvalidInput" => IdentityError::InvalidInput,
            other => panic!("unknown failure {other}"),
        };
        assert_eq!(
            canonical_composition_project(&ir, policy, &cancel).unwrap_err(),
            expected,
            "{}",
            case["id"]
        );
    }
    let bad = &data["rejections"][1]["edits"][0]["value"];
    assert_eq!(
        neutral_ir::ExactNumber::from_normalized_parts(
            bad["negative"].as_bool().unwrap(),
            bad["coefficient"].as_str().unwrap(),
            bad["scale"].as_i64().unwrap(),
            65_536,
            u64::MAX
        )
        .unwrap_err(),
        neutral_ir::IrError::InvalidExactNumber
    );
    let captured = capture_composition_project(request(&fixture["cases"][0])).unwrap();
    let sources = source_facts(&captured);
    let vocabularies = vocabulary_facts(&captured);
    let unknown: Vec<_> = data["rejections"][5]["edits"][0]["value"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        neutral_ir::project_identity::captured_composition_closure(
            &CapturedIdentityInput {
                profile: neutral_core::profile::V1_SOURCE_PROFILE,
                sources: &sources,
                vocabularies: &vocabularies
            },
            &unknown,
            IdentityLimits {
                bytes: 1_048_576,
                nodes: 65_536
            },
            &CancellationToken::new()
        )
        .unwrap_err(),
        IdentityError::InvalidInput
    );
    // Nested absence has no typed value constructor; exercise the hostile wire boundary instead.
    let mut encoded = encode_composition_project(&base, &CancellationToken::new()).unwrap();
    let payload = b"\x82\x66string\x62ok";
    let offset = encoded
        .windows(payload.len())
        .rposition(|v| v == payload)
        .unwrap();
    encoded.splice(
        offset..offset + payload.len(),
        b"\x81\x66absent".iter().copied(),
    );
    assert_eq!(
        decode_composition_project(
            &encoded,
            DecodeLimits::hard(),
            base.complete_ir().limits,
            limits_for(&fixture["cases"][0]),
            &CancellationToken::new()
        )
        .unwrap_err()
        .class(),
        neutral_encoding::DecodeErrorClass::InvalidEncodedSchema
    );
}

/// Reads an explicitly frozen ordered policy tuple, without deriving expected values.
fn tuple<const N: usize>(value: &Value) -> [u64; N] {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap()
}

/// Reads explicit frozen producer controls without taking policy from package or host state.
fn derivation_context(
    captured: neutral_ir::project_identity::CompositionCapturedClosureIdentity,
    input: &Value,
) -> CompositionDerivationContext<'_> {
    let p = tuple::<6>(&input["project_limits"]);
    CompositionDerivationContext {
        captured,
        producer: input["producer"].as_str().unwrap(),
        producer_version: input["producer_version"].as_str().unwrap(),
        capture_limits: tuple(&input["capture_limits"]),
        project_limits: ProjectLimits {
            modules: p[0],
            declarations: p[1],
            import_edges: p[2],
            nodes: p[3],
            text_bytes: p[4],
            artifact_bytes: p[5],
        },
        composition_limits: CompositionPolicy::from_values(tuple(&input["composition_limits"])),
    }
}

/// Every explicit policy control changes derivation only; zeros and tight budgets fail closed.
#[test]
fn security_composition_derivation_controls_and_artifact_boundaries() {
    let data = vectors();
    let fixture: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let captured = capture_composition_project(request(&fixture["cases"][0])).unwrap();
    let reader = compile(&fixture["cases"][0]);
    let cancel = CancellationToken::new();
    let policy = IdentityLimits {
        bytes: 1_048_576,
        nodes: 65_536,
    };
    let captured = captured.identity_transcript(policy, &cancel).unwrap();
    let logical = canonical_composition_project(reader.complete_ir(), policy, &cancel).unwrap();
    for member in ["capture_limits", "project_limits", "composition_limits"] {
        for index in 0..data["request"][member].as_array().unwrap().len() {
            let mut request = data["request"].clone();
            request[member][index] = json!(request[member][index].as_u64().unwrap() + 1);
            let actual = composition_derivation_identity(
                logical.identity(),
                &derivation_context(captured.identity(), &request),
                policy,
                &cancel,
            )
            .unwrap();
            let expected = inspect(&request);
            assert_eq!(
                actual.bytes(),
                bytes(&expected["derivation"]["transcript_hex"]),
                "{member}/{index}"
            );
            assert_ne!(
                actual.identity().to_string(),
                data["expected"]["derivation"]["sha256"].as_str().unwrap()
            );
            request[member][index] = json!(0);
            assert_eq!(
                composition_derivation_identity(
                    logical.identity(),
                    &derivation_context(captured.identity(), &request),
                    policy,
                    &cancel
                )
                .unwrap_err(),
                IdentityError::InvalidInput
            );
        }
    }
    let context = derivation_context(captured.identity(), &data["request"]);
    let expected = &data["expected"]["derivation"];
    let exact = exact_limits(expected);
    let derivation =
        composition_derivation_identity(logical.identity(), &context, exact, &cancel).unwrap();
    for limited in insufficient_limits(exact) {
        assert_eq!(
            composition_derivation_identity(logical.identity(), &context, limited, &cancel)
                .unwrap_err(),
            IdentityError::Limit
        );
    }
    for artifact in data["request"]["artifacts"].as_array().unwrap() {
        let roots = roots(artifact);
        let input = artifact_input(artifact, &roots);
        let expected = &data["expected"][artifact["layer"].as_str().unwrap()];
        let exact = exact_limits(expected);
        for limited in insufficient_limits(exact) {
            assert_eq!(
                composition_artifact_identity(derivation.identity(), &input, limited, &cancel)
                    .unwrap_err(),
                IdentityError::Limit
            );
        }
        let cancelled = CancellationToken::new();
        cancelled.cancel();
        assert_eq!(
            composition_artifact_identity(derivation.identity(), &input, exact, &cancelled)
                .unwrap_err(),
            IdentityError::Cancelled
        );
    }
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert_eq!(
        composition_derivation_identity(logical.identity(), &context, exact, &cancelled)
            .unwrap_err(),
        IdentityError::Cancelled
    );
}

/// Uses literal byte/frame boundaries and compares both transcript bytes and SHA-256.
#[test]
fn conformance_composition_derivation_and_artifact_match_frozen_vectors() {
    let data = vectors();
    let fixture: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let captured = capture_composition_project(request(&fixture["cases"][0])).unwrap();
    let project = compile(&fixture["cases"][0]);
    let cancel = CancellationToken::new();
    let limits = IdentityLimits {
        bytes: 1_048_576,
        nodes: 65_536,
    };
    let logical = canonical_composition_project(project.complete_ir(), limits, &cancel).unwrap();
    let captured = captured.identity_transcript(limits, &cancel).unwrap();
    let input = &data["request"];
    let p = tuple::<6>(&input["project_limits"]);
    let context = CompositionDerivationContext {
        captured: captured.identity(),
        producer: input["producer"].as_str().unwrap(),
        producer_version: input["producer_version"].as_str().unwrap(),
        capture_limits: tuple(&input["capture_limits"]),
        project_limits: ProjectLimits {
            modules: p[0],
            declarations: p[1],
            import_edges: p[2],
            nodes: p[3],
            text_bytes: p[4],
            artifact_bytes: p[5],
        },
        composition_limits: CompositionPolicy::from_values(tuple(&input["composition_limits"])),
    };
    let expected = &data["expected"]["derivation"];
    let exact = IdentityLimits {
        bytes: expected["transcript_bytes"].as_u64().unwrap(),
        nodes: expected["frames"].as_u64().unwrap(),
    };
    let derivation =
        composition_derivation_identity(logical.identity(), &context, exact, &cancel).unwrap();
    assert_eq!(derivation.bytes(), bytes(&expected["transcript_hex"]));
    assert_eq!(
        derivation.identity().to_string(),
        expected["sha256"].as_str().unwrap()
    );
    for v in input["artifacts"].as_array().unwrap() {
        let roots: Vec<_> = v["roots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| {
                ModuleSymbolIdentity::new(
                    LogicalModuleIdentity::new(r[0].as_str().unwrap(), r[1].as_str().unwrap()),
                    r[2].as_str().unwrap(),
                )
            })
            .collect();
        let artifact = ArtifactIdentityInput {
            kind: if v["kind"] == "project" {
                ArtifactKind::Project
            } else {
                ArtifactKind::View
            },
            format: v["format"].as_str().unwrap(),
            roots: &roots,
            options: &[],
        };
        let expected = &data["expected"][format!("artifact-{}", v["kind"].as_str().unwrap())];
        let exact = IdentityLimits {
            bytes: expected["transcript_bytes"].as_u64().unwrap(),
            nodes: expected["frames"].as_u64().unwrap(),
        };
        let actual =
            composition_artifact_identity(derivation.identity(), &artifact, exact, &cancel)
                .unwrap();
        assert_eq!(actual.bytes(), bytes(&expected["transcript_hex"]));
        assert_eq!(
            actual.identity().to_string(),
            expected["sha256"].as_str().unwrap()
        );
    }
}
