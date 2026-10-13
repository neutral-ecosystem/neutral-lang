// SPDX-License-Identifier: Apache-2.0

//! Exact typed entries over independently decoded artifacts; no execution or first-match selection.

use super::consumer::{owner, project, protocol, roundtrip};
use super::*;
use neutral_ir::project_interface::ProjectPublicType as T;
use neutral_reader::consumer::*;

#[test]
fn security_view_charges_unselected_origin_scans() {
    use neutral_reader::composition::{
        CompositionReadError, CompositionViewLimits, CompositionViewRequest,
    };
    let source = format!(
        "neu \"1.0\"\nmodule example\npublic string selected = \"safe\"\npublic List<num> unrelated = [{}]\n",
        vec!["0"; 512].join(",")
    );
    let case = source_case(&source);
    let mut setup_limits = limits_for(&case);
    setup_limits.work = neutral_vocabulary::composition::MAX_WORK;
    let captured = capture_composition_project(request_with_limits(&case, setup_limits)).unwrap();
    let cancel = CancellationToken::new();
    let ir = compile_composition_project(&captured, &cancel).unwrap();
    let project =
        ValidatedCompositionProject::from_ir(Arc::clone(&ir), ir.limits, setup_limits, &cancel)
            .unwrap();
    let request = CompositionViewRequest {
        schema: profile::PROJECT_VIEW_SCHEMA.into(),
        roots: vec![owner("selected")],
    };
    assert_eq!(
        project
            .derive_view_with_limits(
                &request,
                CompositionViewLimits {
                    work: 1024,
                    ..CompositionViewLimits::default()
                },
                &cancel
            )
            .unwrap_err(),
        CompositionReadError::Limit
    );
}

#[test]
fn security_entry_view_exhausted_work_rejects_before_root_reservation() {
    let cancel = CancellationToken::new();
    let project = project();
    let report = T::Nominal(owner("Receipt"));
    let contract = T::Nominal(owner("Capability"));
    let roots = [owner("copied"), owner("description"), owner("entry")];
    let request = EntryRequest {
        schema: ENTRY_REQUEST_SCHEMA,
        contract: &contract,
        selection: EntrySelection::Explicit(&roots),
    };
    let run = |work| {
        ConsumerBoundary::new(
            &project,
            protocol(&report),
            ConsumerLimits {
                work,
                ..ConsumerLimits::default()
            },
            &cancel,
        )
        .and_then(|b| b.entries(&request, &cancel))
    };
    let mut low = 1;
    let mut high = 100_000;
    while low + 1 < high {
        let middle = low + (high - low) / 2;
        if run(middle).is_ok() {
            high = middle;
        } else {
            low = middle;
        }
    }
    let selected = run(high).unwrap();
    let (result, reservations) =
        neutral_core::allocation::testing::observe(None, || selected.derive_view(&cancel));
    assert_eq!(result.unwrap_err(), ConsumerError::Limit);
    assert_eq!(
        reservations, 0,
        "no root/schema allocation after work is exhausted"
    );
}

#[test]
fn security_view_bounds_unused_large_defaults_not_only_selected_values() {
    use neutral_reader::composition::{
        CompositionReadError, CompositionViewLimits, CompositionViewRequest,
    };
    let source = format!(
        "neu \"1.0\"\nmodule example\npublic record LargeDefault {{ string text = \"{}\", }}\npublic LargeDefault selected = {{ text: \"short\", }}\n",
        "a".repeat(8192)
    );
    let project = compile(&source_case(&source));
    let cancel = CancellationToken::new();
    let request = CompositionViewRequest {
        schema: profile::PROJECT_VIEW_SCHEMA.into(),
        roots: vec![owner("selected")],
    };
    // The unused default's 8192 UTF-8 bytes alone exceed this independently chosen 4096-byte output bound.
    assert_eq!(
        project
            .derive_view_with_limits(
                &request,
                CompositionViewLimits {
                    output_bytes: 4096,
                    ..CompositionViewLimits::default()
                },
                &cancel
            )
            .unwrap_err(),
        CompositionReadError::Limit
    );
    let view = project.derive_view(&request, &cancel).unwrap();
    let definition = view
        .declarations()
        .iter()
        .find(|d| d.identity.declaration_name() == "LargeDefault")
        .unwrap();
    let neutral_reader::composition::CompositionSignature::Definition(
        neutral_reader::composition::CompositionBody::Record(fields),
    ) = &definition.signature
    else {
        panic!("record contract required")
    };
    assert_eq!(
        fields[0].default,
        Some(neutral_ir::composition::ClosedValue::String(
            "a".repeat(8192)
        ))
    );
}

