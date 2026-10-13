// SPDX-License-Identifier: Apache-2.0

//! Closed consumer envelopes over real compiled and independently decoded projects.

use super::*;
use neutral_ir::{
    LogicalModuleIdentity, ModuleSymbolIdentity, project_interface::ProjectPublicType as T,
};
use neutral_reader::consumer::*;

#[test]
fn security_consumer_cannot_relax_effective_reader_work_policy() {
    let project = project();
    let cancel = CancellationToken::new();
    let caller = CompositionLimits {
        work: 50_000,
        ..limits_for(&source_case(""))
    };
    let restricted = ValidatedCompositionProject::from_ir(
        Arc::clone(project.complete_ir()),
        project.complete_ir().limits,
        caller,
        &cancel,
    )
    .unwrap();
    let contract = T::Nominal(owner("Receipt"));
    let boundary = ConsumerBoundary::new(
        &restricted,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let subject = owner("description");
    let diagnostic = ConsumerDiagnostic {
        code: ConsumerDiagnosticCode::Consumer("EXAMPLE-UNSUPPORTED"),
        subject: &subject,
        path: &[],
        primary: ConsumerLocation::Unavailable(LocationUnavailable::NotRetained),
        related: &[],
    };
    let diagnostics = vec![diagnostic; 256];
    let small = rejected_report(&contract, boundary.project_identity(), &diagnostics[..1]);
    assert_eq!(
        boundary
            .report(&small, &cancel)
            .unwrap()
            .diagnostics()
            .len(),
        1
    );
    let large = rejected_report(&contract, boundary.project_identity(), &diagnostics);
    report_error(&boundary, large, &cancel, ConsumerError::Limit);
}

#[test]
fn integration_consumer_vocabulary_contracts_require_exact_locked_revision() {
    let cases: Value = serde_json::from_str(REQUESTS[0]).unwrap();
    let project = compile(&cases["cases"][0]);
    let project = roundtrip(&project, &CancellationToken::new());
    let contract = T::VocabularyNominal {
        identity: "ExampleDomain".into(),
        version: "1.0.0".into(),
        name: "Request".into(),
    };
    let payload = owner("request");
    let cancel = CancellationToken::new();
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let request = DescriptionRequest {
        schema: DESCRIPTION_SCHEMA,
        contract: &contract,
        adapter_revision: "adapter-1",
        target_revision: "target-1",
        required_features: &[],
        payload: &payload,
    };
    assert_eq!(
        &boundary
            .description(&request, &cancel)
            .unwrap()
            .payload()
            .ty,
        &contract
    );
    let stale = T::VocabularyNominal {
        identity: "ExampleDomain".into(),
        version: "2.0.0".into(),
        name: "Request".into(),
    };
    assert_eq!(
        boundary
            .description(
                &DescriptionRequest {
                    contract: &stale,
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::PrivateTarget
    );
    let different = T::VocabularyNominal {
        identity: "ExampleDomain".into(),
        version: "1.0.0".into(),
        name: "Outcome".into(),
    };
    assert_eq!(
        boundary
            .description(
                &DescriptionRequest {
                    contract: &different,
                    ..request
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::TypeMismatch
    );
}

#[test]
fn security_consumer_public_reuse_cannot_disclose_private_origin() {
    let project = project();
    let contract = T::Nominal(owner("Receipt"));
    let subject = owner("copied");
    let path = [neutral_ir::composition::ValuePathSegment::Field(
        "kind".into(),
    )];
    let location = source_location(&project, &subject, &path);
    let cancel = CancellationToken::new();
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let diagnostics = [ConsumerDiagnostic {
        code: ConsumerDiagnosticCode::Consumer("EXAMPLE-UNSUPPORTED"),
        subject: &subject,
        path: &path,
        primary: ConsumerLocation::Available {
            subject: &subject,
            path: &path,
            location,
        },
        related: &[],
    }];
    let request = rejected_report(&contract, boundary.project_identity(), &diagnostics);
    report_error(
        &boundary,
        request,
        &cancel,
        ConsumerError::InvalidAttribution,
    );
    let redacted = [ConsumerDiagnostic {
        primary: ConsumerLocation::Unavailable(LocationUnavailable::Redacted),
        ..diagnostics[0]
    }];
    assert_eq!(
        boundary
            .report(
                &rejected_report(&contract, boundary.project_identity(), &redacted),
                &cancel
            )
            .unwrap()
            .diagnostics()
            .len(),
        1
    );
}

/// Independently decodes bytes and checks the complete round trip before envelope testing.
fn roundtrip(
    project: &ValidatedCompositionProject,
    cancel: &CancellationToken,
) -> ValidatedCompositionProject {
    let bytes = encode_composition_project(project, cancel).unwrap();
    let decoded = decode_composition_project(
        &bytes,
        DecodeLimits::hard(),
        project.complete_ir().limits,
        limits_for(&source_case("")),
        cancel,
    )
    .unwrap();
    assert_eq!(encode_composition_project(&decoded, cancel).unwrap(), bytes);
    decoded
}

/// Supplies an explicit rejected context; individual tests override only their changed input.
fn rejected_report<'a>(
    contract: &'a T,
    identity: neutral_reader::composition::CompositionLogicalIdentity,
    diagnostics: &'a [ConsumerDiagnostic<'a>],
) -> ReportRequest<'a> {
    ReportRequest {
        schema: REPORT_SCHEMA,
        contract,
        project_identity: identity,
        entry: None,
        context: context(),
        status: ConsumerStatus::Rejected,
        diagnostics,
    }
}

/// Asserts rejection of a complete envelope, without hiding the expected classification.
fn report_error(
    boundary: &ConsumerBoundary<'_, '_>,
    request: ReportRequest<'_>,
    cancel: &CancellationToken,
    expected: ConsumerError,
) {
    assert_eq!(boundary.report(&request, cancel).unwrap_err(), expected);
}

/// Retrieves actual reader-checked source evidence, never inventing a test location.
fn source_location(
    project: &ValidatedCompositionProject,
    subject: &ModuleSymbolIdentity,
    path: &[neutral_ir::composition::ValuePathSegment],
) -> neutral_core::SourceLocation {
    project
        .complete_ir()
        .origins
        .iter()
        .find_map(|origin| {
            if &origin.binding == subject
                && origin.path == path
                && let Some(neutral_reader::composition::CompositionAttribution::Source(location)) =
                    origin.attribution
            {
                return Some(location);
            }
            None
        })
        .unwrap()
}

#[test]
fn integration_consumer_decoded_evidence_requires_exact_public_occurrence() {
    let case = source_case(include_str!("consumer.neu"));
    let project = compile(&case);
    let cancel = CancellationToken::new();
    let decoded = roundtrip(&project, &cancel);
    let contract = T::Nominal(owner("Receipt"));
    let boundary = ConsumerBoundary::new(
        &decoded,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let original = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    assert_eq!(boundary.project_identity(), original.project_identity());
    let subject = owner("description");
    let other = owner("entry");
    let path = [neutral_ir::composition::ValuePathSegment::Field(
        "kind".into(),
    )];
    let location = source_location(&decoded, &subject, &path);
    let diagnostic = ConsumerDiagnostic {
        code: ConsumerDiagnosticCode::Consumer("EXAMPLE-UNSUPPORTED"),
        subject: &subject,
        path: &path,
        primary: ConsumerLocation::Available {
            subject: &subject,
            path: &path,
            location,
        },
        related: &[],
    };
    let diagnostics = [diagnostic];
    let request = rejected_report(&contract, boundary.project_identity(), &diagnostics);
    assert_eq!(
        boundary
            .report(&request, &cancel)
            .unwrap()
            .diagnostics()
            .len(),
        1
    );
    let wrong = [ConsumerDiagnostic {
        primary: ConsumerLocation::Available {
            subject: &other,
            path: &path,
            location,
        },
        ..diagnostic
    }];
    report_error(
        &boundary,
        ReportRequest {
            diagnostics: &wrong,
            ..request
        },
        &cancel,
        ConsumerError::InvalidAttribution,
    );
    let unrelated = [ConsumerLocation::Available {
        subject: &other,
        path: &path,
        location,
    }];
    let wrong_related = [ConsumerDiagnostic {
        related: &unrelated,
        ..diagnostic
    }];
    report_error(
        &boundary,
        ReportRequest {
            diagnostics: &wrong_related,
            ..request
        },
        &cancel,
        ConsumerError::InvalidAttribution,
    );
    let absent = [neutral_ir::composition::ValuePathSegment::Field(
        "absent".into(),
    )];
    let wrong_path = [ConsumerDiagnostic {
        path: &absent,
        primary: ConsumerLocation::Unavailable(LocationUnavailable::NotRetained),
        ..diagnostic
    }];
    report_error(
        &boundary,
        ReportRequest {
            diagnostics: &wrong_path,
            ..request
        },
        &cancel,
        ConsumerError::MissingMember,
    );
}

#[test]
fn security_consumer_registration_and_envelopes_fail_closed() {
    let project = project();
    let contract = T::Nominal(owner("Receipt"));
    let private = T::Nominal(owner("private_description"));
    let cancel = CancellationToken::new();
    for registration in [
        ConsumerProtocol {
            supported_features: &["join", "emit"],
            ..protocol(&contract)
        },
        ConsumerProtocol {
            diagnostic_codes: &["NEU-COMP-001"],
            ..protocol(&contract)
        },
        ConsumerProtocol {
            diagnostic_codes: &[""],
            ..protocol(&contract)
        },
    ] {
        assert_eq!(
            ConsumerBoundary::new(&project, registration, ConsumerLimits::default(), &cancel).err(),
            Some(ConsumerError::InvalidInput)
        );
    }
    assert_eq!(
        ConsumerBoundary::new(
            &project,
            protocol(&private),
            ConsumerLimits::default(),
            &cancel
        )
        .err(),
        Some(ConsumerError::PrivateTarget)
    );
    assert_eq!(
        ConsumerBoundary::new(
            &project,
            protocol(&T::Num),
            ConsumerLimits::default(),
            &cancel
        )
        .err(),
        Some(ConsumerError::TypeMismatch)
    );
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let entry = owner("private_description");
    let request = ReportRequest {
        schema: REPORT_SCHEMA,
        contract: &contract,
        project_identity: boundary.project_identity(),
        entry: Some(&entry),
        context: context(),
        status: ConsumerStatus::Accepted,
        diagnostics: &[],
    };
    assert_eq!(
        boundary.report(&request, &cancel).unwrap_err(),
        ConsumerError::PrivateTarget
    );
    let unknown = ReportRequest {
        schema: "neutral.consumer-report/future",
        entry: None,
        ..request
    };
    assert_eq!(
        boundary.report(&unknown, &cancel).unwrap_err(),
        ConsumerError::Unsupported
    );
    let invalid = ReportRequest {
        entry: None,
        context: ConsumerContext {
            input_revision: "secret\ntext",
            ..context()
        },
        ..request
    };
    assert_eq!(
        boundary.report(&invalid, &cancel).unwrap_err(),
        ConsumerError::InvalidInput
    );
    assert_eq!(ConsumerError::Limit.code(), "NEU-CONS-010");
    assert_eq!(
        ConsumerDiagnosticCode::ReaderContract(ConsumerError::InvalidInput).phase(),
        "reader-contract"
    );
}

#[test]
fn security_consumer_allocation_and_cancellation_sweeps_preserve_reader() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    let project = project();
    let contract = T::Nominal(owner("Receipt"));
    let cancel = CancellationToken::new();
    let (initial, count) = observe(None, || {
        ConsumerBoundary::new(
            &project,
            protocol(&contract),
            ConsumerLimits::default(),
            &cancel,
        )
    });
    let boundary = initial.unwrap();
    assert_ne!(count, 0);
    for index in 0..count {
        let (result, _) = observe(Some(index), || {
            ConsumerBoundary::new(
                &project,
                protocol(&contract),
                ConsumerLimits::default(),
                &cancel,
            )
        });
        assert_eq!(
            result.err(),
            Some(ConsumerError::Limit),
            "allocation {index}"
        );
        let signal = CancellationToken::new();
        let (result, _) = observe_cancellation(index, &signal, || {
            ConsumerBoundary::new(
                &project,
                protocol(&contract),
                ConsumerLimits::default(),
                &signal,
            )
        });
        assert_eq!(
            result.err(),
            Some(ConsumerError::Cancelled),
            "cancellation {index}"
        );
    }
    let recovered = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    assert_eq!(recovered.project_identity(), boundary.project_identity());
}

#[test]
fn security_consumer_independent_limits_reject_at_acceptance_boundary() {
    let project = project();
    let contract = T::Nominal(owner("Receipt"));
    let subject = owner("description");
    let path = [
        neutral_ir::composition::ValuePathSegment::Field("labels".into()),
        neutral_ir::composition::ValuePathSegment::Element(0),
    ];
    let cancel = CancellationToken::new();
    let diagnostic = ConsumerDiagnostic {
        code: ConsumerDiagnosticCode::Consumer("EXAMPLE-UNSUPPORTED"),
        subject: &subject,
        path: &path,
        primary: ConsumerLocation::Unavailable(LocationUnavailable::NotRetained),
        related: &[],
    };
    let diagnostics = [diagnostic];
    let defaults = ConsumerLimits::default();
    for control in 0..8 {
        let policy = |value| {
            let mut limits = defaults;
            match control {
                0 => limits.bytes = value,
                1 => limits.text_bytes = value,
                2 => limits.features = value,
                3 => limits.diagnostics = value,
                4 => limits.locations = value,
                5 => limits.path_segments = value,
                6 => limits.work = value,
                7 => limits.output_bytes = value,
                _ => unreachable!(),
            }
            limits
        };
        let run = |value| -> Result<(), ConsumerError> {
            let boundary =
                ConsumerBoundary::new(&project, protocol(&contract), policy(value), &cancel)?;
            let request = ReportRequest {
                schema: REPORT_SCHEMA,
                contract: &contract,
                project_identity: boundary.project_identity(),
                entry: None,
                context: context(),
                status: ConsumerStatus::Rejected,
                diagnostics: &diagnostics,
            };
            boundary.report(&request, &cancel)?;
            Ok(())
        };
        let mut low = 0;
        let mut high = match control {
            0 => defaults.bytes,
            1 => defaults.text_bytes,
            2 => defaults.features,
            3 => defaults.diagnostics,
            4 => defaults.locations,
            5 => defaults.path_segments,
            6 => defaults.work,
            _ => defaults.output_bytes,
        };
        assert_eq!(run(high), Ok(()));
        while high - low > 1 {
            let middle = low + (high - low) / 2;
            if run(middle).is_ok() {
                high = middle;
            } else {
                low = middle;
            }
        }
        assert_eq!(run(high), Ok(()), "exact control {control}");
        assert_eq!(run(high + 1), Ok(()), "below control {control}");
        assert!(run(high - 1).is_err(), "over control {control}");
    }
}

fn owner(name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(LogicalModuleIdentity::new("1.0", "example"), name)
}

fn project() -> ValidatedCompositionProject {
    compile(&source_case(include_str!("consumer.neu")))
}

fn protocol(contract: &T) -> ConsumerProtocol<'_> {
    ConsumerProtocol {
        report_contract: contract,
        supported_features: &["emit", "join"],
        diagnostic_codes: &["EXAMPLE-UNSUPPORTED"],
    }
}

fn context() -> ConsumerContext<'static> {
    ConsumerContext {
        adapter_revision: "adapter-1",
        target_revision: "target-1",
        input_revision: "inputs-1",
    }
}

