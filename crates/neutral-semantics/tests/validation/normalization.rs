// SPDX-License-Identifier: Apache-2.0

//! Producer folding shares selected-path arithmetic, without supplying consumer bindings.

use super::*;
use neutral_ir::composition::BindingValue as V;
use neutral_semantics::{NormalizedBinding, normalize_binding};

#[test]
fn static_decimal_folding_matches_literal_and_recovers_after_every_reservation_failure() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    let number = |digits| Expression {
        ty: T::Num,
        node: ExpressionNode::Value(V::Number(
            neutral_ir::ExactNumber::from_unsigned_integer(digits).unwrap(),
        )),
    };
    let mut ir = project(vec![binding(
        "result",
        T::Num,
        ExpressionBody::Expression(Expression {
            ty: T::Num,
            node: ExpressionNode::Binary {
                operator: BinaryOperator::Divide,
                left: Box::new(number("1")),
                right: Box::new(number("3")),
            },
        }),
    )]);
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: 3,
        residual_depth: 2,
        retained_expression_text_bytes: 34,
        ..ExpressionResourceFacts::default()
    };
    let model = validate(ir).unwrap();
    let selected = symbol("result");
    let expected = NormalizedBinding::Value(V::Number(
        neutral_ir::ExactNumber::from_source("0.3333333333333333333333333333333333", 35, 34)
            .unwrap(),
    ));
    let cancel = CancellationToken::new();
    let (result, count) = observe(None, || {
        normalize_binding(&model, &selected, ExpressionLimits::hard(), &cancel)
    });
    assert_eq!(result, Ok(expected));
    assert_ne!(count, 0);
    for index in 0..count {
        let (result, _) = observe(Some(index), || {
            normalize_binding(&model, &selected, ExpressionLimits::hard(), &cancel)
        });
        assert_eq!(result, Err(neutral_semantics::EvaluationError::Limit));
        let signal = CancellationToken::new();
        let (result, _) = observe_cancellation(index, &signal, || {
            normalize_binding(&model, &selected, ExpressionLimits::hard(), &signal)
        });
        assert_eq!(result, Err(neutral_semantics::EvaluationError::Cancelled));
        assert!(normalize_binding(&model, &selected, ExpressionLimits::hard(), &cancel).is_ok());
    }
}

#[test]
fn private_static_binding_folds_without_becoming_a_public_entry() {
    let mut declaration = binding("result", T::Bool, ExpressionBody::Expression(leaf(true)));
    declaration.public = false;
    let mut ir = project(vec![declaration]);
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: 1,
        residual_depth: 1,
        retained_expression_text_bytes: 13,
        ..ExpressionResourceFacts::default()
    };
    let model = validate(ir).unwrap();
    assert_eq!(
        normalize_binding(
            &model,
            &symbol("result"),
            ExpressionLimits::hard(),
            &CancellationToken::new()
        ),
        Ok(NormalizedBinding::Value(V::Bool(true)))
    );
    assert_eq!(
        neutral_semantics::evaluate(
            &model,
            &neutral_semantics::EvaluationRequest {
                selected: &symbol("result"),
                profiles: &model.ir().semantic_profiles,
                bindings: &[],
                associations: &[],
                limits: ExpressionLimits::hard()
            },
            &CancellationToken::new()
        ),
        neutral_semantics::EvaluationResult::Failure(
            neutral_semantics::EvaluationError::UnknownEntry
        )
    );
}

#[test]
fn unresolved_guard_retains_original_body_without_eager_arithmetic() {
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
    let mut ir = project(vec![
        binding("ask", T::Bool, ExpressionBody::Input),
        binding(
            "result",
            T::Num,
            ExpressionBody::Expression(Expression {
                ty: T::Num,
                node: ExpressionNode::If {
                    guard: Box::new(Expression {
                        ty: T::Bool,
                        node: ExpressionNode::Input(symbol("ask")),
                    }),
                    then_branch: Box::new(divide),
                    else_branch: Box::new(number("2")),
                },
            }),
        ),
    ]);
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: 6,
        residual_depth: 3,
        input_declarations: 1,
        potential_value_edges: 1,
        retained_expression_text_bytes: 73,
        ..ExpressionResourceFacts::default()
    };
    let model = validate(ir).unwrap();
    let result = normalize_binding(
        &model,
        &symbol("result"),
        ExpressionLimits::hard(),
        &CancellationToken::new(),
    )
    .unwrap();
    let NormalizedBinding::Residual(body) = result else {
        panic!("deferred guard must remain residual");
    };
    assert_eq!(&body, model.ir().declarations[1].body.as_ref().unwrap());
}

