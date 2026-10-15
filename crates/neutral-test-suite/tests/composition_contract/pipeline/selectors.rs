// SPDX-License-Identifier: Apache-2.0

//! Independent decoded-reader selectors and public origin privacy checks.

use super::consumer::{owner, project, protocol};
use super::*;
use neutral_ir::{composition::ValueOriginKind, project_interface::ProjectPublicType as T};
use neutral_reader::consumer::*;

#[test]
fn security_consumer_origins_enforce_independent_output_bytes() {
    let project = project();
    let cancel = CancellationToken::new();
    let contract = T::Nominal(owner("Receipt"));
    let subject = owner("description");
    let path = [MemberSegment::Field("kind")];
    let request = ValueOriginRequest {
        schema: VALUE_ORIGIN_SCHEMA,
        symbol: &subject,
        path: &path,
    };
    let run = |output_bytes| {
        let boundary = ConsumerBoundary::new(
            &project,
            protocol(&contract),
            ConsumerLimits {
                output_bytes,
                ..ConsumerLimits::default()
            },
            &cancel,
        )?;
        boundary.value_origin(&request, &cancel)?;
        Ok::<_, ConsumerError>(())
    };
    assert_eq!(run(1), Err(ConsumerError::Limit));
    let exact = std::mem::size_of::<PublicValueAttribution<'_>>() as u64;
    assert_eq!(run(exact), Ok(()));
    assert_eq!(run(exact - 1), Err(ConsumerError::Limit));
}

#[test]
fn security_consumer_root_reuse_preserves_safe_use_site_and_redacts_private_origin() {
    for (source, contract, private) in [
        (sources::CONSUMER, "Capability", true),
        (sources::SELECTORS, "Selection", false),
    ] {
        let project = compile(&source_case(source));
        let cancel = CancellationToken::new();
        let project = super::consumer::roundtrip(&project, &cancel);
        let contract = T::Nominal(owner(contract));
        let subject = owner("copied");
        let boundary = ConsumerBoundary::new(
            &project,
            protocol(&contract),
            ConsumerLimits::default(),
            &cancel,
        )
        .unwrap();
        let origin = boundary
            .value_origin(
                &ValueOriginRequest {
                    schema: VALUE_ORIGIN_SCHEMA,
                    symbol: &subject,
                    path: &[],
                },
                &cancel,
            )
            .unwrap();
        assert_eq!(format!("{origin:?}"), "PublicValueAttribution { .. }");
        let PublicOriginLocation::Available(location) = origin.occurrence() else {
            panic!("root reuse use site was retained");
        };
        let span = location.span();
        let original =
            &source[usize::try_from(span.start()).unwrap()..usize::try_from(span.end()).unwrap()];
        assert_eq!(
            original,
            if private {
                "private_description"
            } else {
                "supplied"
            }
        );
        if private {
            assert!(matches!(
                origin.origin(),
                PublicValueOrigin::Reuse(PublicOriginLocation::Unavailable(
                    LocationUnavailable::Redacted
                ))
            ));
        } else {
            assert!(matches!(
                origin.origin(),
                PublicValueOrigin::Reuse(PublicOriginLocation::Available(_))
            ));
        }
    }
}

#[test]
fn integration_consumer_public_origins_preserve_null_default_and_reuse_distinctions() {
    let project = compile(&source_case(sources::SELECTORS));
    let cancel = CancellationToken::new();
    let project = super::consumer::roundtrip(&project, &cancel);
    let contract = T::Nominal(owner("Selection"));
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let path = [MemberSegment::Field("label")];
    for (symbol, presence) in [
        ("supplied", ValueOriginKind::Supplied),
        ("defaulted", ValueOriginKind::Defaulted),
        ("copied", ValueOriginKind::Supplied),
    ] {
        let subject = owner(symbol);
        let origin = boundary
            .value_origin(
                &ValueOriginRequest {
                    schema: VALUE_ORIGIN_SCHEMA,
                    symbol: &subject,
                    path: &path,
                },
                &cancel,
            )
            .unwrap();
        assert_eq!(origin.presence(), presence);
        match symbol {
            "supplied" => {
                assert!(matches!(
                    origin.occurrence(),
                    PublicOriginLocation::Available(_)
                ));
                assert!(matches!(origin.origin(), PublicValueOrigin::Supplied));
            }
            "defaulted" => {
                assert!(matches!(
                    origin.occurrence(),
                    PublicOriginLocation::Unavailable(LocationUnavailable::NotRetained)
                ));
                assert!(matches!(
                    origin.origin(),
                    PublicValueOrigin::Default {
                        owner: T::Nominal(_),
                        field: "label",
                        location: PublicOriginLocation::Available(_),
                        vocabulary: None
                    }
                ));
            }
            _ => {
                assert!(matches!(
                    origin.occurrence(),
                    PublicOriginLocation::Unavailable(LocationUnavailable::NotRetained)
                ));
                assert!(matches!(
                    origin.origin(),
                    PublicValueOrigin::Reuse(PublicOriginLocation::Available(_))
                ));
            }
        }
    }
    let subject = owner("supplied");
    let path = [MemberSegment::Field("maybe")];
    let origin = boundary
        .value_origin(
            &ValueOriginRequest {
                schema: VALUE_ORIGIN_SCHEMA,
                symbol: &subject,
                path: &path,
            },
            &cancel,
        )
        .unwrap();
    assert_eq!(origin.presence(), ValueOriginKind::ExplicitNull);
    assert!(matches!(
        origin.occurrence(),
        PublicOriginLocation::Available(_)
    ));
}

