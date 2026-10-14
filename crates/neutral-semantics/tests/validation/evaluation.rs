// SPDX-License-Identifier: Apache-2.0

//! Explicit binding generations and selected-path demand, without acquisition or scheduling.

use super::*;
use neutral_ir::composition::BindingValue as V;
use neutral_semantics::{
    Binding, BindingKey, BindingState, EvaluationError, EvaluationRequest, EvaluationResult,
    evaluate,
};

fn program() -> neutral_semantics::ValidatedExpressionModel {
    let mut ir = project(vec![
        binding("ask", T::Bool, ExpressionBody::Input),
        binding(
            "result",
            T::Bool,
            ExpressionBody::Expression(Expression {
                ty: T::Bool,
                node: ExpressionNode::If {
                    guard: Box::new(Expression {
                        ty: T::Bool,
                        node: ExpressionNode::Input(symbol("ask")),
                    }),
                    then_branch: Box::new(leaf(false)),
                    else_branch: Box::new(leaf(true)),
                },
            }),
        ),
        binding("spare", T::Bool, ExpressionBody::Input),
    ]);
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: 4,
        residual_depth: 2,
        input_declarations: 2,
        potential_value_edges: 1,
        retained_expression_text_bytes: 54,
        ..ExpressionResourceFacts::default()
    };
    validate(ir).unwrap()
}

fn run<'m>(
    model: &'m neutral_semantics::ValidatedExpressionModel,
    bindings: &[Binding<'_>],
) -> EvaluationResult<'m> {
    evaluate(
        model,
        &EvaluationRequest {
            selected: &symbol("result"),
            profiles: &model.ir().semantic_profiles,
            bindings,
            associations: &[],
            limits: ExpressionLimits::hard(),
        },
        &CancellationToken::new(),
    )
}

#[test]
fn missing_guard_demands_only_guard() {
    let model = program();
    let EvaluationResult::RequiredBindings(keys) = run(&model, &[]) else {
        panic!("guard demand expected");
    };
    assert_eq!(keys, [BindingKey::Input(&symbol("ask"))]);
}

#[test]
fn match_payload_binding_is_selected_and_request_local() {
    let ty = T::Nominal(symbol("State"));
    let mut ir = project(vec![
        D {
            identity: symbol("State"),
            public: true,
            signature: S::Definition(CompositionBody::Variant(vec![
                neutral_ir::composition::CompositionAlternative {
                    tag: "failed".into(),
                    ty: T::Bool,
                },
                neutral_ir::composition::CompositionAlternative {
                    tag: "ready".into(),
                    ty: T::Bool,
                },
            ])),
            body: None,
        },
        binding("input", T::Nominal(symbol("State")), ExpressionBody::Input),
        binding(
            "result",
            T::Bool,
            ExpressionBody::Expression(Expression {
                ty: T::Bool,
                node: ExpressionNode::Match {
                    scrutinee: Box::new(Expression {
                        ty: T::Nominal(symbol("State")),
                        node: ExpressionNode::Input(symbol("input")),
                    }),
                    arms: ["failed", "ready"]
                        .into_iter()
                        .map(|tag| MatchArm {
                            tag: tag.into(),
                            payload_type: T::Bool,
                            body: Expression {
                                ty: T::Bool,
                                node: ExpressionNode::Bound { depth: 0, slot: 0 },
                            },
                        })
                        .collect(),
                },
            }),
        ),
    ]);
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: 4,
        residual_depth: 2,
        input_declarations: 1,
        potential_value_edges: 1,
        retained_expression_text_bytes: 88,
        ..ExpressionResourceFacts::default()
    };
    let model = validate(ir).unwrap();
    let input = symbol("input");
    for (tag, payload) in [("ready", true), ("failed", false), ("ready", false)] {
        let value = V::Variant {
            tag: tag.into(),
            payload: Box::new(V::Bool(payload)),
        };
        assert_eq!(
            run(
                &model,
                &[Binding {
                    key: BindingKey::Input(&input),
                    ty: &ty,
                    state: BindingState::Value(&value)
                }]
            ),
            EvaluationResult::Value {
                ty: &T::Bool,
                value: V::Bool(payload)
            }
        );
    }
    assert_eq!(
        run(&model, &[]),
        EvaluationResult::RequiredBindings(vec![BindingKey::Input(&input)])
    );
}