#[test]
fn unconditional_static_operands_fold_inside_a_deferred_expression() {
    let number = |digits| Expression {
        ty: T::Num,
        node: ExpressionNode::Value(V::Number(
            neutral_ir::ExactNumber::from_unsigned_integer(digits).unwrap(),
        )),
    };
    let mut ir = project(vec![
        binding("input", T::Num, ExpressionBody::Input),
        binding(
            "result",
            T::Num,
            ExpressionBody::Expression(Expression {
                ty: T::Num,
                node: ExpressionNode::Binary {
                    operator: BinaryOperator::Add,
                    left: Box::new(Expression {
                        ty: T::Num,
                        node: ExpressionNode::Input(symbol("input")),
                    }),
                    right: Box::new(Expression {
                        ty: T::Num,
                        node: ExpressionNode::Binary {
                            operator: BinaryOperator::Add,
                            left: Box::new(number("1")),
                            right: Box::new(number("2")),
                        },
                    }),
                },
            }),
        ),
    ]);
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: 5,
        residual_depth: 3,
        input_declarations: 1,
        potential_value_edges: 1,
        retained_expression_text_bytes: 67,
        ..ExpressionResourceFacts::default()
    };
    let model = validate(ir).unwrap();
    let result = normalize_binding(
        &model,
        &symbol("result"),
        ExpressionLimits {
            dependency_edges: 1,
            ..ExpressionLimits::hard()
        },
        &CancellationToken::new(),
    )
    .unwrap();
    let NormalizedBinding::Residual(ExpressionBody::Expression(Expression {
        node: ExpressionNode::Binary { left, right, .. },
        ..
    })) = result
    else {
        panic!("residual addition expected");
    };
    assert_eq!(left.node, ExpressionNode::Input(symbol("input")));
    assert_eq!(*right, number("3"));
}

#[test]
fn static_guard_selects_only_its_deferred_branch() {
    let number = |digits| Expression {
        ty: T::Num,
        node: ExpressionNode::Value(V::Number(
            neutral_ir::ExactNumber::from_unsigned_integer(digits).unwrap(),
        )),
    };
    let mut ir = project(vec![
        binding("input", T::Num, ExpressionBody::Input),
        binding(
            "result",
            T::Num,
            ExpressionBody::Expression(Expression {
                ty: T::Num,
                node: ExpressionNode::If {
                    guard: Box::new(leaf(true)),
                    then_branch: Box::new(Expression {
                        ty: T::Num,
                        node: ExpressionNode::Input(symbol("input")),
                    }),
                    else_branch: Box::new(Expression {
                        ty: T::Num,
                        node: ExpressionNode::Binary {
                            operator: BinaryOperator::Divide,
                            left: Box::new(number("1")),
                            right: Box::new(number("0")),
                        },
                    }),
                },
            }),
        ),
    ]);
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: 6,
        residual_depth: 3,
        input_declarations: 1,
        potential_value_edges: 1,
        retained_expression_text_bytes: 75,
        ..ExpressionResourceFacts::default()
    };
    let model = validate(ir).unwrap();
    assert_eq!(
        normalize_binding(
            &model,
            &symbol("result"),
            ExpressionLimits::hard(),
            &CancellationToken::new()
        ),
        Ok(NormalizedBinding::Residual(ExpressionBody::Expression(
            Expression {
                ty: T::Num,
                node: ExpressionNode::Input(symbol("input"))
            }
        )))
    );
}

fn deferred_match_ir() -> ExpressionProjectIr {
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
        binding("ask", T::Bool, ExpressionBody::Input),
        binding(
            "result",
            T::Bool,
            ExpressionBody::Expression(Expression {
                ty: T::Bool,
                node: ExpressionNode::Match {
                    scrutinee: Box::new(Expression {
                        ty: T::Nominal(symbol("State")),
                        node: ExpressionNode::Value(V::Variant {
                            tag: "ready".into(),
                            payload: Box::new(V::Bool(true)),
                        }),
                    }),
                    arms: vec![
                        MatchArm {
                            tag: "failed".into(),
                            payload_type: T::Bool,
                            body: leaf(false),
                        },
                        MatchArm {
                            tag: "ready".into(),
                            payload_type: T::Bool,
                            body: Expression {
                                ty: T::Bool,
                                node: ExpressionNode::If {
                                    guard: Box::new(Expression {
                                        ty: T::Bool,
                                        node: ExpressionNode::Input(symbol("ask")),
                                    }),
                                    then_branch: Box::new(Expression {
                                        ty: T::Bool,
                                        node: ExpressionNode::Bound { depth: 0, slot: 0 },
                                    }),
                                    else_branch: Box::new(leaf(false)),
                                },
                            },
                        },
                    ],
                },
            }),
        ),
    ]);
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: 7,
        residual_depth: 3,
        input_declarations: 1,
        potential_value_edges: 1,
        retained_expression_text_bytes: 134,
        ..ExpressionResourceFacts::default()
    };
    ir
}

