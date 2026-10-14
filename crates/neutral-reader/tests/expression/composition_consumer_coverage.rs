// SPDX-License-Identifier: Apache-2.0

//! Exact consumer negotiation, entry closure and reports using independent complete artifacts.

use super::*;
use crate::composition::ValidatedCompositionProject;
use crate::consumer::*;
use neutral_ir::composition::{
    profile as cp,
    project::{CompositionDeclaration, CompositionProjectIr},
};

/// Converts materialized fixture data into the separately checked composition authority.
fn authority(ir: ExpressionProjectIr) -> ValidatedCompositionProject {
    let ir = prepare(ir);
    let mut raw = CompositionProjectIr {
        schema: cp::PROJECT_IR_SCHEMA.into(),
        modules: ir.modules,
        declarations: ir
            .declarations
            .into_iter()
            .map(|d| CompositionDeclaration {
                identity: d.identity,
                public: d.public,
                signature: d.signature,
                value: match d.body {
                    Some(ExpressionBody::Value(value)) => Some(value),
                    None => None,
                    _ => panic!("materialized fixture required"),
                },
            })
            .collect(),
        vocabularies: ir.vocabularies,
        interface_digest: ir.interface_digest,
        sources: ir.sources,
        source_maps: ir.source_maps,
        provenance: ir.provenance,
        limits: ir.limits,
        resources: ir.resources,
        vocabulary_sources: ir.vocabulary_sources,
        origins: ir.origins,
        composition_limits: ir.composition_limits,
        composition_resources: ir.composition_resources,
    };
    let cancel = CancellationToken::new();
    raw.interface_digest = SemanticDigest::from_raw_bytes(
        neutral_ir::project_identity::composition_interface(
            &raw,
            neutral_ir::project_identity::IdentityLimits {
                bytes: raw.limits.artifact_bytes,
                nodes: raw.limits.nodes,
            },
            &cancel,
        )
        .unwrap()
        .identity()
        .as_bytes(),
    );
    ValidatedCompositionProject::from_ir(
        Shared::try_new(raw).unwrap(),
        limits().project,
        limits().composition,
        &cancel,
    )
    .unwrap()
}

/// Supplies literal exact protocol capabilities, independent of discovery output.
fn compatibility() -> ConsumerCompatibilityRequest<'static> {
    ConsumerCompatibilityRequest {
        profile: "neutral.consumer/1",
        artifact: cp::COMPOSITION_PROFILE,
        vocabulary_schemas: &[],
        required_features: &[
            "entry-discovery-v1",
            "member-selectors-v1",
            "value-attribution-v1",
        ],
        authoring_profile: None,
    }
}

/// Registers a public report contract with exact consumer codes.
fn protocol(contract: &T) -> ConsumerProtocol<'_> {
    ConsumerProtocol {
        report_contract: contract,
        supported_features: &[],
        diagnostic_codes: &["APP-001", "APP-002"],
    }
}

/// Checks actual reader authority against the exact negotiated tuple without compiler activation.
#[test]
fn negotiated_consumer_validates_actual_complete_authority() {
    let project = authority(default_fixture(loc("m", 20, 21)));
    let cancel = CancellationToken::new();
    let request = compatibility();
    let supported = negotiate_consumer(&request, ConsumerLimits::default(), &cancel).unwrap();
    assert_eq!(supported.profile, "neutral.consumer/1");
    assert_eq!(
        supported.features,
        [
            "entry-discovery-v1",
            "member-selectors-v1",
            "value-attribution-v1"
        ]
    );
    assert_eq!(supported.vocabulary_schemas, ["1.0", "2.0"]);
    assert_eq!(supported.authoring_profile, None);
    assert_eq!(consumer_compatibilities().len(), 1);
    assert!(std::ptr::eq(
        supported,
        &raw const consumer_compatibilities()[0]
    ));
    assert_eq!(
        supported.validate_project(&project, &request, ConsumerLimits::default(), &cancel),
        Ok(())
    );
    let fabricated = ConsumerCompatibility {
        profile: "unsupported",
        ..*supported
    };
    assert_eq!(
        fabricated.validate_project(&project, &request, ConsumerLimits::default(), &cancel),
        Err(ConsumerError::Unsupported)
    );
    let vocabulary = authority(super::public_projection_coverage::vocabulary_fixture());
    assert_eq!(
        supported.validate_project(&vocabulary, &request, ConsumerLimits::default(), &cancel),
        Err(ConsumerError::Unsupported)
    );
    assert_eq!(
        supported.validate_project(
            &vocabulary,
            &ConsumerCompatibilityRequest {
                vocabulary_schemas: &["2.0"],
                ..request
            },
            ConsumerLimits::default(),
            &cancel
        ),
        Ok(())
    );
}

