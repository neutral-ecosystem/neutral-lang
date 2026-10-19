// SPDX-License-Identifier: Apache-2.0

//! Explicit generations over a captured operation/result/prerequisite vocabulary, without scheduling.

use super::*;
use neutral_reader::consumer::{
    ConsumerError, ConsumerLocation, LocationUnavailable, expression::ExpressionConsumerError as E,
};

/// Independently decodes the shared file-backed consumer model before any external values arrive.
fn project() -> ValidatedExpressionProject {
    vocabulary_project(
        include_bytes!("../fixtures/consumer.neu"),
        include_bytes!("../fixtures/consumer-domain.json"),
    )
}

/// Literal fixture contract, independent of package/tool versions and producer implementation.
fn contract(name: &str) -> T {
    T::VocabularyNominal {
        identity: "PipelineDomain".into(),
        version: "1.0.0".into(),
        name: name.into(),
    }
}

/// Uses the complete decoded semantic profile and only explicit caller envelopes.
fn request<'a>(
    project: &'a ValidatedExpressionProject,
    selected: &'a ModuleSymbolIdentity,
    bindings: &'a [Binding<'a>],
) -> EvaluationRequest<'a> {
    EvaluationRequest {
        selected,
        profiles: &project.ir().semantic_profiles,
        bindings,
        associations: &[],
        limits: ExpressionLimits::hard(),
    }
}

/// Consumer-owned prerequisite fields remain ordinary values, never implicit output demands.
#[test]
fn integration_prerequisites_do_not_become_guarded_value_edges() {
    let project = project();
    let cancel = CancellationToken::new();
    let (entry, enabled, producer, prepare) = (
        symbol("entry"),
        symbol("enabled"),
        symbol("producer"),
        symbol("prepare"),
    );
    let result = contract("Result");
    let selector = [
        SelectorSegment::Field("values".into()),
        SelectorSegment::Index(1),
    ];
    let dependencies = project
        .dependencies(&entry, ExpressionInspectionLimits::default(), &cancel)
        .unwrap();
    assert_eq!(dependencies.edges().len(), 2);
    assert_eq!(dependencies.edges()[0].key, BindingKey::Input(&enabled));
    assert_eq!(
        dependencies.edges()[1].key,
        BindingKey::Output {
            target: &producer,
            result_type: &result,
            selector: &selector,
        }
    );
    assert_eq!(
        dependencies.edges()[1].guards[0].condition,
        GuardCondition::Boolean(true)
    );
    let report = contract("Report");
    let plan = contract("Plan");
    let associations = [OutputAssociation {
        target: &producer,
        result_type: &result,
    }];
    let boundary = boundary(&project, &report, &associations);
    let off = V::Bool(false);
    let bindings = [Binding {
        key: BindingKey::Input(&enabled),
        ty: &T::Bool,
        state: BindingState::Value(&off),
    }];
    let outcome = boundary
        .evaluate(
            &request(&project, &entry, &bindings),
            context("generation"),
            &cancel,
        )
        .unwrap();
    assert_eq!(
        outcome.outcome,
        R::Value {
            ty: &plan,
            value: V::Record(vec![
                (
                    "prerequisites".into(),
                    Some(V::List(vec![V::Reference(prepare)]))
                ),
                (
                    "value".into(),
                    Some(V::Number(
                        ExactNumber::from_source("0.125", 128, 128).unwrap()
                    ))
                ),
            ])
        }
    );
}