#[test]
fn duplicate_unknown_and_wrong_profile_envelopes_reject_before_demand() {
    let model = program();
    let ask = symbol("ask");
    let supplied = Binding {
        key: BindingKey::Input(&ask),
        ty: &T::Bool,
        state: BindingState::Value(&V::Bool(true)),
    };
    assert_eq!(
        run(&model, &[supplied, supplied]),
        EvaluationResult::Failure(EvaluationError::InvalidBinding)
    );
    let unknown = symbol("unknown");
    assert_eq!(
        run(
            &model,
            &[Binding {
                key: BindingKey::Input(&unknown),
                ..supplied
            }]
        ),
        EvaluationResult::Failure(EvaluationError::InvalidBinding)
    );
    let mut profiles = SemanticProfiles::try_frozen().unwrap();
    profiles.evaluator = "unsupported".into();
    assert_eq!(
        evaluate(
            &model,
            &EvaluationRequest {
                selected: &symbol("result"),
                profiles: &profiles,
                bindings: &[],
                associations: &[],
                limits: ExpressionLimits::hard()
            },
            &CancellationToken::new()
        ),
        EvaluationResult::Failure(EvaluationError::Unsupported)
    );
}

#[test]
fn evaluator_output_and_depth_limits_are_independent_of_input_retention() {
    let model = program();
    let ask = symbol("ask");
    let bindings = [Binding {
        key: BindingKey::Input(&ask),
        ty: &T::Bool,
        state: BindingState::Value(&V::Bool(true)),
    }];
    let output = 1;
    for (bytes, depth, passes) in [
        (output, 2, true),
        (output - 1, 2, false),
        (output, 1, false),
    ] {
        let limits = ExpressionLimits {
            output_bytes: bytes,
            evaluation_depth: depth,
            ..ExpressionLimits::hard()
        };
        let result = evaluate(
            &model,
            &EvaluationRequest {
                selected: &symbol("result"),
                profiles: &model.ir().semantic_profiles,
                bindings: &bindings,
                associations: &[],
                limits,
            },
            &CancellationToken::new(),
        );
        assert_eq!(
            result,
            if passes {
                EvaluationResult::Value {
                    ty: &T::Bool,
                    value: V::Bool(false),
                }
            } else {
                EvaluationResult::Failure(EvaluationError::Limit)
            }
        );
    }
}

#[test]
fn malformed_supplied_skipped_binding_fails_preflight() {
    let model = program();
    let spare = symbol("spare");
    let wrong = V::String("not a boolean".into());
    assert_eq!(
        run(
            &model,
            &[Binding {
                key: BindingKey::Input(&spare),
                ty: &T::Bool,
                state: BindingState::Value(&wrong)
            }]
        ),
        EvaluationResult::Failure(EvaluationError::BindingType)
    );
}

#[test]
fn failed_and_unavailable_are_not_null() {
    let model = program();
    let ask = symbol("ask");
    assert_eq!(
        run(
            &model,
            &[Binding {
                key: BindingKey::Input(&ask),
                ty: &T::Bool,
                state: BindingState::Failed
            }]
        ),
        EvaluationResult::Failure(EvaluationError::BindingFailed)
    );
    assert_eq!(
        run(
            &model,
            &[Binding {
                key: BindingKey::Input(&ask),
                ty: &T::Bool,
                state: BindingState::Unavailable
            }]
        ),
        EvaluationResult::Unavailable(BindingKey::Input(&ask))
    );
    assert_eq!(
        run(
            &model,
            &[Binding {
                key: BindingKey::Input(&ask),
                ty: &T::Bool,
                state: BindingState::Value(&V::Null)
            }]
        ),
        EvaluationResult::Failure(EvaluationError::BindingType)
    );
}

#[test]
fn explicit_binding_generations_do_not_mutate_the_model() {
    let model = program();
    let ask = symbol("ask");
    for (input, output) in [(true, false), (false, true), (true, false)] {
        assert_eq!(
            run(
                &model,
                &[Binding {
                    key: BindingKey::Input(&ask),
                    ty: &T::Bool,
                    state: BindingState::Value(&V::Bool(input))
                }]
            ),
            EvaluationResult::Value {
                ty: &T::Bool,
                value: V::Bool(output)
            }
        );
    }
    assert!(matches!(
        run(&model, &[]),
        EvaluationResult::RequiredBindings(_)
    ));
}