fn compatible() -> ConsumerCompatibilityRequest<'static> {
    ConsumerCompatibilityRequest {
        profile: CONSUMER_PROFILE,
        artifact: profile::COMPOSITION_PROFILE,
        vocabulary_schemas: &[profile::VOCABULARY_SCHEMA_VERSION],
        required_features: CONSUMER_FEATURES,
        authoring_profile: None,
    }
}

#[test]
fn integration_entries_match_registered_literal_oracle_outcomes() {
    // Literal expectations from the frozen consumer boundary entry cases, not a production-derived oracle.
    let cancel = CancellationToken::new();
    let project = roundtrip(&compile(&source_case(sources::ENTRIES)), &cancel);
    let report = T::Nominal(owner("Service"));
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&report),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let reverse = [owner("copied"), owner("api")];
    let private = [owner("hidden")];
    for (id, ty, selection, names, code) in [
        (
            "entry-enumerate",
            "Service",
            EntrySelection::Enumerate,
            &["api", "copied"][..],
            None,
        ),
        (
            "entry-ambiguous",
            "Service",
            EntrySelection::Unique,
            &[][..],
            Some("NEU-CONS-007"),
        ),
        (
            "entry-unique",
            "State",
            EntrySelection::Unique,
            &["status"][..],
            None,
        ),
        (
            "entry-no-matching-binding",
            "Empty",
            EntrySelection::Unique,
            &[][..],
            Some("NEU-CONS-006"),
        ),
        (
            "entry-explicit-empty",
            "Service",
            EntrySelection::Explicit(&[]),
            &[][..],
            None,
        ),
        (
            "entry-explicit-private",
            "Service",
            EntrySelection::Explicit(&private),
            &[][..],
            Some("NEU-CONS-005"),
        ),
        (
            "entry-explicit-order",
            "Service",
            EntrySelection::Explicit(&reverse),
            &["api", "copied"][..],
            None,
        ),
    ] {
        let contract = T::Nominal(owner(ty));
        let request = EntryRequest {
            schema: ENTRY_REQUEST_SCHEMA,
            contract: &contract,
            selection,
        };
        match boundary.entries(&request, &cancel) {
            Ok(selected) => {
                assert_eq!(code, None, "{id}");
                let actual: Vec<_> = selected
                    .bindings()
                    .iter()
                    .map(|b| b.owner.declaration_name())
                    .collect();
                assert_eq!(actual, names, "{id}");
                let view = selected.derive_view(&cancel).unwrap();
                assert_eq!(view.roots().len(), names.len(), "{id}");
            }
            Err(error) => assert_eq!(Some(error.code()), code, "{id}"),
        }
    }
}

#[test]
fn security_consumer_exact_tuple_fields_never_fall_back() {
    let request = compatible();
    let cancel = CancellationToken::new();
    for artifact in [
        profile::CompositionProfileDescriptor {
            source_profile: "0.1",
            ..request.artifact
        },
        profile::CompositionProfileDescriptor {
            capture_profile: "neutral.capture/v3",
            ..request.artifact
        },
        profile::CompositionProfileDescriptor {
            project_ir: "neutral.project-ir/1",
            ..request.artifact
        },
        profile::CompositionProfileDescriptor {
            view: "neutral.project-view/unknown",
            ..request.artifact
        },
        profile::CompositionProfileDescriptor {
            result: "neutral.project-result/unknown",
            ..request.artifact
        },
        profile::CompositionProfileDescriptor {
            identity: "neutral.project-identity/1",
            ..request.artifact
        },
        profile::CompositionProfileDescriptor {
            required_features: &["unknown"],
            ..request.artifact
        },
    ] {
        assert_eq!(
            negotiate_consumer(
                &ConsumerCompatibilityRequest {
                    artifact,
                    ..request
                },
                ConsumerLimits::default(),
                &cancel
            )
            .unwrap_err(),
            ConsumerError::Unsupported
        );
    }
    assert_eq!(
        neutral_core::profile::language_profile(LanguageProfile::V1_0).availability(),
        neutral_core::profile::ProfileAvailability::Unavailable
    );
}