#[test]
fn security_consumer_selectors_reject_nullable_traversal_private_types_and_wrong_schemas() {
    let project = compile(&source_case(sources::SELECTORS));
    let cancel = CancellationToken::new();
    let contract = T::Nominal(owner("Selection"));
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let subject = owner("supplied");
    let path = [MemberSegment::Field("labels"), MemberSegment::Index(0)];
    let request = MemberSelector {
        schema: MEMBER_SELECTOR_SCHEMA,
        root: SelectorRoot::Value(&subject),
        path: &path,
        expected_type: &T::String,
    };
    assert_eq!(
        boundary.select(&request, &cancel).unwrap_err(),
        ConsumerError::MissingMember
    );
    let hidden = T::Nominal(owner("Hidden"));
    assert_eq!(
        boundary
            .select(
                &MemberSelector {
                    root: SelectorRoot::Type(&hidden),
                    path: &[],
                    expected_type: &T::String,
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::PrivateTarget
    );
    assert_eq!(
        boundary
            .select(
                &MemberSelector {
                    schema: VALUE_ORIGIN_SCHEMA,
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::Unsupported
    );
    assert_eq!(
        boundary
            .value_origin(
                &ValueOriginRequest {
                    schema: MEMBER_SELECTOR_SCHEMA,
                    symbol: &subject,
                    path: &[]
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::Unsupported
    );
}

#[test]
fn integration_consumer_selectors_resolve_values_and_retain_type_obligations() {
    let project = project();
    let project = super::consumer::roundtrip(&project, &CancellationToken::new());
    let cancel = CancellationToken::new();
    let contract = T::Nominal(owner("Receipt"));
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let subject = owner("description");
    let ty = T::Nominal(owner("Capability"));
    let path = [MemberSegment::Field("labels"), MemberSegment::Index(1)];
    let request = MemberSelector {
        schema: MEMBER_SELECTOR_SCHEMA,
        root: SelectorRoot::Value(&subject),
        path: &path,
        expected_type: &T::String,
    };
    let selected = boundary.select(&request, &cancel).unwrap();
    assert_eq!(format!("{selected:?}"), "SelectedMember { .. }");
    assert_eq!(selected.identity().schema, MEMBER_SELECTOR_SCHEMA);
    assert_eq!(selected.identity().path, path);
    assert!(matches!(selected.identity().root, SelectorRoot::Value(root) if root == &subject));
    assert_eq!(selected.identity().resolved_type, &T::String);
    assert_eq!(
        selected.value(),
        Some(&BindingValue::String("typed".into()))
    );
    assert_eq!(selected.resolved_type(), &T::String);
    assert_eq!(selected.obligations(), []);
    let type_request = MemberSelector {
        root: SelectorRoot::Type(&ty),
        ..request
    };
    let selected = boundary.select(&type_request, &cancel).unwrap();
    assert_eq!(selected.value(), None);
    assert_eq!(
        selected.obligations(),
        [AvailabilityObligation {
            segment: 1,
            kind: AvailabilityKind::Index
        }]
    );
    assert_eq!(
        boundary
            .select(
                &MemberSelector {
                    expected_type: &T::Num,
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::TypeMismatch
    );
    let bad = [MemberSegment::Field("labels"), MemberSegment::Index(2)];
    assert_eq!(
        boundary
            .select(
                &MemberSelector {
                    path: &bad,
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::MissingMember
    );
    let origin = boundary
        .value_origin(
            &ValueOriginRequest {
                schema: VALUE_ORIGIN_SCHEMA,
                symbol: &subject,
                path: &path,
            },
            &cancel,
        )
        .unwrap();
    assert_eq!(origin.presence(), ValueOriginKind::Defaulted);
    assert!(matches!(origin.origin(), PublicValueOrigin::Default { .. }));
}

#[test]
fn security_consumer_origin_lookup_redacts_private_reuse_and_checks_paths() {
    let project = project();
    let cancel = CancellationToken::new();
    let contract = T::Nominal(owner("Receipt"));
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let subject = owner("copied");
    let path = [MemberSegment::Field("kind")];
    let request = ValueOriginRequest {
        schema: VALUE_ORIGIN_SCHEMA,
        symbol: &subject,
        path: &path,
    };
    let origin = boundary.value_origin(&request, &cancel).unwrap();
    assert!(matches!(
        origin.occurrence(),
        PublicOriginLocation::Unavailable(LocationUnavailable::NotRetained)
    ));
    assert!(matches!(
        origin.origin(),
        PublicValueOrigin::Reuse(PublicOriginLocation::Unavailable(
            LocationUnavailable::Redacted
        ))
    ));
    assert_eq!(
        boundary
            .value_origin(
                &ValueOriginRequest {
                    symbol: &owner("private_description"),
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::PrivateTarget
    );
    let bad = [MemberSegment::Field("missing")];
    assert_eq!(
        boundary
            .value_origin(
                &ValueOriginRequest {
                    path: &bad,
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::MissingMember
    );
    cancel.cancel();
    assert_eq!(
        boundary.value_origin(&request, &cancel).unwrap_err(),
        ConsumerError::Cancelled
    );
}

#[test]
fn integration_consumer_variant_paths_and_nested_default_owners_are_exact() {
    let project = compile(&source_case(sources::DEFAULTS));
    let cancel = CancellationToken::new();
    let project = super::consumer::roundtrip(&project, &cancel);
    let contract = T::Nominal(owner("Outer"));
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let subject = owner("states");
    let path = [
        MemberSegment::Index(0),
        MemberSegment::Alternative("ready"),
        MemberSegment::Field("inner"),
        MemberSegment::Field("count"),
    ];
    let request = MemberSelector {
        schema: MEMBER_SELECTOR_SCHEMA,
        root: SelectorRoot::Value(&subject),
        path: &path,
        expected_type: &T::Num,
    };
    assert_eq!(
        boundary.select(&request, &cancel).unwrap().value(),
        Some(&BindingValue::Number(
            neutral_ir::ExactNumber::from_unsigned_integer("42").unwrap()
        ))
    );
    let bad = [
        MemberSegment::Index(0),
        MemberSegment::Alternative("failed"),
    ];
    assert_eq!(
        boundary
            .select(
                &MemberSelector {
                    path: &bad,
                    expected_type: &T::String,
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::MissingMember
    );
    let unused = T::Nominal(owner("Inner"));
    let empty = MemberSelector {
        root: SelectorRoot::Type(&unused),
        path: &[],
        expected_type: &unused,
        ..request
    };
    assert_eq!(
        boundary.select(&empty, &cancel).unwrap().resolved_type(),
        &unused
    );
    let original = owner("original");
    let default_path = [MemberSegment::Field("inner"), MemberSegment::Field("count")];
    let origin = boundary
        .value_origin(
            &ValueOriginRequest {
                schema: VALUE_ORIGIN_SCHEMA,
                symbol: &original,
                path: &default_path,
            },
            &cancel,
        )
        .unwrap();
    let PublicValueOrigin::Default {
        owner: actual,
        field,
        ..
    } = origin.origin()
    else {
        panic!("missing default owner");
    };
    assert_eq!(actual, &T::Nominal(owner("Inner")));
    assert_eq!(field, "count");
}

#[test]
fn integration_consumer_vocabulary_omission_reference_and_type_only_variant() {
    let cases: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let project = compile(&cases["cases"][0]);
    let cancel = CancellationToken::new();
    let project = super::consumer::roundtrip(&project, &cancel);
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
    let subject = owner("request");
    let nullable = T::Nullable(Box::new(T::String));
    let path = [MemberSegment::Field("label")];
    let request = MemberSelector {
        schema: MEMBER_SELECTOR_SCHEMA,
        root: SelectorRoot::Value(&subject),
        path: &path,
        expected_type: &nullable,
    };
    assert_eq!(boundary.select(&request, &cancel).unwrap().value(), None);
    let origin = boundary
        .value_origin(
            &ValueOriginRequest {
                schema: VALUE_ORIGIN_SCHEMA,
                symbol: &subject,
                path: &path,
            },
            &cancel,
        )
        .unwrap();
    assert_eq!(origin.presence(), ValueOriginKind::OmittedOptional);
    let typed = MemberSelector {
        root: SelectorRoot::Type(&contract),
        ..request
    };
    assert_eq!(
        boundary.select(&typed, &cancel).unwrap().obligations(),
        [AvailabilityObligation {
            segment: 0,
            kind: AvailabilityKind::Presence
        }]
    );
    let path = [MemberSegment::Field("attempts")];
    let origin = boundary
        .value_origin(
            &ValueOriginRequest {
                schema: VALUE_ORIGIN_SCHEMA,
                symbol: &subject,
                path: &path,
            },
            &cancel,
        )
        .unwrap();
    assert!(matches!(
        origin.origin(),
        PublicValueOrigin::Default {
            field: "attempts",
            vocabulary: Some(_),
            ..
        }
    ));
}

#[test]
fn security_consumer_type_only_alternatives_never_dereference_or_claim_availability() {
    let cases: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let project = compile(&cases["cases"][0]);
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
    let subject = owner("request");
    let path = [
        MemberSegment::Field("selected"),
        MemberSegment::Alternative("success"),
    ];
    let request = MemberSelector {
        schema: MEMBER_SELECTOR_SCHEMA,
        root: SelectorRoot::Value(&subject),
        path: &path,
        expected_type: &T::String,
    };
    assert_eq!(
        boundary.select(&request, &cancel).unwrap_err(),
        ConsumerError::MissingMember
    );
    let path = [
        MemberSegment::Field("outcomes"),
        MemberSegment::Index(u32::MAX),
        MemberSegment::Alternative("failure"),
    ];
    let request = MemberSelector {
        root: SelectorRoot::Type(&contract),
        path: &path,
        expected_type: &T::Num,
        ..request
    };
    let selected = boundary.select(&request, &cancel).unwrap();
    assert_eq!(
        selected.obligations(),
        [
            AvailabilityObligation {
                segment: 1,
                kind: AvailabilityKind::Index
            },
            AvailabilityObligation {
                segment: 2,
                kind: AvailabilityKind::Alternative
            }
        ]
    );
}

#[test]
fn security_consumer_selector_faults_and_limits_leave_reader_unchanged() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    let project = project();
    let cancel = CancellationToken::new();
    let contract = T::Nominal(owner("Receipt"));
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let ty = T::Nominal(owner("Capability"));
    let path = [MemberSegment::Field("labels"), MemberSegment::Index(1)];
    let request = MemberSelector {
        schema: MEMBER_SELECTOR_SCHEMA,
        root: SelectorRoot::Type(&ty),
        path: &path,
        expected_type: &T::String,
    };
    let identity = boundary.project_identity();
    let (result, count) = observe(None, || boundary.select(&request, &cancel));
    assert_eq!(result.unwrap().obligations().len(), 1);
    assert_ne!(count, 0);
    for index in 0..count {
        let (result, _) = observe(Some(index), || boundary.select(&request, &cancel));
        assert_eq!(result.unwrap_err(), ConsumerError::Limit);
        let signal = CancellationToken::new();
        let (result, _) =
            observe_cancellation(index, &signal, || boundary.select(&request, &signal));
        assert_eq!(result.unwrap_err(), ConsumerError::Cancelled);
        assert_eq!(
            boundary
                .select(&request, &cancel)
                .unwrap()
                .obligations()
                .len(),
            1
        );
    }
    assert_eq!(boundary.project_identity(), identity);
    for control in 0..6 {
        let defaults = ConsumerLimits::default();
        let run = |limit| {
            let mut limits = defaults;
            match control {
                0 => limits.bytes = limit,
                1 => limits.text_bytes = limit,
                2 => limits.path_segments = limit,
                3 => limits.work = limit,
                4 => limits.output_bytes = limit,
                _ => limits.locations = limit,
            }
            let b = ConsumerBoundary::new(&project, protocol(&contract), limits, &cancel)?;
            b.select(&request, &cancel)?;
            if control == 5 {
                b.value_origin(
                    &ValueOriginRequest {
                        schema: VALUE_ORIGIN_SCHEMA,
                        symbol: &owner("description"),
                        path: &path,
                    },
                    &cancel,
                )?;
            }
            Ok::<_, ConsumerError>(())
        };
        let mut low = 0;
        let mut high = match control {
            0 => defaults.bytes,
            1 => defaults.text_bytes,
            2 => defaults.path_segments,
            3 => defaults.work,
            4 => defaults.output_bytes,
            _ => defaults.locations,
        };
        while high - low > 1 {
            let mid = low + (high - low) / 2;
            if run(mid).is_ok() {
                high = mid;
            } else {
                low = mid;
            }
        }
        assert_eq!(run(high), Ok(()), "exact control {control}");
        assert_eq!(
            run(high - 1),
            Err(ConsumerError::Limit),
            "one-over control {control}"
        );
    }
}
