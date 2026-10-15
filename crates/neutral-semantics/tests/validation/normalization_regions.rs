// SPDX-License-Identifier: Apache-2.0

//! Partial folding must retain unresolved inputs while simplifying unconditional children.

use super::*;
use neutral_core::allocation::testing::{observe, observe_cancellation};

/// Creates an independently specified numeric leaf.
fn number(value: &str) -> Expression {
    Expression {
        ty: T::Num,
        node: ExpressionNode::Value(V::Number(
            neutral_ir::ExactNumber::from_source(value, 64, 64).unwrap(),
        )),
    }
}

/// References one explicit input without supplying consumer data during producer folding.
fn input(name: &str, ty: T) -> Expression {
    Expression {
        ty,
        node: ExpressionNode::Input(symbol(name)),
    }
}

/// A constant child whose independently expected folded value is three.
fn addition() -> Expression {
    Expression {
        ty: T::Num,
        node: ExpressionNode::Binary {
            operator: BinaryOperator::Add,
            left: Box::new(number("1")),
            right: Box::new(number("2")),
        },
    }
}

/// Keeps complete record and variant contracts beside both scalar input kinds.
fn model(body: Expression) -> neutral_semantics::ValidatedExpressionModel {
    let mut ir = project(vec![
        D {
            identity: symbol("Choice"),
            public: true,
            signature: S::Definition(CompositionBody::Variant(vec![
                neutral_ir::composition::CompositionAlternative {
                    tag: "ready".into(),
                    ty: T::Num,
                },
            ])),
            body: None,
        },
        D {
            identity: symbol("Row"),
            public: true,
            signature: S::Definition(CompositionBody::Record(vec![
                CompositionField {
                    name: "first".into(),
                    ty: T::Num,
                    presence: FieldPresence::Required,
                    restrictions: FieldRestrictions::default(),
                    default: None,
                },
                CompositionField {
                    name: "second".into(),
                    ty: T::Num,
                    presence: FieldPresence::Required,
                    restrictions: FieldRestrictions::default(),
                    default: None,
                },
            ])),
            body: None,
        },
        binding("bool_input", T::Bool, ExpressionBody::Input),
        binding("number_input", T::Num, ExpressionBody::Input),
        binding("result", body.ty.clone(), ExpressionBody::Expression(body)),
    ]);
    ir.expression_resources =
        neutral_semantics::derive_expression_resources(&ir, controls(), &CancellationToken::new())
            .unwrap();
    validate(ir).unwrap()
}

/// Independently specifies each residual shape and its unconditional folded child.
fn cases() -> Vec<(Expression, Expression)> {
    let unary = || Expression {
        ty: T::Bool,
        node: ExpressionNode::Unary {
            operator: UnaryOperator::Not,
            operand: Box::new(input("bool_input", T::Bool)),
        },
    };
    let text = || Expression {
        ty: T::String,
        node: ExpressionNode::Text(Box::new(input("number_input", T::Num))),
    };
    let list = |last| Expression {
        ty: T::List(Box::new(T::Num)),
        node: ExpressionNode::List(vec![input("number_input", T::Num), last]),
    };
    let record = |last| Expression {
        ty: T::Nominal(symbol("Row")),
        node: ExpressionNode::Record(vec![
            ("first".into(), Some(input("number_input", T::Num))),
            ("second".into(), Some(last)),
        ]),
    };
    let variant = || Expression {
        ty: T::Nominal(symbol("Choice")),
        node: ExpressionNode::Variant {
            tag: "ready".into(),
            payload: Box::new(input("number_input", T::Num)),
        },
    };
    let interpolate = || Expression {
        ty: T::String,
        node: ExpressionNode::Interpolate(vec![
            text(),
            Expression {
                ty: T::String,
                node: ExpressionNode::Value(V::String("!".into())),
            },
        ]),
    };
    let mut cases = vec![
        (unary(), unary()),
        (text(), text()),
        (list(addition()), list(number("3"))),
        (record(addition()), record(number("3"))),
        (variant(), variant()),
        (interpolate(), interpolate()),
    ];
    for (operator, known) in [(BinaryOperator::And, true), (BinaryOperator::Or, false)] {
        cases.push((
            Expression {
                ty: T::Bool,
                node: ExpressionNode::Binary {
                    operator,
                    left: Box::new(leaf(known)),
                    right: Box::new(input("bool_input", T::Bool)),
                },
            },
            input("bool_input", T::Bool),
        ));
    }
    cases.push((
        Expression {
            ty: T::Num,
            node: ExpressionNode::If {
                guard: Box::new(leaf(false)),
                then_branch: Box::new(number("99")),
                else_branch: Box::new(input("number_input", T::Num)),
            },
        },
        input("number_input", T::Num),
    ));
    cases
}