/// Null is an explicit typed value; omission, unavailable and failed remain distinct safe reports.
#[test]
fn integration_nullable_binding_reports_distinguish_null_absence_unavailable_and_failed() {
    let project = project();
    let cancel = CancellationToken::new();
    let report = contract("Report");
    let boundary = boundary(&project, &report, &[]);
    let selected = symbol("nullable_value");
    let nullable = T::Nullable(Box::new(T::Num));
    let key = BindingKey::Input(&selected);
    let null = V::Null;
    for (state, expected) in [
        (None, R::RequiredBindings(vec![key])),
        (
            Some(BindingState::Value(&null)),
            R::Value {
                ty: &nullable,
                value: V::Null,
            },
        ),
        (Some(BindingState::Unavailable), R::Unavailable(key)),
        (
            Some(BindingState::Failed),
            R::Failure(EvaluationError::BindingFailed),
        ),
    ] {
        let bindings = state.map(|state| Binding {
            key,
            ty: &nullable,
            state,
        });
        let request = request(&project, &selected, bindings.as_slice());
        boundary.validate_bindings(&request, &cancel).unwrap();
        let report = boundary
            .evaluate(&request, context("nullable"), &cancel)
            .unwrap();
        assert_eq!(report.outcome, expected);
        assert!(matches!(
            report.attribution,
            ConsumerLocation::Unavailable(LocationUnavailable::NotRetained)
        ));
    }
}

/// Every malformed envelope rejects atomically, even when the selected entry never demands it.
#[test]
fn integration_invalid_skipped_bindings_reject_and_next_generation_recovers() {
    let project = project();
    let cancel = CancellationToken::new();
    let report = contract("Report");
    let boundary = boundary(&project, &report, &[]);
    let (selected, enabled, unknown, private, result_input) = (
        symbol("nullable_value"),
        symbol("enabled"),
        symbol("unknown"),
        symbol("private_number"),
        symbol("result_input"),
    );
    let nullable = T::Nullable(Box::new(T::Num));
    let null = V::Null;
    let good = Binding {
        key: BindingKey::Input(&selected),
        ty: &nullable,
        state: BindingState::Value(&null),
    };
    let accepted = boundary
        .evaluate(
            &request(&project, &selected, &[good]),
            context("before"),
            &cancel,
        )
        .unwrap();
    let wrong_revision = T::VocabularyNominal {
        identity: "PipelineDomain".into(),
        version: "9.0.0".into(),
        name: "Result".into(),
    };
    let empty = V::Record(vec![]);
    let cases = [
        (good, EvaluationError::InvalidBinding),
        (
            Binding {
                key: BindingKey::Input(&unknown),
                ty: &T::Num,
                state: BindingState::Unavailable,
            },
            EvaluationError::InvalidBinding,
        ),
        (
            Binding {
                key: BindingKey::Input(&private),
                ty: &T::Num,
                state: BindingState::Failed,
            },
            EvaluationError::InvalidBinding,
        ),
        (
            Binding {
                key: BindingKey::Input(&enabled),
                ty: &T::Bool,
                state: BindingState::Value(&null),
            },
            EvaluationError::BindingType,
        ),
        (
            Binding {
                key: BindingKey::Input(&enabled),
                ty: &T::Num,
                state: BindingState::Unavailable,
            },
            EvaluationError::BindingType,
        ),
        (
            Binding {
                key: BindingKey::Input(&result_input),
                ty: &wrong_revision,
                state: BindingState::Value(&empty),
            },
            EvaluationError::BindingType,
        ),
    ];
    for (bad, expected) in cases {
        let supplied = [good, bad];
        let supplied_request = request(&project, &selected, &supplied);
        assert_eq!(
            boundary.validate_bindings(&supplied_request, &cancel),
            Err(E::Evaluation(expected))
        );
        assert!(
            matches!(boundary.evaluate(&supplied_request, context("bad"), &cancel), Err(E::Evaluation(error)) if error == expected)
        );
        let next = boundary
            .evaluate(
                &request(&project, &selected, &[good]),
                context("after"),
                &cancel,
            )
            .unwrap();
        assert_eq!(next.outcome, accepted.outcome);
        assert_eq!(next.project_identity, accepted.project_identity);
    }
}

