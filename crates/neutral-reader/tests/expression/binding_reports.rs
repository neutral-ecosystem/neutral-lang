// SPDX-License-Identifier: Apache-2.0

//! Complete explicit report generations share context, envelope and evaluation controls.

use super::*;

/// Context validation cannot spend almost the whole work budget and then reset it for evaluation.
#[test]
fn report_context_and_evaluation_share_the_consumer_work_budget() {
    let project = authority(default_fixture(loc("m", 20, 21)));
    let contract = T::Nominal(symbol("m", "Record"));
    let cancel = CancellationToken::new();
    let controls = ConsumerLimits {
        work: 100_000,
        ..Default::default()
    };
    let boundary =
        ExpressionConsumerBoundary::new(&project, policy(&contract), controls, &cancel).unwrap();
    let entry = symbol("m", "a");
    let request = request(&project, &entry, &[]);
    let accepted = boundary.evaluate(&request, context(), &cancel).unwrap();
    let revision = "r".repeat(usize::try_from(controls.work - 10).unwrap());
    let failure = boundary.evaluate(
        &request,
        ConsumerContext {
            adapter_revision: "a",
            target_revision: "t",
            input_revision: &revision,
        },
        &cancel,
    );
    assert!(matches!(
        failure,
        Err(CE::Evaluation(EvaluationError::Limit))
    ));
    assert_eq!(accepted.project_identity, boundary.project_identity());
    assert_eq!(
        boundary
            .evaluate(&request, context(), &cancel)
            .unwrap()
            .outcome,
        accepted.outcome
    );
}

/// Registration must not publish partial authority when identity retention fails or is cancelled.
#[test]
fn consumer_registration_faults_preserve_prior_boundary_and_recover() {
    use neutral_core::allocation::testing;
    let project = authority(default_fixture(loc("m", 20, 21)));
    let contract = T::Nominal(symbol("m", "Record"));
    let cancel = CancellationToken::new();
    let register = |signal: &CancellationToken| {
        ExpressionConsumerBoundary::new(
            &project,
            policy(&contract),
            ConsumerLimits::default(),
            signal,
        )
    };
    let (accepted, count) = testing::observe(None, || register(&cancel));
    let accepted = accepted.unwrap();
    let entry = symbol("m", "a");
    let request = request(&project, &entry, &[]);
    let expected = accepted
        .evaluate(&request, context(), &cancel)
        .unwrap()
        .outcome;
    assert!(count > 0);
    for index in 0..count {
        let (failed, _) = testing::observe(Some(index), || register(&cancel));
        assert!(
            matches!(failed, Err(CE::Consumer(ConsumerError::Limit))),
            "allocation {index}"
        );
        let signal = CancellationToken::new();
        let (failed, _) = testing::observe_cancellation(index, &signal, || register(&signal));
        assert!(signal.is_cancelled());
        assert!(
            matches!(failed, Err(CE::Consumer(ConsumerError::Cancelled))),
            "cancellation {index}"
        );
        assert_eq!(
            accepted
                .evaluate(&request, context(), &cancel)
                .unwrap()
                .outcome,
            expected
        );
        assert_eq!(
            register(&cancel).unwrap().project_identity(),
            accepted.project_identity()
        );
    }
}

/// Every observed preflight/evaluation failure is atomic and confined to its explicit generation.
#[test]
fn consumer_binding_and_report_faults_recover_without_revoking_accepted_reports() {
    use neutral_core::allocation::testing;
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
    let enabled = symbol("m", "enabled");
    let enabled_value = V::Bool(true);
    let bindings = [Binding {
        key: BindingKey::Input(&enabled),
        ty: &T::Bool,
        state: BindingState::Value(&enabled_value),
    }];
    let request = request(&project, &entry, &bindings);
    let accepted = boundary.evaluate(&request, context(), &cancel).unwrap();
    for validation_only in [true, false] {
        let run = |signal: &CancellationToken| {
            if validation_only {
                boundary.validate_bindings(&request, signal).map(|()| None)
            } else {
                boundary
                    .evaluate(&request, context(), signal)
                    .and_then(|report| match report.outcome {
                        EvaluationResult::Failure(error) => Err(CE::Evaluation(error)),
                        outcome => Ok(Some(outcome)),
                    })
            }
        };
        let (result, count) = testing::observe(None, || run(&cancel));
        let result = result.unwrap();
        assert!(count > 0);
        for index in 0..count {
            let (failed, _) = testing::observe(Some(index), || run(&cancel));
            assert!(
                matches!(failed, Err(CE::Evaluation(EvaluationError::Limit))),
                "allocation {index}, validation={validation_only}"
            );
            let signal = CancellationToken::new();
            let (failed, _) = testing::observe_cancellation(index, &signal, || run(&signal));
            assert!(signal.is_cancelled());
            assert!(
                matches!(
                    failed,
                    Err(CE::Evaluation(EvaluationError::Cancelled)
                        | CE::Consumer(ConsumerError::Cancelled))
                ),
                "cancellation {index}, validation={validation_only}"
            );
            assert_eq!(run(&cancel).unwrap(), result);
            assert_eq!(
                boundary
                    .evaluate(&request, context(), &cancel)
                    .unwrap()
                    .outcome,
                accepted.outcome
            );
        }
    }
}