/// Pure folding retains every unresolved shape and fails atomically at every allocation boundary.
#[test]
fn residual_shapes_fold_only_unconditional_children_and_recover_after_faults() {
    for (body, expected) in cases() {
        let model = model(body);
        let selected = symbol("result");
        let cancel = CancellationToken::new();
        let run = |signal: &CancellationToken| {
            normalize_binding(&model, &selected, ExpressionLimits::hard(), signal)
        };
        let (result, checkpoints) = observe(None, || run(&cancel));
        assert_eq!(
            result,
            Ok(NormalizedBinding::Residual(ExpressionBody::Expression(
                expected
            )))
        );
        assert!(checkpoints > 0);
        for index in 0..checkpoints {
            assert_eq!(
                observe(Some(index), || run(&cancel)).0,
                Err(neutral_semantics::EvaluationError::Limit),
                "allocation {index}"
            );
            let signal = CancellationToken::new();
            assert_eq!(
                observe_cancellation(index, &signal, || run(&signal)).0,
                Err(neutral_semantics::EvaluationError::Cancelled),
                "cancellation {index}"
            );
        }
        assert!(run(&cancel).is_ok());
    }
}

/// A retained guarded child can contain a reference to an eliminated match payload at any shape.
fn payload_tree(kind: usize, retained: bool) -> Expression {
    let payload = || {
        if retained {
            leaf(true)
        } else {
            Expression {
                ty: T::Bool,
                node: ExpressionNode::Bound { depth: 0, slot: 0 },
            }
        }
    };
    let (ty, node) = match kind {
        0 => (
            T::Bool,
            ExpressionNode::Unary {
                operator: UnaryOperator::Not,
                operand: Box::new(payload()),
            },
        ),
        1 => (
            T::Bool,
            ExpressionNode::Binary {
                operator: BinaryOperator::Equal,
                left: Box::new(payload()),
                right: Box::new(leaf(false)),
            },
        ),
        2 => (T::String, ExpressionNode::Text(Box::new(payload()))),
        3 => (
            T::List(Box::new(T::Bool)),
            ExpressionNode::List(vec![payload(), leaf(false)]),
        ),
        4 => (
            T::Nominal(symbol("Row")),
            ExpressionNode::Record(vec![("flag".into(), Some(payload()))]),
        ),
        5 => (
            T::Nominal(symbol("State")),
            ExpressionNode::Variant {
                tag: "ready".into(),
                payload: Box::new(payload()),
            },
        ),
        6 => (
            T::String,
            ExpressionNode::Interpolate(vec![Expression {
                ty: T::String,
                node: ExpressionNode::Text(Box::new(payload())),
            }]),
        ),
        _ => (
            T::Bool,
            ExpressionNode::If {
                guard: Box::new(payload()),
                then_branch: Box::new(leaf(false)),
                else_branch: Box::new(leaf(true)),
            },
        ),
    };
    Expression { ty, node }
}

/// Unknown guards retain their branches while all references to the removed outer frame are closed.
#[test]
fn eliminated_match_payloads_are_substituted_through_every_retained_shape() {
    for kind in 0..8 {
        let guarded = |retained| {
            let child = payload_tree(kind, retained);
            Expression {
                ty: child.ty.clone(),
                node: ExpressionNode::If {
                    guard: Box::new(input("ask", T::Bool)),
                    then_branch: Box::new(child),
                    else_branch: Box::new(payload_tree(kind, retained)),
                },
            }
        };
        let mut ir = deferred_match_ir();
        let result = &mut ir.declarations[2];
        let ty = payload_tree(kind, false).ty;
        result.signature = S::Binding(ty.clone());
        let Some(ExpressionBody::Expression(root)) = &mut result.body else {
            panic!("match body");
        };
        root.ty = ty;
        let ExpressionNode::Match { arms, .. } = &mut root.node else {
            panic!("match expression");
        };
        for arm in arms {
            arm.body = guarded(false);
        }
        if kind == 4 {
            ir.declarations.insert(
                0,
                D {
                    identity: symbol("Row"),
                    public: true,
                    signature: S::Definition(CompositionBody::Record(vec![CompositionField {
                        name: "flag".into(),
                        ty: T::Bool,
                        presence: FieldPresence::Required,
                        restrictions: FieldRestrictions::default(),
                        default: None,
                    }])),
                    body: None,
                },
            );
        }
        ir.expression_resources = neutral_semantics::derive_expression_resources(
            &ir,
            controls(),
            &CancellationToken::new(),
        )
        .unwrap();
        let model = validate(ir).unwrap();
        assert_eq!(
            normalize_binding(
                &model,
                &symbol("result"),
                ExpressionLimits::hard(),
                &CancellationToken::new()
            ),
            Ok(NormalizedBinding::Residual(ExpressionBody::Expression(
                guarded(true)
            ))),
            "shape {kind}"
        );
    }
}