#[test]
fn integration_entries_standalone_encoded_probe_checks_compatibility_and_identity() {
    use neutral_probe::composition::{
        CompositionEntryProbeRequest, CompositionProbeError, inspect_composition_entries_encoded,
    };
    let cancel = CancellationToken::new();
    let project = project();
    let bytes = encode_composition_project(&project, &cancel).unwrap();
    let contract = T::Nominal(owner("Capability"));
    let roots = [owner("entry")];
    let request = CompositionEntryProbeRequest {
        compatibility: compatible(),
        entry: EntryRequest {
            schema: ENTRY_REQUEST_SCHEMA,
            contract: &contract,
            selection: EntrySelection::Explicit(&roots),
        },
        limits: ConsumerLimits::default(),
    };
    let run = |request: &CompositionEntryProbeRequest<'_>| {
        inspect_composition_entries_encoded(
            &bytes,
            DecodeLimits::hard(),
            project.complete_ir().limits,
            limits_for(&source_case("")),
            request,
            &cancel,
        )
    };
    let summary = run(&request).unwrap();
    assert_eq!(summary.view.roots(), roots);
    let complete = neutral_probe::composition::inspect_composition_encoded(
        &bytes,
        DecodeLimits::hard(),
        project.complete_ir().limits,
        limits_for(&source_case("")),
        None,
        &cancel,
    )
    .unwrap();
    assert_eq!(summary.logical_identity, complete.logical_identity);
    assert_eq!(summary.view.profile_facts().encoding, "NIR-PROJECT-CBOR/2");
    let empty = run(&CompositionEntryProbeRequest {
        entry: EntryRequest {
            selection: EntrySelection::Explicit(&[]),
            ..request.entry
        },
        ..request
    })
    .unwrap();
    assert_eq!(empty.view.declarations(), []);
    assert_eq!(empty.logical_identity, summary.logical_identity);
    let unsupported = CompositionEntryProbeRequest {
        compatibility: ConsumerCompatibilityRequest {
            required_features: &["unknown"],
            ..request.compatibility
        },
        ..request
    };
    assert_eq!(
        run(&unsupported).unwrap_err(),
        CompositionProbeError::Consumer(ConsumerError::Unsupported)
    );
    assert!(
        inspect_composition_entries_encoded(
            &bytes[..bytes.len() - 1],
            DecodeLimits::hard(),
            project.complete_ir().limits,
            limits_for(&source_case("")),
            &request,
            &cancel
        )
        .is_err()
    );
    assert_eq!(
        run(&CompositionEntryProbeRequest {
            limits: ConsumerLimits {
                output_bytes: 1,
                ..request.limits
            },
            ..request
        })
        .unwrap_err(),
        CompositionProbeError::Consumer(ConsumerError::Limit)
    );
}

#[test]
fn security_standalone_entry_probe_allocation_and_cancellation_are_atomic() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    use neutral_probe::composition::{
        CompositionEntryProbeRequest, inspect_composition_entries_encoded,
    };
    let cancel = CancellationToken::new();
    let project = project();
    let bytes = encode_composition_project(&project, &cancel).unwrap();
    let contract = T::Nominal(owner("Capability"));
    let request = CompositionEntryProbeRequest {
        compatibility: compatible(),
        entry: EntryRequest {
            schema: ENTRY_REQUEST_SCHEMA,
            contract: &contract,
            selection: EntrySelection::Enumerate,
        },
        limits: ConsumerLimits::default(),
    };
    let run = |signal: &CancellationToken| {
        inspect_composition_entries_encoded(
            &bytes,
            DecodeLimits::hard(),
            project.complete_ir().limits,
            limits_for(&source_case("")),
            &request,
            signal,
        )
    };
    let (result, count) = observe(None, || run(&cancel));
    assert_eq!(result.unwrap().view.roots().len(), 3);
    for index in 0..count {
        assert!(
            observe(Some(index), || run(&cancel)).0.is_err(),
            "allocation {index}"
        );
        let signal = CancellationToken::new();
        assert!(
            observe_cancellation(index, &signal, || run(&signal))
                .0
                .is_err(),
            "cancellation {index}"
        );
    }
    assert_eq!(run(&cancel).unwrap().view.roots().len(), 3);
}

