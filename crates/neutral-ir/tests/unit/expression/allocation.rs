// SPDX-License-Identifier: Apache-2.0

//! Fallible copies preserve retained semantics and reject every injected fault atomically.

use super::*;
use neutral_core::{
    CancellationToken,
    allocation::testing::{observe, observe_cancellation},
};
use neutral_ir::expression::profile::ExpressionLimits;

/// All raw node shapes, including omitted fields and unselected bodies.
fn all_nodes() -> Expression {
    let child = || Box::new(leaf(true));
    let nodes = vec![
        ExpressionNode::Value(V::Record(vec![
            ("a".into(), None),
            (
                "b".into(),
                Some(V::Variant {
                    tag: "ready".into(),
                    payload: Box::new(V::List(vec![
                        V::String("literal".into()),
                        V::Reference(symbol("ref_target")),
                    ])),
                }),
            ),
        ])),
        ExpressionNode::Input(symbol("external")),
        ExpressionNode::Output {
            target: symbol("op"),
            result_type: T::Nominal(symbol("Result")),
            selector: vec![
                SelectorSegment::Field("items".into()),
                SelectorSegment::Index(u32::MAX),
                SelectorSegment::Alternative("ready".into()),
            ],
        },
        ExpressionNode::Unary {
            operator: UnaryOperator::Not,
            operand: child(),
        },
        ExpressionNode::Binary {
            operator: BinaryOperator::And,
            left: child(),
            right: Box::new(leaf(false)),
        },
        ExpressionNode::If {
            guard: child(),
            then_branch: child(),
            else_branch: Box::new(leaf(false)),
        },
        ExpressionNode::Match {
            scrutinee: child(),
            arms: vec![MatchArm {
                tag: "ready".into(),
                payload_type: T::String,
                body: Expression {
                    ty: T::String,
                    node: ExpressionNode::Bound { depth: 0, slot: 0 },
                },
            }],
        },
        ExpressionNode::Text(child()),
        ExpressionNode::Interpolate(vec![Expression {
            ty: T::String,
            node: ExpressionNode::Value(V::String("message".into())),
        }]),
        ExpressionNode::List(vec![leaf(false), leaf(true)]),
        ExpressionNode::Record(vec![
            ("absent".into(), None),
            ("present".into(), Some(leaf(true))),
        ]),
        ExpressionNode::Variant {
            tag: "ready".into(),
            payload: child(),
        },
        ExpressionNode::Bound { depth: 3, slot: 7 },
    ];
    let nominal = T::VocabularyNominal {
        identity: "Domain".into(),
        version: "1.0.0".into(),
        name: "Value".into(),
    };
    Expression {
        ty: T::List(Box::new(T::Nullable(Box::new(T::Ref(Box::new(nominal)))))),
        node: ExpressionNode::List(
            nodes
                .into_iter()
                .map(|node| Expression { ty: T::Bool, node })
                .collect(),
        ),
    }
}

#[test]
fn copy_preserves_every_node_kind() {
    // The raw shape is deliberately not semantically typed: copy cannot silently retype or evaluate it.
    let tree = all_nodes();
    let copy = tree
        .try_clone_bounded(ExpressionLimits::hard(), &CancellationToken::new())
        .unwrap();
    assert_eq!(copy, tree);
    let ExpressionNode::List(items) = copy.node else {
        panic!("retained list required")
    };
    assert_eq!(items.len(), 13);
    assert_eq!(items[12].node, ExpressionNode::Bound { depth: 3, slot: 7 });
    let ExpressionNode::Binary { left, right, .. } = &items[4].node else {
        panic!("binary required")
    };
    assert_eq!(**left, leaf(true));
    assert_eq!(**right, leaf(false));
}

#[test]
fn copy_faults_and_cancellation_are_atomic() {
    let tree = all_nodes();
    let limits = ExpressionLimits::hard();
    let cancel = CancellationToken::new();
    let (ok, reservations) = observe(None, || tree.try_clone_bounded(limits, &cancel));
    assert_eq!(ok.unwrap(), tree);
    assert_ne!(reservations, 0);
    for index in 0..reservations {
        let (failed, _) = observe(Some(index), || tree.try_clone_bounded(limits, &cancel));
        assert_eq!(
            failed.unwrap_err(),
            ExpressionShapeError::Limit,
            "reservation {index}"
        );
        let signal = CancellationToken::new();
        let (failed, _) =
            observe_cancellation(index, &signal, || tree.try_clone_bounded(limits, &signal));
        assert_eq!(
            failed.unwrap_err(),
            ExpressionShapeError::Cancelled,
            "cancellation {index}"
        );
        assert_eq!(tree.try_clone_bounded(limits, &cancel).unwrap(), tree);
    }
}

#[test]
fn copy_cancellation_stops_before_literal_children_are_reserved() {
    let tree = Expression {
        ty: T::List(Box::new(T::String)),
        node: ExpressionNode::Value(V::List(vec![V::String("large literal".into()); 128])),
    };
    // The type wrapper reserves once; cancellation at the literal's outer vector is next.
    let signal = CancellationToken::new();
    let (failed, count) = observe_cancellation(1, &signal, || {
        tree.try_clone_bounded(ExpressionLimits::hard(), &signal)
    });
    assert_eq!(failed.unwrap_err(), ExpressionShapeError::Cancelled);
    assert_eq!(count, 2, "no child reservation after cancellation");
}

#[test]
fn copy_preflight_rejects_before_any_reservation() {
    let mut tree = leaf(true);
    for _ in 0..65 {
        tree = Expression {
            ty: T::Bool,
            node: ExpressionNode::Text(Box::new(tree)),
        };
    }
    let cancel = CancellationToken::new();
    let (failed, count) = observe(None, || {
        tree.try_clone_bounded(ExpressionLimits::hard(), &cancel)
    });
    assert_eq!(failed.unwrap_err(), ExpressionShapeError::Limit);
    assert_eq!(count, 0);
    let tree = all_nodes();
    let (failed, count) = observe(None, || {
        tree.try_clone_bounded(
            ExpressionLimits {
                output_bytes: 1,
                ..ExpressionLimits::hard()
            },
            &cancel,
        )
    });
    assert_eq!(failed.unwrap_err(), ExpressionShapeError::Limit);
    assert_eq!(count, 0);
}