/// Missing, extra and malformed features cannot be interpreted as optional negotiation hints.
#[test]
fn negotiation_rejects_inexact_capabilities_and_independent_limits() {
    let good = compatibility();
    let cancel = CancellationToken::new();
    for (features, expected) in [
        (&[][..], ConsumerError::Unsupported),
        (&["entry-discovery-v1"][..], ConsumerError::Unsupported),
        (
            &[
                "entry-discovery-v1",
                "member-selectors-v1",
                "unknown",
                "value-attribution-v1",
            ][..],
            ConsumerError::Unsupported,
        ),
        (
            &["entry-discovery-v1", "entry-discovery-v1"][..],
            ConsumerError::InvalidInput,
        ),
        (
            &["value-attribution-v1", "entry-discovery-v1"][..],
            ConsumerError::InvalidInput,
        ),
    ] {
        assert_eq!(
            negotiate_consumer(
                &ConsumerCompatibilityRequest {
                    required_features: features,
                    ..good
                },
                ConsumerLimits::default(),
                &cancel
            )
            .unwrap_err(),
            expected
        );
    }
    for request in [
        ConsumerCompatibilityRequest {
            profile: "neutral.consumer/2",
            ..good
        },
        ConsumerCompatibilityRequest {
            authoring_profile: Some("authoring"),
            ..good
        },
        ConsumerCompatibilityRequest {
            vocabulary_schemas: &["3.0"],
            ..good
        },
        ConsumerCompatibilityRequest {
            artifact: cp::CompositionProfileDescriptor {
                encoding: "NIR-PROJECT-CBOR/1",
                ..good.artifact
            },
            ..good
        },
    ] {
        assert_eq!(
            negotiate_consumer(&request, ConsumerLimits::default(), &cancel),
            Err(ConsumerError::Unsupported)
        );
    }
    assert_eq!(
        negotiate_consumer(
            &good,
            ConsumerLimits {
                features: 2,
                ..Default::default()
            },
            &cancel
        ),
        Err(ConsumerError::Limit)
    );
    cancel.cancel();
    assert_eq!(
        negotiate_consumer(&good, ConsumerLimits::default(), &cancel),
        Err(ConsumerError::Cancelled)
    );
}

/// Retains two independently mapped public bindings of the same exact nominal contract.
fn two_entry_fixture() -> ExpressionProjectIr {
    let mut ir = default_fixture(loc("m", 20, 21));
    let mut b = default_fixture(loc("m", 20, 21));
    b.declarations[1].identity = symbol("m", "b");
    b.source_maps[1].declaration = symbol("m", "b");
    b.origins
        .iter_mut()
        .for_each(|origin| origin.binding = symbol("m", "b"));
    b.provenance[0].from = symbol("m", "b");
    ir.declarations.push(b.declarations.remove(1));
    ir.source_maps.push(b.source_maps.remove(1));
    ir.origins.extend(b.origins);
    ir.provenance.extend(b.provenance);
    ir
}

/// Explicit roots are canonicalized and carry nominal interpretation closure into the public view.
#[test]
fn composition_entries_canonicalize_roots_and_retain_contract_closure() {
    let project = authority(two_entry_fixture());
    let contract = T::Nominal(symbol("m", "Record"));
    let cancel = CancellationToken::new();
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let roots = [symbol("m", "b"), symbol("m", "a")];
    let request = EntryRequest {
        schema: "neutral.entry-request/1",
        contract: &contract,
        selection: EntrySelection::Explicit(&roots),
    };
    let selected = boundary.entries(&request, &cancel).unwrap();
    assert_eq!(
        selected
            .bindings()
            .iter()
            .map(|b| b.owner.declaration_name())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    let debug = format!("{selected:?}");
    assert!(!debug.contains("Record"));
    assert!(!debug.contains("source:m"));
    let view = selected.derive_view(&cancel).unwrap();
    assert_eq!(
        view.declarations()
            .iter()
            .map(|d| d.identity.declaration_name())
            .collect::<Vec<_>>(),
        ["Record", "a", "b"]
    );
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
                    selection: EntrySelection::Enumerate,
                    ..request
                },
                &cancel
            )
            .unwrap()
            .bindings()
            .len(),
        2
    );
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
            .derive_view(&cancel)
            .unwrap()
            .declarations()
            .len(),
        0
    );
    let duplicate = [symbol("m", "a"), symbol("m", "a")];
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
    cancel.cancel();
    assert_eq!(
        selected.derive_view(&cancel).unwrap_err(),
        ConsumerError::Cancelled
    );
}