/// Aggregate context bytes cannot be reused as fresh binding/evaluation storage.
#[test]
fn report_context_and_evaluation_share_the_consumer_byte_budget() {
    let project = authority(default_fixture(loc("m", 20, 21)));
    let contract = T::Nominal(symbol("m", "Record"));
    let cancel = CancellationToken::new();
    let controls = ConsumerLimits {
        bytes: 100_000,
        ..Default::default()
    };
    let boundary =
        ExpressionConsumerBoundary::new(&project, policy(&contract), controls, &cancel).unwrap();
    let entry = symbol("m", "a");
    let request = request(&project, &entry, &[]);
    let accepted = boundary.evaluate(&request, context(), &cancel).unwrap();
    let revision = "r".repeat(usize::try_from(controls.bytes - 27).unwrap());
    assert!(matches!(
        boundary.evaluate(
            &request,
            ConsumerContext {
                adapter_revision: "a",
                target_revision: "t",
                input_revision: &revision,
            },
            &cancel
        ),
        Err(CE::Evaluation(EvaluationError::Limit))
    ));
    assert_eq!(
        boundary
            .evaluate(&request, context(), &cancel)
            .unwrap()
            .outcome,
        accepted.outcome
    );
}

/// Common-prefix nominal keys charge comparison bytes, not just the number of policy entries.
#[test]
fn association_prefix_comparisons_respect_the_registration_work_budget() {
    let mut ir = default_fixture(loc("m", 20, 21));
    let contract = T::Nominal(symbol("m", "Record"));
    let prefix = "p".repeat(256);
    let targets: Vec<_> = (0..64)
        .map(|n| symbol("m", &format!("{prefix}{n:02}")))
        .collect();
    for target in &targets {
        ir.declarations.push(ExpressionDeclaration {
            identity: target.clone(),
            public: true,
            signature: S::Binding(contract.clone()),
            body: Some(ExpressionBody::Input),
        });
        ir.source_maps.push(ProjectSourceMap {
            declaration: target.clone(),
            location: loc("m", 80, 90),
        });
        ir.provenance.push(ProjectProvenance {
            from: target.clone(),
            to: symbol("m", "Record"),
            kind: K::Type,
            location: loc("m", 81, 82),
        });
    }
    let project = authority(ir);
    let cancel = CancellationToken::new();
    let associations: Vec<_> = targets
        .iter()
        .map(|target| OutputAssociation {
            target,
            result_type: &contract,
        })
        .collect();
    let registered = ExpressionConsumerPolicy {
        associations: &associations,
        ..policy(&contract)
    };
    let controls = ConsumerLimits {
        work: 300_000,
        ..Default::default()
    };
    assert!(
        ExpressionConsumerBoundary::new(&project, policy(&contract), controls, &cancel).is_ok()
    );
    assert!(matches!(
        ExpressionConsumerBoundary::new(&project, registered, controls, &cancel),
        Err(CE::Consumer(ConsumerError::Limit))
    ));
    assert!(
        ExpressionConsumerBoundary::new(&project, registered, ConsumerLimits::default(), &cancel)
            .is_ok()
    );
    let boundary = ExpressionConsumerBoundary::new(
        &project,
        registered,
        ConsumerLimits {
            work: 1_000_000,
            ..Default::default()
        },
        &cancel,
    )
    .unwrap();
    let entry = symbol("m", "a");
    let request = EvaluationRequest {
        associations: &associations,
        ..request(&project, &entry, &[])
    };
    assert_eq!(
        boundary.validate_bindings(&request, &cancel),
        Err(CE::Consumer(ConsumerError::Limit))
    );
    assert!(matches!(
        boundary.evaluate(&request, context(), &cancel),
        Err(CE::Consumer(ConsumerError::Limit))
    ));
}