/// Consumer policy, not operation-shaped data or a caller override, owns exact output associations.
#[test]
fn integration_forged_output_association_is_rejected() {
    let project = project();
    let cancel = CancellationToken::new();
    let report = contract("Report");
    let result = contract("Result");
    let producer = symbol("producer");
    let private = symbol("private_operation");
    let wrong_revision = T::VocabularyNominal {
        identity: "PipelineDomain".into(),
        version: "9.0.0".into(),
        name: "Result".into(),
    };
    let register = |associations| {
        ExpressionConsumerBoundary::new(
            &project,
            ExpressionConsumerPolicy {
                protocol: ConsumerProtocol {
                    report_contract: &report,
                    supported_features: profile::REQUIRED_FEATURES,
                    diagnostic_codes: &["APP-001"],
                },
                associations,
            },
            ConsumerLimits::default(),
            &cancel,
        )
        .map(|_| ())
    };
    let invalid_associations = [
        [OutputAssociation {
            target: &private,
            result_type: &result,
        }],
        [OutputAssociation {
            target: &producer,
            result_type: &wrong_revision,
        }],
    ];
    for associations in &invalid_associations {
        assert!(matches!(
            register(associations),
            Err(E::Consumer(ConsumerError::PrivateTarget))
        ));
    }
    let boundary = boundary(&project, &report, &[]);
    let selected = symbol("nullable_value");
    let associations = [OutputAssociation {
        target: &producer,
        result_type: &result,
    }];
    let forged_request = EvaluationRequest {
        associations: &associations,
        ..request(&project, &selected, &[])
    };
    assert_eq!(
        boundary.validate_bindings(&forged_request, &cancel),
        Err(E::Consumer(ConsumerError::InvalidInput))
    );
    assert!(matches!(
        boundary.evaluate(&forged_request, context("forged"), &cancel),
        Err(E::Consumer(ConsumerError::InvalidInput))
    ));
    assert_eq!(
        boundary
            .evaluate(
                &request(&project, &selected, &[]),
                context("recovered"),
                &cancel
            )
            .unwrap()
            .outcome,
        R::RequiredBindings(vec![BindingKey::Input(&selected)])
    );
    let registered = super::boundary(&project, &report, &associations);
    let duplicated = [associations[0], associations[0]];
    let duplicated_request = EvaluationRequest {
        associations: &duplicated,
        ..request(&project, &selected, &[])
    };
    assert_eq!(
        registered.validate_bindings(&duplicated_request, &cancel),
        Err(E::Consumer(ConsumerError::InvalidInput))
    );
    assert!(matches!(
        registered.evaluate(&duplicated_request, context("duplicate"), &cancel),
        Err(E::Consumer(ConsumerError::InvalidInput))
    ));
    registered
        .validate_bindings(&request(&project, &selected, &[]), &cancel)
        .unwrap();
}

/// Adapter/target/input revisions distinguish reports, never the complete immutable project identity.
#[test]
fn integration_binding_revisions_do_not_rewrite_project_identity() {
    let project = project();
    let cancel = CancellationToken::new();
    let report = contract("Report");
    let boundary = boundary(&project, &report, &[]);
    let selected = symbol("nullable_value");
    let request = request(&project, &selected, &[]);
    let accepted = boundary
        .evaluate(&request, context("input-1"), &cancel)
        .unwrap();
    for changed in [
        ConsumerContext {
            adapter_revision: "adapter-2",
            ..accepted.context
        },
        ConsumerContext {
            target_revision: "target-2",
            ..accepted.context
        },
        ConsumerContext {
            input_revision: "input-2",
            ..accepted.context
        },
    ] {
        let next = boundary.evaluate(&request, changed, &cancel).unwrap();
        assert_eq!(next.project_identity, accepted.project_identity);
        assert_eq!(next.entry, accepted.entry);
        assert_eq!(next.context.adapter_revision, changed.adapter_revision);
        assert_eq!(next.context.target_revision, changed.target_revision);
        assert_eq!(next.context.input_revision, changed.input_revision);
        assert_eq!(next.outcome, accepted.outcome);
    }
    assert_eq!(accepted.context.adapter_revision, "adapter-1");
    assert_eq!(accepted.context.target_revision, "target-1");
    assert_eq!(accepted.context.input_revision, "input-1");
}