/// Public selection fails closed for schema, nominal mismatch, private roots and root storage limits.
#[test]
fn composition_entry_rejections_do_not_disclose_private_values() {
    let mut ir = default_fixture(loc("m", 20, 21));
    scalar(&mut ir, "m", "number", 100);
    scalar(&mut ir, "m", "private_secret", 120);
    ir.declarations.last_mut().unwrap().public = false;
    let project = authority(ir);
    let contract = T::Nominal(symbol("m", "Record"));
    let cancel = CancellationToken::new();
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    for (schema, root, expected) in [
        ("unknown", "a", ConsumerError::Unsupported),
        (
            "neutral.entry-request/1",
            "number",
            ConsumerError::TypeMismatch,
        ),
        (
            "neutral.entry-request/1",
            "private_secret",
            ConsumerError::PrivateTarget,
        ),
        (
            "neutral.entry-request/1",
            "absent",
            ConsumerError::PrivateTarget,
        ),
    ] {
        let roots = [symbol("m", root)];
        let error = boundary
            .entries(
                &EntryRequest {
                    schema,
                    contract: &contract,
                    selection: EntrySelection::Explicit(&roots),
                },
                &cancel,
            )
            .unwrap_err();
        assert_eq!(error, expected);
        assert!(error.to_string().starts_with("NEU-CONS-"));
        assert!(!error.to_string().contains(root));
    }
    let selected = boundary
        .entries(
            &EntryRequest {
                schema: ENTRY_REQUEST_SCHEMA,
                contract: &contract,
                selection: EntrySelection::Unique,
            },
            &cancel,
        )
        .unwrap();
    assert_eq!(selected.bindings()[0].owner, symbol("m", "a"));
    let constrained = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits {
            output_bytes: 1,
            ..Default::default()
        },
        &cancel,
    )
    .unwrap();
    assert_eq!(
        constrained
            .entries(
                &EntryRequest {
                    schema: ENTRY_REQUEST_SCHEMA,
                    contract: &contract,
                    selection: EntrySelection::Unique
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::Limit
    );
}

/// Checked report accessors expose only validated context, decision and canonical diagnostics.
#[test]
fn descriptions_and_reports_preserve_explicit_decisions_and_public_context() {
    let project = authority(default_fixture(loc("m", 20, 21)));
    let contract = T::Nominal(symbol("m", "Record"));
    let entry = symbol("m", "a");
    let cancel = CancellationToken::new();
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let description = DescriptionRequest {
        schema: DESCRIPTION_SCHEMA,
        contract: &contract,
        adapter_revision: "adapter-1",
        target_revision: "target-1",
        required_features: &[],
        payload: &entry,
    };
    let accepted = boundary.description(&description, &cancel).unwrap();
    assert_eq!(accepted.payload().owner, entry);
    assert!(std::ptr::eq(accepted.request(), &raw const description));
    assert!(!format!("{accepted:?}").contains("source:m"));
    let diagnostics = [
        ConsumerDiagnostic {
            code: ConsumerDiagnosticCode::Consumer("APP-002"),
            subject: &entry,
            path: &[],
            primary: ConsumerLocation::Unavailable(LocationUnavailable::NotRetained),
            related: &[],
        },
        ConsumerDiagnostic {
            code: ConsumerDiagnosticCode::ReaderContract(ConsumerError::MissingMember),
            subject: &entry,
            path: &[],
            primary: ConsumerLocation::Unavailable(LocationUnavailable::Redacted),
            related: &[],
        },
    ];
    for (status, accepted) in [
        (ConsumerStatus::Accepted, true),
        (ConsumerStatus::Rejected, false),
        (ConsumerStatus::AlternativeRequired, false),
    ] {
        let request = ReportRequest {
            schema: REPORT_SCHEMA,
            contract: &contract,
            project_identity: boundary.project_identity(),
            entry: Some(&entry),
            context: ConsumerContext {
                adapter_revision: "adapter-1",
                target_revision: "target-1",
                input_revision: "input-1",
            },
            status,
            diagnostics: &diagnostics,
        };
        let report = boundary.report(&request, &cancel).unwrap();
        assert_eq!(report.is_accepted(), accepted);
        assert_eq!(report.status(), status);
        assert_eq!(report.contract(), &contract);
        assert_eq!(report.entry(), Some(&entry));
        assert_eq!(report.project_identity(), boundary.project_identity());
        assert_eq!(report.context().input_revision, "input-1");
        assert_eq!(
            report
                .diagnostics()
                .iter()
                .map(|d| (d.code.phase(), d.code.code()))
                .collect::<Vec<_>>(),
            [("consumer", "APP-002"), ("reader-contract", "NEU-CONS-003")]
        );
        assert!(!format!("{report:?}").contains("source:m"));
    }
}