#[test]
fn integration_consumer_description_enforces_exact_contract_features_and_visibility() {
    let project = project();
    let report_contract = T::Nominal(owner("Receipt"));
    let contract = T::Nominal(owner("Capability"));
    let payload = owner("description");
    let cancel = CancellationToken::new();
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&report_contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let mut request = DescriptionRequest {
        schema: DESCRIPTION_SCHEMA,
        contract: &contract,
        adapter_revision: "adapter-1",
        target_revision: "target-1",
        required_features: &["emit"],
        payload: &payload,
    };
    let description = boundary.description(&request, &cancel).unwrap();
    assert_eq!(description.payload().owner, payload);
    request.contract = &report_contract;
    assert_eq!(
        boundary.description(&request, &cancel).unwrap_err(),
        ConsumerError::TypeMismatch
    );
    request.contract = &contract;
    request.required_features = &["missing"];
    assert_eq!(
        boundary.description(&request, &cancel).unwrap_err(),
        ConsumerError::Unsupported
    );
    request.required_features = &["emit", "emit"];
    assert_eq!(
        boundary.description(&request, &cancel).unwrap_err(),
        ConsumerError::InvalidInput
    );
    request.required_features = &["emit"];
    let private = owner("private_description");
    request.payload = &private;
    assert_eq!(
        boundary.description(&request, &cancel).unwrap_err(),
        ConsumerError::PrivateTarget
    );
    let absent = owner("absent");
    request.payload = &absent;
    assert_eq!(
        boundary.description(&request, &cancel).unwrap_err(),
        ConsumerError::PrivateTarget
    );
}

