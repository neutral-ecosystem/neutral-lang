// SPDX-License-Identifier: Apache-2.0

//! Explicit consumer reports and policy rejection over complete reader authority.

use super::*;
use crate::consumer::expression::{
    ExpressionConsumerBoundary, ExpressionConsumerError as CE, ExpressionConsumerPolicy,
};
use crate::consumer::{ConsumerContext, ConsumerError, ConsumerLimits, ConsumerProtocol};
use neutral_semantics::{
    Binding, BindingState, EvaluationError, EvaluationRequest, EvaluationResult, OutputAssociation,
};

/// Registers the exact retained features without adding output associations.
fn policy(contract: &T) -> ExpressionConsumerPolicy<'_> {
    ExpressionConsumerPolicy {
        protocol: ConsumerProtocol {
            report_contract: contract,
            supported_features: profile::REQUIRED_FEATURES,
            diagnostic_codes: &["APP-001"],
        },
        associations: &[],
    }
}

/// Supplies a complete explicit generation with independently positive controls.
fn request<'a>(
    project: &'a ValidatedExpressionProject,
    entry: &'a ModuleSymbolIdentity,
    bindings: &'a [Binding<'a>],
) -> EvaluationRequest<'a> {
    EvaluationRequest {
        selected: entry,
        profiles: &project.ir().semantic_profiles,
        bindings,
        associations: &[],
        limits: ExpressionLimits::hard(),
    }
}

/// Keeps consumer revisions separate from the immutable project meaning.
fn context() -> ConsumerContext<'static> {
    ConsumerContext {
        adapter_revision: "adapter-1",
        target_revision: "target-1",
        input_revision: "input-1",
    }
}

/// Retains a report contract alongside independently guarded Boolean input ports.
fn guarded_consumer_project() -> ValidatedExpressionProject {
    let mut ir = guarded_fixture(BinaryOperator::And);
    let mut definition = default_fixture(loc("m", 20, 21));
    ir.declarations.push(definition.declarations.remove(0));
    ir.source_maps.push(definition.source_maps.remove(0));
    authority(ir)
}

/// Missing required features and malformed registrations cannot confer evaluation authority.
#[test]
fn consumer_policy_rejects_missing_features_reserved_codes_and_private_contracts() {
    let project = authority(default_fixture(loc("m", 20, 21)));
    let contract = T::Nominal(symbol("m", "Record"));
    let good = policy(&contract);
    let cancel = CancellationToken::new();
    for (features, codes, expected) in [
        (&[][..], &[][..], ConsumerError::Unsupported),
        (
            profile::REQUIRED_FEATURES,
            &["NEU-FORGED"][..],
            ConsumerError::InvalidInput,
        ),
        (
            profile::REQUIRED_FEATURES,
            &["APP-1", "APP-1"][..],
            ConsumerError::InvalidInput,
        ),
        (&["z", "a"][..], &[][..], ConsumerError::InvalidInput),
    ] {
        let result = ExpressionConsumerBoundary::new(
            &project,
            ExpressionConsumerPolicy {
                protocol: ConsumerProtocol {
                    supported_features: features,
                    diagnostic_codes: codes,
                    ..good.protocol
                },
                ..good
            },
            ConsumerLimits::default(),
            &cancel,
        );
        assert!(matches!(result, Err(CE::Consumer(error)) if error == expected));
    }
    for (contract, expected) in [
        (T::Num, ConsumerError::TypeMismatch),
        (
            T::Nominal(symbol("m", "absent")),
            ConsumerError::PrivateTarget,
        ),
    ] {
        let result = ExpressionConsumerBoundary::new(
            &project,
            policy(&contract),
            ConsumerLimits::default(),
            &cancel,
        );
        assert!(matches!(result, Err(CE::Consumer(error)) if error == expected));
    }
    let mut ir = default_fixture(loc("m", 20, 21));
    ir.declarations.iter_mut().for_each(|d| d.public = false);
    let private = authority(ir);
    let result = ExpressionConsumerBoundary::new(
        &private,
        policy(&contract),
        ConsumerLimits::default(),
        &cancel,
    );
    assert!(matches!(
        result,
        Err(CE::Consumer(ConsumerError::PrivateTarget))
    ));
}