#[test]
fn folded_match_closes_payload_bindings_inside_unresolved_guards_and_recovers() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    let model = validate(deferred_match_ir()).unwrap();
    let selected = symbol("result");
    let cancel = CancellationToken::new();
    let (result, count) = observe(None, || {
        normalize_binding(&model, &selected, ExpressionLimits::hard(), &cancel)
    });
    assert_eq!(
        result,
        Ok(NormalizedBinding::Residual(ExpressionBody::Expression(
            Expression {
                ty: T::Bool,
                node: ExpressionNode::If {
                    guard: Box::new(Expression {
                        ty: T::Bool,
                        node: ExpressionNode::Input(symbol("ask"))
                    }),
                    then_branch: Box::new(leaf(true)),
                    else_branch: Box::new(leaf(false)),
                }
            }
        )))
    );
    assert_ne!(count, 0);
    for index in 0..count {
        let (result, _) = observe(Some(index), || {
            normalize_binding(&model, &selected, ExpressionLimits::hard(), &cancel)
        });
        assert_eq!(result, Err(neutral_semantics::EvaluationError::Limit));
        let signal = CancellationToken::new();
        let (result, _) = observe_cancellation(index, &signal, || {
            normalize_binding(&model, &selected, ExpressionLimits::hard(), &signal)
        });
        assert_eq!(result, Err(neutral_semantics::EvaluationError::Cancelled));
        assert!(normalize_binding(&model, &selected, ExpressionLimits::hard(), &cancel).is_ok());
    }
}

#[test]
fn eliminated_outer_match_substitution_preserves_retained_inner_match_frames() {
    let make = || {
        let mut ir = deferred_match_ir();
        ir.declarations[1].signature = S::Binding(T::Nominal(symbol("State")));
        let Some(ExpressionBody::Expression(Expression {
            node: ExpressionNode::Match { arms, .. },
            ..
        })) = &mut ir.declarations[2].body
        else {
            panic!("outer match expected");
        };
        arms[1].body = Expression {
            ty: T::Bool,
            node: ExpressionNode::Match {
                scrutinee: Box::new(Expression {
                    ty: T::Nominal(symbol("State")),
                    node: ExpressionNode::Input(symbol("ask")),
                }),
                arms: vec![
                    MatchArm {
                        tag: "failed".into(),
                        payload_type: T::Bool,
                        body: Expression {
                            ty: T::Bool,
                            node: ExpressionNode::Bound { depth: 0, slot: 0 },
                        },
                    },
                    MatchArm {
                        tag: "ready".into(),
                        payload_type: T::Bool,
                        body: Expression {
                            ty: T::Bool,
                            node: ExpressionNode::Bound { depth: 1, slot: 0 },
                        },
                    },
                ],
            },
        };
        ir.expression_resources.retained_expression_text_bytes = 170;
        ir
    };
    let model = validate(make()).unwrap();
    let NormalizedBinding::Residual(body) = normalize_binding(
        &model,
        &symbol("result"),
        ExpressionLimits::hard(),
        &CancellationToken::new(),
    )
    .unwrap() else {
        panic!("inner match remains deferred");
    };
    let ExpressionBody::Expression(Expression {
        node: ExpressionNode::Match { arms, .. },
        ..
    }) = &body
    else {
        panic!("residual match expected");
    };
    assert_eq!(
        arms[0].body.node,
        ExpressionNode::Bound { depth: 0, slot: 0 }
    );
    assert_eq!(arms[1].body, leaf(true));
    let mut normalized = make();
    normalized.declarations[2].body = Some(body);
    normalized.expression_resources.residual_nodes = 4;
    normalized.expression_resources.residual_depth = 2;
    normalized
        .expression_resources
        .retained_expression_text_bytes = 90;
    assert!(validate(normalized).is_ok());
}