#[test]
fn integration_entries_vocabulary_revision_and_actual_schema_are_checked() {
    let cases: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let project = roundtrip(&compile(&cases["cases"][0]), &CancellationToken::new());
    let cancel = CancellationToken::new();
    let contract = T::VocabularyNominal {
        identity: "ExampleDomain".into(),
        version: "1.0.0".into(),
        name: "Request".into(),
    };
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let request = EntryRequest {
        schema: ENTRY_REQUEST_SCHEMA,
        contract: &contract,
        selection: EntrySelection::Unique,
    };
    let selected = boundary.entries(&request, &cancel).unwrap();
    assert_eq!(selected.bindings()[0].owner.declaration_name(), "request");
    let stale = T::VocabularyNominal {
        identity: "ExampleDomain".into(),
        version: "2.0.0".into(),
        name: "Request".into(),
    };
    assert_eq!(
        boundary
            .entries(
                &EntryRequest {
                    contract: &stale,
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::PrivateTarget
    );
    let compatible = compatible();
    let support = negotiate_consumer(&compatible, ConsumerLimits::default(), &cancel).unwrap();
    let absent = ConsumerCompatibilityRequest {
        vocabulary_schemas: &[],
        ..compatible
    };
    assert_eq!(
        support
            .validate_project(&project, &absent, ConsumerLimits::default(), &cancel)
            .unwrap_err(),
        ConsumerError::Unsupported
    );
    support
        .validate_project(&project, &compatible, ConsumerLimits::default(), &cancel)
        .unwrap();
    let view = selected.derive_view(&cancel).unwrap();
    assert_eq!(view.vocabularies().len(), 1);
    assert!(
        view.vocabularies()[0]
            .definitions
            .iter()
            .any(|d| d.name == "Request")
    );
}

#[test]
fn integration_consumer_exact_profile_and_feature_negotiation() {
    let cancel = CancellationToken::new();
    let limits = ConsumerLimits::default();
    let request = compatible();
    let supported = negotiate_consumer(&request, limits, &cancel).unwrap();
    assert_eq!(
        supported.availability,
        neutral_core::profile::ProfileAvailability::Available
    );
    assert_eq!(
        supported.features,
        [
            "entry-discovery-v1",
            "member-selectors-v1",
            "value-attribution-v1"
        ]
    );
    assert_eq!(supported.artifact.source_profile, "1.0");
    assert_eq!(supported.artifact.encoding, "NIR-PROJECT-CBOR/2");
    for encoding in [
        "NIR-CBOR/0.1",
        "NIR-PROJECT-CBOR/1",
        "NIR-PROJECT-CBOR/unknown",
    ] {
        let artifact = profile::CompositionProfileDescriptor {
            encoding,
            ..request.artifact
        };
        assert_eq!(
            negotiate_consumer(
                &ConsumerCompatibilityRequest {
                    artifact,
                    ..request
                },
                limits,
                &cancel
            )
            .unwrap_err(),
            ConsumerError::Unsupported
        );
    }
}

#[test]
fn security_consumer_negotiation_rejects_malformed_and_unavailable_features() {
    let cancel = CancellationToken::new();
    let limits = ConsumerLimits::default();
    let request = compatible();
    for required in [
        &["entry-discovery-v1"][..],
        &["entry-discovery-v1", "member-selectors-v1", "unknown"][..],
    ] {
        assert_eq!(
            negotiate_consumer(
                &ConsumerCompatibilityRequest {
                    required_features: required,
                    ..request
                },
                limits,
                &cancel
            )
            .unwrap_err(),
            ConsumerError::Unsupported
        );
    }
    assert_eq!(
        negotiate_consumer(
            &ConsumerCompatibilityRequest {
                required_features: &["entry-discovery-v1", "entry-discovery-v1"],
                ..request
            },
            limits,
            &cancel
        )
        .unwrap_err(),
        ConsumerError::InvalidInput
    );
    assert_eq!(
        negotiate_consumer(
            &ConsumerCompatibilityRequest {
                authoring_profile: Some("neutral.authoring/1"),
                ..request
            },
            limits,
            &cancel
        )
        .unwrap_err(),
        ConsumerError::Unsupported
    );
    assert_eq!(
        negotiate_consumer(
            &ConsumerCompatibilityRequest {
                vocabulary_schemas: &["unknown"],
                ..request
            },
            limits,
            &cancel
        )
        .unwrap_err(),
        ConsumerError::Unsupported
    );
    assert_eq!(
        negotiate_consumer(
            &request,
            ConsumerLimits {
                features: 2,
                ..limits
            },
            &cancel
        )
        .unwrap_err(),
        ConsumerError::Limit
    );
    cancel.cancel();
    assert_eq!(
        negotiate_consumer(&request, limits, &cancel).unwrap_err(),
        ConsumerError::Cancelled
    );
}

#[test]
fn integration_entries_views_retain_contracts_and_redact_private_origins() {
    let cancel = CancellationToken::new();
    let project = roundtrip(&project(), &cancel);
    let report = T::Nominal(owner("Receipt"));
    let contract = T::Nominal(owner("Capability"));
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&report),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let roots = [owner("copied")];
    let request = EntryRequest {
        schema: ENTRY_REQUEST_SCHEMA,
        contract: &contract,
        selection: EntrySelection::Explicit(&roots),
    };
    let selected = boundary.entries(&request, &cancel).unwrap();
    let view = selected.derive_view(&cancel).unwrap();
    assert_eq!(view.roots(), roots);
    let names: Vec<_> = view
        .declarations()
        .iter()
        .map(|d| d.identity.declaration_name())
        .collect();
    assert_eq!(names, ["Capability", "copied"]);
    assert!(view.origins().iter().all(|o| o.attribution.is_none()));
    let empty = boundary
        .entries(
            &EntryRequest {
                selection: EntrySelection::Explicit(&[]),
                ..request
            },
            &cancel,
        )
        .unwrap()
        .derive_view(&cancel)
        .unwrap();
    assert_eq!(empty.declarations(), []);
    assert_eq!(empty.vocabularies(), []);
    assert_eq!(empty.references(), []);
}

#[test]
fn security_entry_view_closure_and_output_limits_precede_copy() {
    use neutral_reader::composition::{
        CompositionReadError, CompositionViewLimits, CompositionViewRequest,
    };
    let cancel = CancellationToken::new();
    let project = project();
    let request = CompositionViewRequest {
        schema: profile::PROJECT_VIEW_SCHEMA.into(),
        roots: vec![owner("description")],
    };
    let run = |output| {
        project.derive_view_with_limits(
            &request,
            CompositionViewLimits {
                output_bytes: output,
                ..CompositionViewLimits::default()
            },
            &cancel,
        )
    };
    let mut low = 1;
    let mut high = 100_000;
    assert!(run(high).is_ok());
    while low + 1 < high {
        let middle = low + (high - low) / 2;
        if run(middle).is_ok() {
            high = middle;
        } else {
            low = middle;
        }
    }
    assert_eq!(run(high - 1).unwrap_err(), CompositionReadError::Limit);
    assert!(run(high).is_ok());
    assert_eq!(
        project
            .derive_view_with_limits(
                &request,
                CompositionViewLimits {
                    closure_edges: 1,
                    ..CompositionViewLimits::default()
                },
                &cancel
            )
            .unwrap_err(),
        CompositionReadError::Limit
    );
    assert_eq!(
        project
            .derive_view_with_limits(
                &request,
                CompositionViewLimits {
                    work: 1,
                    ..CompositionViewLimits::default()
                },
                &cancel
            )
            .unwrap_err(),
        CompositionReadError::Limit
    );
    assert_eq!(
        project
            .derive_view_with_limits(
                &request,
                CompositionViewLimits {
                    roots: 0,
                    ..CompositionViewLimits::default()
                },
                &cancel
            )
            .unwrap_err(),
        CompositionReadError::Limit
    );
    let (rejected, reservations) = neutral_core::allocation::testing::observe(None, || run(1));
    assert_eq!(rejected.unwrap_err(), CompositionReadError::Limit);
    assert_eq!(
        reservations, 0,
        "output preflight must precede root retention"
    );
}

#[test]
fn security_entry_view_faults_leave_selection_and_project_usable() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    let cancel = CancellationToken::new();
    let project = project();
    let report = T::Nominal(owner("Receipt"));
    let contract = T::Nominal(owner("Capability"));
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&report),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let selected = boundary
        .entries(
            &EntryRequest {
                schema: ENTRY_REQUEST_SCHEMA,
                contract: &contract,
                selection: EntrySelection::Enumerate,
            },
            &cancel,
        )
        .unwrap();
    let identity = boundary.project_identity();
    let (result, count) = observe(None, || selected.derive_view(&cancel));
    assert_eq!(result.unwrap().roots().len(), 3);
    for index in 0..count {
        assert_eq!(
            observe(Some(index), || selected.derive_view(&cancel))
                .0
                .unwrap_err(),
            ConsumerError::Limit,
            "allocation {index}"
        );
        let signal = CancellationToken::new();
        assert_eq!(
            observe_cancellation(index, &signal, || selected.derive_view(&signal))
                .0
                .unwrap_err(),
            ConsumerError::Cancelled,
            "cancellation {index}"
        );
    }
    assert_eq!(selected.derive_view(&cancel).unwrap().roots().len(), 3);
    assert_eq!(boundary.project_identity(), identity);
}