/// Guard reports require only the selected key and retain canonical entry/context authority.
#[test]
fn consumer_reports_guarded_input_demand_and_complete_values() {
    let contract = T::Nominal(symbol("m", "Record"));
    // The report contract is independent of the selected Boolean entry.
    let project = guarded_consumer_project();
    let cancel = CancellationToken::new();
    let boundary = ExpressionConsumerBoundary::new(
        &project,
        policy(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let entry = symbol("m", "result");
    let enabled = symbol("m", "enabled");
    let other = symbol("m", "other");
    let report = boundary
        .evaluate(&request(&project, &entry, &[]), context(), &cancel)
        .unwrap();
    assert_eq!(
        report.outcome,
        EvaluationResult::RequiredBindings(vec![BindingKey::Input(&enabled)])
    );
    assert_eq!(report.entry, &entry);
    assert!(std::ptr::eq(
        report.entry,
        &raw const project.ir().declarations.last().unwrap().identity
    ));
    assert_eq!(report.project_identity, boundary.project_identity());
    assert_eq!(report.context.adapter_revision, "adapter-1");
    assert_eq!(report.context.target_revision, "target-1");
    assert_eq!(report.context.input_revision, "input-1");
    let disabled = V::Bool(false);
    let enabled_value = V::Bool(true);
    for (value, expected) in [
        (
            &disabled,
            EvaluationResult::Value {
                ty: &T::Bool,
                value: V::Bool(false),
            },
        ),
        (
            &enabled_value,
            EvaluationResult::RequiredBindings(vec![BindingKey::Input(&other)]),
        ),
    ] {
        let bindings = [Binding {
            key: BindingKey::Input(&enabled),
            ty: &T::Bool,
            state: BindingState::Value(value),
        }];
        let report = boundary
            .evaluate(
                &request(&project, &entry, &bindings),
                ConsumerContext {
                    input_revision: "input-2",
                    ..context()
                },
                &cancel,
            )
            .unwrap();
        assert_eq!(report.outcome, expected);
        assert_eq!(report.project_identity, boundary.project_identity());
        assert_eq!(report.context.input_revision, "input-2");
    }
    for (state, expected) in [
        (
            BindingState::Unavailable,
            EvaluationResult::Unavailable(BindingKey::Input(&enabled)),
        ),
        (
            BindingState::Failed,
            EvaluationResult::Failure(EvaluationError::BindingFailed),
        ),
    ] {
        let bindings = [Binding {
            key: BindingKey::Input(&enabled),
            ty: &T::Bool,
            state,
        }];
        assert_eq!(
            boundary
                .evaluate(&request(&project, &entry, &bindings), context(), &cancel)
                .unwrap()
                .outcome,
            expected
        );
    }
}

/// A skipped branch still cannot carry a wrong typed or duplicate caller envelope.
#[test]
fn consumer_validates_complete_generations_before_guard_execution() {
    let project = guarded_consumer_project();
    let contract = T::Nominal(symbol("m", "Record"));
    let cancel = CancellationToken::new();
    let boundary = ExpressionConsumerBoundary::new(
        &project,
        policy(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let entry = symbol("m", "result");
    assert_eq!(
        boundary.validate_bindings(&request(&project, &entry, &[]), &cancel),
        Ok(())
    );
    let unknown = symbol("m", "unknown");
    let bindings = [Binding {
        key: BindingKey::Input(&unknown),
        ty: &T::Num,
        state: BindingState::Value(&V::Bool(true)),
    }];
    assert_eq!(
        boundary.validate_bindings(&request(&project, &entry, &bindings), &cancel),
        Err(CE::Evaluation(EvaluationError::InvalidBinding))
    );
    let enabled = symbol("m", "enabled");
    let other = symbol("m", "other");
    let disabled = V::Bool(false);
    let first = Binding {
        key: BindingKey::Input(&enabled),
        ty: &T::Bool,
        state: BindingState::Value(&disabled),
    };
    let wrong_type = [
        first,
        Binding {
            key: BindingKey::Input(&other),
            ty: &T::Num,
            state: BindingState::Value(&one()),
        },
    ];
    assert!(matches!(
        boundary.evaluate(&request(&project, &entry, &wrong_type), context(), &cancel),
        Err(CE::Evaluation(EvaluationError::BindingType))
    ));
    let duplicate = [first, first];
    assert_eq!(
        boundary.validate_bindings(&request(&project, &entry, &duplicate), &cancel),
        Err(CE::Evaluation(EvaluationError::InvalidBinding))
    );
    assert!(matches!(
        boundary.evaluate(
            &request(&project, &entry, &[]),
            ConsumerContext {
                adapter_revision: "",
                ..context()
            },
            &cancel
        ),
        Err(CE::Consumer(ConsumerError::InvalidInput))
    ));
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert_eq!(
        boundary.validate_bindings(&request(&project, &entry, &[]), &cancelled),
        Err(CE::Evaluation(EvaluationError::Cancelled))
    );
    assert!(matches!(
        boundary.evaluate(&request(&project, &entry, &[]), context(), &cancelled),
        Err(CE::Consumer(ConsumerError::Cancelled))
    ));
}

/// Output policy cannot be expanded by a request or contain duplicate/private targets.
#[test]
fn consumer_association_registration_cannot_be_overridden() {
    let project = authority(default_fixture(loc("m", 20, 21)));
    let contract = T::Nominal(symbol("m", "Record"));
    let entry = symbol("m", "a");
    let association = OutputAssociation {
        target: &entry,
        result_type: &contract,
    };
    let cancel = CancellationToken::new();
    let good = policy(&contract);
    let duplicates = [association, association];
    assert!(matches!(
        ExpressionConsumerBoundary::new(
            &project,
            ExpressionConsumerPolicy {
                associations: &duplicates,
                ..good
            },
            ConsumerLimits::default(),
            &cancel
        ),
        Err(CE::Consumer(ConsumerError::InvalidInput))
    ));
    let missing = symbol("m", "missing");
    let associations = [OutputAssociation {
        target: &missing,
        result_type: &contract,
    }];
    assert!(matches!(
        ExpressionConsumerBoundary::new(
            &project,
            ExpressionConsumerPolicy {
                associations: &associations,
                ..good
            },
            ConsumerLimits::default(),
            &cancel
        ),
        Err(CE::Consumer(ConsumerError::PrivateTarget))
    ));
    let boundary =
        ExpressionConsumerBoundary::new(&project, good, ConsumerLimits::default(), &cancel)
            .unwrap();
    let associations = [association];
    let mut request = request(&project, &entry, &[]);
    request.associations = &associations;
    assert_eq!(
        boundary.validate_bindings(&request, &cancel),
        Err(CE::Consumer(ConsumerError::InvalidInput))
    );
    let registered = ExpressionConsumerBoundary::new(
        &project,
        ExpressionConsumerPolicy {
            associations: &associations,
            ..good
        },
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    assert_eq!(
        registered.validate_bindings(&request, &cancel),
        Err(CE::Evaluation(EvaluationError::InvalidBinding))
    );
}

/// Each consumer control remains positive; no zero dimension may bypass policy validation.
#[test]
fn consumer_controls_and_cancellation_reject_before_publication() {
    let project = authority(default_fixture(loc("m", 20, 21)));
    let contract = T::Nominal(symbol("m", "Record"));
    let cancel = CancellationToken::new();
    let mut controls = Vec::new();
    for index in 0..11 {
        let mut limits = ConsumerLimits::default();
        match index {
            0 => limits.roots = 0,
            1 => limits.candidates = 0,
            2 => limits.closure_edges = 0,
            3 => limits.bytes = 0,
            4 => limits.text_bytes = 0,
            5 => limits.features = 0,
            6 => limits.diagnostics = 0,
            7 => limits.locations = 0,
            8 => limits.path_segments = 0,
            9 => limits.work = 0,
            _ => limits.output_bytes = 0,
        }
        controls.push(limits);
    }
    controls.push(ConsumerLimits {
        features: 1,
        ..Default::default()
    });
    for limits in controls {
        assert!(matches!(
            ExpressionConsumerBoundary::new(&project, policy(&contract), limits, &cancel),
            Err(CE::Consumer(ConsumerError::Limit))
        ));
    }
    cancel.cancel();
    assert!(matches!(
        ExpressionConsumerBoundary::new(
            &project,
            policy(&contract),
            ConsumerLimits::default(),
            &cancel
        ),
        Err(CE::Consumer(ConsumerError::Cancelled))
    ));
}

/// A positive budget sufficient for registration still must bound complete logical identity framing.
#[test]
fn consumer_identity_budget_fails_without_revoking_reader_authority() {
    let project = authority(default_fixture(loc("m", 20, 21)));
    let contract = T::Nominal(symbol("m", "Record"));
    let cancel = CancellationToken::new();
    let controls = ConsumerLimits {
        bytes: 512,
        ..Default::default()
    };
    let result = ExpressionConsumerBoundary::new(&project, policy(&contract), controls, &cancel);
    assert!(matches!(result, Err(CE::Consumer(ConsumerError::Limit))));
    let boundary = ExpressionConsumerBoundary::new(
        &project,
        policy(&contract),
        ConsumerLimits::default(),
        &cancel,
    )
    .unwrap();
    let entry = symbol("m", "a");
    let report = boundary
        .evaluate(&request(&project, &entry, &[]), context(), &cancel)
        .unwrap();
    assert_eq!(report.project_identity, boundary.project_identity());
    assert_eq!(
        report.outcome,
        EvaluationResult::Value {
            ty: &contract,
            value: V::Record(vec![("x".into(), Some(one()))])
        }
    );
}