#[test]
fn integration_consumer_reports_bind_real_identity_context_and_do_not_accept_alternatives() {
    let project = project();
    let contract = T::Nominal(owner("Receipt"));
    let cancel = CancellationToken::new();
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let logical = boundary.project_identity();
    let entry = owner("entry");
    let mut request = ReportRequest {
        schema: REPORT_SCHEMA,
        contract: &contract,
        project_identity: logical,
        entry: Some(&entry),
        context: context(),
        status: ConsumerStatus::Accepted,
        diagnostics: &[],
    };
    assert!(boundary.report(&request, &cancel).unwrap().is_accepted());
    request.status = ConsumerStatus::AlternativeRequired;
    assert!(!boundary.report(&request, &cancel).unwrap().is_accepted());
    request.status = ConsumerStatus::Rejected;
    assert!(!boundary.report(&request, &cancel).unwrap().is_accepted());
    request.context.input_revision = "inputs-2";
    assert_eq!(
        boundary
            .report(&request, &cancel)
            .unwrap()
            .project_identity(),
        logical
    );
    let other = compile(&source_case(
        &include_str!("consumer.neu").replace("\"emit\"", "\"other\""),
    ));
    request.project_identity = ConsumerBoundary::new(
        &other,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap()
    .project_identity();
    assert_eq!(
        boundary.report(&request, &cancel).unwrap_err(),
        ConsumerError::InvalidInput
    );
    request.project_identity = logical;
    request.context.target_revision = "";
    assert_eq!(
        boundary.report(&request, &cancel).unwrap_err(),
        ConsumerError::InvalidInput
    );
}

#[test]
fn integration_consumer_report_sorts_diagnostics_and_rejects_impersonation_and_private_evidence() {
    let project = project();
    let contract = T::Nominal(owner("Receipt"));
    let cancel = CancellationToken::new();
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let first = owner("description");
    let last = owner("entry");
    let unavailable = ConsumerLocation::Unavailable(LocationUnavailable::NotRetained);
    let diagnostic = |subject| ConsumerDiagnostic {
        code: ConsumerDiagnosticCode::Consumer("EXAMPLE-UNSUPPORTED"),
        subject,
        path: &[],
        primary: unavailable,
        related: &[],
    };
    let diagnostics = [diagnostic(&last), diagnostic(&first)];
    let envelope = ReportRequest {
        schema: REPORT_SCHEMA,
        contract: &contract,
        project_identity: boundary.project_identity(),
        entry: None,
        context: context(),
        status: ConsumerStatus::Rejected,
        diagnostics: &diagnostics,
    };
    let report = boundary.report(&envelope, &cancel).unwrap();
    assert_eq!(report.diagnostics()[0].subject, &first);
    assert_eq!(report.diagnostics()[1].subject, &last);
    let impersonation = [ConsumerDiagnostic {
        code: ConsumerDiagnosticCode::Consumer("NEU-COMP-001"),
        ..diagnostic(&last)
    }];
    assert_eq!(
        boundary
            .report(
                &ReportRequest {
                    diagnostics: &impersonation,
                    ..envelope
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::InvalidInput
    );
    let private = owner("private_description");
    let location = project
        .complete_ir()
        .source_maps
        .iter()
        .find(|m| m.declaration == private)
        .unwrap()
        .location;
    let private_evidence = [ConsumerDiagnostic {
        primary: ConsumerLocation::Available {
            subject: &private,
            path: &[],
            location,
        },
        ..diagnostic(&last)
    }];
    assert_eq!(
        boundary
            .report(
                &ReportRequest {
                    diagnostics: &private_evidence,
                    ..envelope
                },
                &cancel
            )
            .unwrap_err(),
        ConsumerError::InvalidAttribution
    );
}

#[test]
fn integration_consumer_bounds_cancellation_and_allocation_fail_atomically() {
    let project = project();
    let contract = T::Nominal(owner("Receipt"));
    let cancel = CancellationToken::new();
    let boundary = ConsumerBoundary::new(
        &project,
        protocol(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let subject = owner("description");
    let diagnostic = ConsumerDiagnostic {
        code: ConsumerDiagnosticCode::Consumer("EXAMPLE-UNSUPPORTED"),
        subject: &subject,
        path: &[],
        primary: ConsumerLocation::Unavailable(LocationUnavailable::Redacted),
        related: &[],
    };
    let diagnostics = [diagnostic, diagnostic];
    let request = ReportRequest {
        schema: REPORT_SCHEMA,
        contract: &contract,
        project_identity: boundary.project_identity(),
        entry: None,
        context: context(),
        status: ConsumerStatus::Rejected,
        diagnostics: &diagnostics[..1],
    };
    let limits = ConsumerLimits {
        diagnostics: 1,
        ..ConsumerLimits::default()
    };
    let small = ConsumerBoundary::new(&project, protocol(&contract), limits, &cancel).unwrap();
    assert_eq!(
        small.report(&request, &cancel).unwrap().diagnostics().len(),
        1
    );
    let over = ReportRequest {
        diagnostics: &diagnostics,
        ..request
    };
    assert_eq!(
        small.report(&over, &cancel).unwrap_err(),
        ConsumerError::Limit
    );
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert_eq!(
        boundary.report(&request, &cancelled).unwrap_err(),
        ConsumerError::Cancelled
    );
    let observed =
        neutral_core::allocation::testing::observe(Some(0), || boundary.report(&request, &cancel));
    assert_eq!(observed.0.unwrap_err(), ConsumerError::Limit);
    assert_eq!(
        boundary
            .report(&request, &cancel)
            .unwrap()
            .diagnostics()
            .len(),
        1
    );
}