#[test]
fn missing_operands_are_sorted_unique_and_failed_unused_inputs_are_not_demanded() {
    let make = |right: &str| {
        let mut ir = project(vec![
            binding("ask", T::Bool, ExpressionBody::Input),
            binding(
                "result",
                T::Bool,
                ExpressionBody::Expression(Expression {
                    ty: T::Bool,
                    node: ExpressionNode::Binary {
                        operator: BinaryOperator::Equal,
                        left: Box::new(Expression {
                            ty: T::Bool,
                            node: ExpressionNode::Input(symbol("spare")),
                        }),
                        right: Box::new(Expression {
                            ty: T::Bool,
                            node: ExpressionNode::Input(symbol(right)),
                        }),
                    },
                }),
            ),
            binding("spare", T::Bool, ExpressionBody::Input),
        ]);
        ir.expression_resources = ExpressionResourceFacts {
            residual_nodes: 3,
            residual_depth: 2,
            input_declarations: 2,
            potential_value_edges: 2,
            retained_expression_text_bytes: if right == "ask" { 58 } else { 60 },
            ..ExpressionResourceFacts::default()
        };
        validate(ir).unwrap()
    };
    let model = make("ask");
    assert_eq!(
        run(&model, &[]),
        EvaluationResult::RequiredBindings(vec![
            BindingKey::Input(&symbol("ask")),
            BindingKey::Input(&symbol("spare"))
        ])
    );
    let same = make("spare");
    assert_eq!(
        run(&same, &[]),
        EvaluationResult::RequiredBindings(vec![BindingKey::Input(&symbol("spare"))])
    );
    let model = program();
    let ask = symbol("ask");
    let spare = symbol("spare");
    assert_eq!(
        run(
            &model,
            &[
                Binding {
                    key: BindingKey::Input(&ask),
                    ty: &T::Bool,
                    state: BindingState::Value(&V::Bool(true))
                },
                Binding {
                    key: BindingKey::Input(&spare),
                    ty: &T::Bool,
                    state: BindingState::Failed
                },
            ]
        ),
        EvaluationResult::Value {
            ty: &T::Bool,
            value: V::Bool(false)
        }
    );
}

#[test]
fn boolean_short_circuit_skips_arithmetic_but_selected_division_fails() {
    let make = |guard| {
        let number = |digits| Expression {
            ty: T::Num,
            node: ExpressionNode::Value(V::Number(
                neutral_ir::ExactNumber::from_unsigned_integer(digits).unwrap(),
            )),
        };
        let divide = Expression {
            ty: T::Num,
            node: ExpressionNode::Binary {
                operator: BinaryOperator::Divide,
                left: Box::new(number("1")),
                right: Box::new(number("0")),
            },
        };
        let mut ir = project(vec![binding(
            "result",
            T::Bool,
            ExpressionBody::Expression(Expression {
                ty: T::Bool,
                node: ExpressionNode::Binary {
                    operator: BinaryOperator::And,
                    left: Box::new(leaf(guard)),
                    right: Box::new(Expression {
                        ty: T::Bool,
                        node: ExpressionNode::Binary {
                            operator: BinaryOperator::Equal,
                            left: Box::new(divide),
                            right: Box::new(number("0")),
                        },
                    }),
                },
            }),
        )]);
        ir.expression_resources = ExpressionResourceFacts {
            residual_nodes: 7,
            residual_depth: 4,
            retained_expression_text_bytes: 83,
            ..ExpressionResourceFacts::default()
        };
        validate(ir).unwrap()
    };
    assert_eq!(
        run(&make(false), &[]),
        EvaluationResult::Value {
            ty: &T::Bool,
            value: V::Bool(false)
        }
    );
    assert_eq!(
        run(&make(true), &[]),
        EvaluationResult::Failure(EvaluationError::DivisionByZero)
    );
}

#[test]
fn evaluation_allocation_failure_and_cancellation_leave_the_model_reusable() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    let model = program();
    let ask = symbol("ask");
    let values = [Binding {
        key: BindingKey::Input(&ask),
        ty: &T::Bool,
        state: BindingState::Value(&V::Bool(true)),
    }];
    let owner = symbol("result");
    let request = EvaluationRequest {
        selected: &owner,
        profiles: &model.ir().semantic_profiles,
        bindings: &values,
        associations: &[],
        limits: ExpressionLimits::hard(),
    };
    let signal = CancellationToken::new();
    let (result, reservations) = observe(None, || evaluate(&model, &request, &signal));
    assert_eq!(
        result,
        EvaluationResult::Value {
            ty: &T::Bool,
            value: V::Bool(false)
        }
    );
    for index in 0..reservations {
        let (result, _) = observe(Some(index), || evaluate(&model, &request, &signal));
        assert_eq!(result, EvaluationResult::Failure(EvaluationError::Limit));
        let cancelled = CancellationToken::new();
        let (result, _) =
            observe_cancellation(index, &cancelled, || evaluate(&model, &request, &cancelled));
        assert_eq!(
            result,
            EvaluationResult::Failure(EvaluationError::Cancelled)
        );
        assert_eq!(
            evaluate(&model, &request, &signal),
            EvaluationResult::Value {
                ty: &T::Bool,
                value: V::Bool(false)
            }
        );
    }
}