#[test]
fn integration_entries_enumerate_only_exact_public_bindings() {
    let cancel = CancellationToken::new();
    let project = roundtrip(&project(), &cancel);
    let report = T::Nominal(owner("Receipt"));
    let contract = T::Nominal(owner("Capability"));
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&report),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let identity = boundary.project_identity();
    let request = EntryRequest {
        schema: ENTRY_REQUEST_SCHEMA,
        contract: &contract,
        selection: EntrySelection::Enumerate,
    };
    let selected = boundary.entries(&request, &cancel).unwrap();
    let names: Vec<_> = selected
        .bindings()
        .iter()
        .map(|b| b.owner.declaration_name())
        .collect();
    assert_eq!(names, ["copied", "description", "entry"]);
    assert_eq!(boundary.project_identity(), identity);
    assert_eq!(
        boundary
            .entries(
                &EntryRequest {
                    selection: EntrySelection::Unique,
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::AmbiguousEntry
    );
    assert_eq!(
        boundary
            .entries(
                &EntryRequest {
                    contract: &report,
                    selection: EntrySelection::Unique,
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::MissingEntry
    );
}

#[test]
fn integration_entries_explicit_roots_are_canonical_and_fail_closed() {
    let cancel = CancellationToken::new();
    let project = roundtrip(&project(), &cancel);
    let report = T::Nominal(owner("Receipt"));
    let contract = T::Nominal(owner("Capability"));
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&report),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let roots = [owner("entry"), owner("description")];
    let request = EntryRequest {
        schema: ENTRY_REQUEST_SCHEMA,
        contract: &contract,
        selection: EntrySelection::Explicit(&roots),
    };
    let result = boundary.entries(&request, &cancel).unwrap();
    let names: Vec<_> = result
        .bindings()
        .iter()
        .map(|b| b.owner.declaration_name())
        .collect();
    assert_eq!(names, ["description", "entry"]);
    assert_eq!(
        boundary
            .entries(
                &EntryRequest {
                    selection: EntrySelection::Explicit(&[]),
                    ..request
                },
                &cancel
            )
            .unwrap()
            .bindings()
            .len(),
        0
    );
    let duplicate = [owner("entry"), owner("entry")];
    assert_eq!(
        boundary
            .entries(
                &EntryRequest {
                    selection: EntrySelection::Explicit(&duplicate),
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::InvalidInput
    );
}

#[test]
fn security_entries_explicit_targets_require_public_exact_binding_authority() {
    let cancel = CancellationToken::new();
    let project = roundtrip(&project(), &cancel);
    let report = T::Nominal(owner("Receipt"));
    let contract = T::Nominal(owner("Capability"));
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&report),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let roots = [owner("entry")];
    let request = EntryRequest {
        schema: ENTRY_REQUEST_SCHEMA,
        contract: &contract,
        selection: EntrySelection::Explicit(&roots),
    };
    for name in ["private_description", "absent"] {
        let roots = [owner(name)];
        assert_eq!(
            boundary
                .entries(
                    &EntryRequest {
                        selection: EntrySelection::Explicit(&roots),
                        ..request
                    },
                    &cancel
                )
                .unwrap_err(),
            ConsumerError::PrivateTarget
        );
    }
    assert_eq!(
        boundary
            .entries(
                &EntryRequest {
                    contract: &report,
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::TypeMismatch
    );
    let definitions = [owner("Capability")];
    assert_eq!(
        boundary
            .entries(
                &EntryRequest {
                    selection: EntrySelection::Explicit(&definitions),
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::PrivateTarget
    );
    assert_eq!(
        boundary
            .entries(
                &EntryRequest {
                    schema: "unknown",
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::Unsupported
    );
}

#[test]
fn security_entries_independent_candidate_root_and_output_limits() {
    let cancel = CancellationToken::new();
    let project = project();
    let report = T::Nominal(owner("Receipt"));
    let contract = T::Nominal(owner("Capability"));
    let roots = [owner("description"), owner("entry")];
    for (limits, selection, expected) in [
        (
            ConsumerLimits {
                candidates: 3,
                roots: 3,
                ..ConsumerLimits::default()
            },
            EntrySelection::Enumerate,
            None,
        ),
        (
            ConsumerLimits {
                candidates: 2,
                ..ConsumerLimits::default()
            },
            EntrySelection::Enumerate,
            Some(ConsumerError::Limit),
        ),
        (
            ConsumerLimits {
                roots: 2,
                ..ConsumerLimits::default()
            },
            EntrySelection::Explicit(&roots),
            None,
        ),
        (
            ConsumerLimits {
                roots: 1,
                ..ConsumerLimits::default()
            },
            EntrySelection::Explicit(&roots),
            Some(ConsumerError::Limit),
        ),
        (
            ConsumerLimits {
                output_bytes: 1,
                ..ConsumerLimits::default()
            },
            EntrySelection::Enumerate,
            Some(ConsumerError::Limit),
        ),
    ] {
        let boundary = ConsumerBoundary::new(&project, protocol(&report), limits, &cancel).unwrap();
        let result = boundary.entries(
            &EntryRequest {
                schema: ENTRY_REQUEST_SCHEMA,
                contract: &contract,
                selection,
            },
            &cancel,
        );
        assert_eq!(result.err(), expected);
    }
}

#[test]
fn security_entries_allocation_cancellation_and_recovery() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    let cancel = CancellationToken::new();
    let project = project();
    let report = T::Nominal(owner("Receipt"));
    let contract = T::Nominal(owner("Capability"));
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&report),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let request = EntryRequest {
        schema: ENTRY_REQUEST_SCHEMA,
        contract: &contract,
        selection: EntrySelection::Enumerate,
    };
    let (result, count) = observe(None, || boundary.entries(&request, &cancel));
    assert_eq!(result.unwrap().bindings().len(), 3);
    assert_ne!(count, 0);
    for index in 0..count {
        let (result, _) = observe(Some(index), || boundary.entries(&request, &cancel));
        assert_eq!(
            result.unwrap_err(),
            ConsumerError::Limit,
            "allocation {index}"
        );
        let signal = CancellationToken::new();
        let (result, _) =
            observe_cancellation(index, &signal, || boundary.entries(&request, &signal));
        assert_eq!(
            result.unwrap_err(),
            ConsumerError::Cancelled,
            "cancellation {index}"
        );
        assert_eq!(
            boundary
                .entries(&request, &cancel)
                .unwrap()
                .bindings()
                .len(),
            3
        );
    }
    cancel.cancel();
    assert_eq!(
        boundary.entries(&request, &cancel).unwrap_err(),
        ConsumerError::Cancelled
    );
}
