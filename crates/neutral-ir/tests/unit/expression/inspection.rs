// SPDX-License-Identifier: Apache-2.0

//! Shape inspection rejects before retention and never short-circuits retained branches.

use super::*;
use neutral_core::{CancellationToken, allocation::testing::observe};
use neutral_ir::expression::profile::ExpressionLimits;

#[test]
fn inspection_exposes_aggregate_retention_cost_without_allocation() {
    let tree = leaf(true);
    let (result, allocations) = observe(None, || {
        inspect_expression_retention(&tree, ExpressionLimits::hard(), &CancellationToken::new())
    });
    let cost = result.unwrap();
    assert_eq!(cost.shape.nodes, 1);
    assert_eq!(
        cost.retained_bytes,
        std::mem::size_of::<Expression>() as u64
    );
    assert!(cost.work >= cost.retained_bytes);
    assert_eq!(allocations, 0);
}

fn inspect(
    tree: &Expression,
    limits: ExpressionLimits,
) -> Result<ExpressionShape, ExpressionShapeError> {
    inspect_expression_shape(tree, limits, &CancellationToken::new())
}

#[test]
fn inspection_counts_skipped_branches() {
    let tree = Expression {
        ty: T::Bool,
        node: ExpressionNode::If {
            guard: Box::new(leaf(true)),
            then_branch: Box::new(leaf(false)),
            else_branch: Box::new(leaf(true)),
        },
    };
    assert_eq!(
        inspect(&tree, ExpressionLimits::hard()).unwrap(),
        ExpressionShape {
            nodes: 4,
            depth: 2,
            output_leaves: 0
        }
    );
    for limits in [
        ExpressionLimits {
            nodes: 3,
            ..ExpressionLimits::hard()
        },
        ExpressionLimits {
            evaluation_depth: 1,
            ..ExpressionLimits::hard()
        },
    ] {
        assert_eq!(inspect(&tree, limits), Err(ExpressionShapeError::Limit));
    }
    assert!(
        inspect(
            &tree,
            ExpressionLimits {
                nodes: 4,
                evaluation_depth: 2,
                syntax_depth: 2,
                ..ExpressionLimits::hard()
            }
        )
        .is_ok()
    );
}

#[test]
fn inspection_exact_and_one_over_controls() {
    let tree = Expression {
        ty: T::String,
        node: ExpressionNode::Value(V::String("abc".into())),
    };
    let bytes = std::mem::size_of::<Expression>() as u64 + 3;
    for field in [0, 1] {
        let mut limits = ExpressionLimits::hard();
        if field == 0 {
            limits.retained_bytes = bytes;
        } else {
            limits.output_bytes = bytes;
        }
        assert!(inspect(&tree, limits).is_ok());
        if field == 0 {
            limits.retained_bytes -= 1;
        } else {
            limits.output_bytes -= 1;
        }
        assert_eq!(inspect(&tree, limits), Err(ExpressionShapeError::Limit));
    }
    assert_eq!(
        inspect(
            &tree,
            ExpressionLimits {
                work: 1,
                ..ExpressionLimits::hard()
            }
        ),
        Err(ExpressionShapeError::Limit)
    );
    assert_eq!(
        inspect(
            &tree,
            ExpressionLimits {
                nodes: 0,
                ..ExpressionLimits::hard()
            }
        ),
        Err(ExpressionShapeError::Limit)
    );
    let number = Expression {
        ty: T::Num,
        node: ExpressionNode::Value(V::Number(
            neutral_ir::ExactNumber::from_normalized_parts(false, "123", -2, 3, 2).unwrap(),
        )),
    };
    assert!(
        inspect(
            &number,
            ExpressionLimits {
                coefficient_digits: 3,
                absolute_scale: 2,
                ..ExpressionLimits::hard()
            }
        )
        .is_ok()
    );
    for limits in [
        ExpressionLimits {
            coefficient_digits: 2,
            ..ExpressionLimits::hard()
        },
        ExpressionLimits {
            absolute_scale: 1,
            ..ExpressionLimits::hard()
        },
    ] {
        assert_eq!(inspect(&number, limits), Err(ExpressionShapeError::Limit));
    }
}

#[test]
fn inspection_rejects_noncanonical_shapes() {
    for names in [["b", "a"], ["a", "a"]] {
        let tree = Expression {
            ty: T::Nominal(symbol("Record")),
            node: ExpressionNode::Record(names.into_iter().map(|n| (n.into(), None)).collect()),
        };
        assert_eq!(
            inspect(&tree, ExpressionLimits::hard()),
            Err(ExpressionShapeError::Malformed)
        );
        let tree = Expression {
            ty: T::Bool,
            node: ExpressionNode::Match {
                scrutinee: Box::new(leaf(true)),
                arms: names
                    .into_iter()
                    .map(|n| MatchArm {
                        tag: n.into(),
                        payload_type: T::Bool,
                        body: leaf(true),
                    })
                    .collect(),
            },
        };
        assert_eq!(
            inspect(&tree, ExpressionLimits::hard()),
            Err(ExpressionShapeError::Malformed)
        );
    }
}

#[test]
fn inspection_bounds_nested_values_types_and_selector_edges() {
    let mut tree = leaf(true);
    for _ in 0..65 {
        tree.ty = T::List(Box::new(tree.ty));
    }
    assert_eq!(
        inspect(&tree, ExpressionLimits::hard()),
        Err(ExpressionShapeError::Limit)
    );
    let mut value = V::Bool(true);
    for _ in 0..65 {
        value = V::List(vec![value]);
    }
    tree = Expression {
        ty: T::Bool,
        node: ExpressionNode::Value(value),
    };
    assert_eq!(
        inspect(&tree, ExpressionLimits::hard()),
        Err(ExpressionShapeError::Limit)
    );
    tree = Expression {
        ty: T::Bool,
        node: ExpressionNode::List(vec![leaf(true), leaf(false)]),
    };
    assert_eq!(
        inspect(
            &tree,
            ExpressionLimits {
                nodes: 2,
                ..ExpressionLimits::hard()
            }
        ),
        Err(ExpressionShapeError::Limit)
    );
    tree = Expression {
        ty: T::Bool,
        node: ExpressionNode::Output {
            target: symbol("op"),
            result_type: T::Nominal(symbol("Result")),
            selector: (0..64).map(SelectorSegment::Index).collect(),
        },
    };
    assert_eq!(
        inspect(&tree, ExpressionLimits::hard())
            .unwrap()
            .output_leaves,
        1
    );
    let ExpressionNode::Output { selector, .. } = &mut tree.node else {
        unreachable!()
    };
    selector.push(SelectorSegment::Index(64));
    assert_eq!(
        inspect(&tree, ExpressionLimits::hard()),
        Err(ExpressionShapeError::Limit)
    );
}

#[test]
fn inspection_is_allocation_free_and_cancellable() {
    let tree = leaf(true);
    let cancel = CancellationToken::new();
    let (result, reservations) = observe(None, || {
        inspect_expression_shape(&tree, ExpressionLimits::hard(), &cancel)
    });
    assert!(result.is_ok());
    assert_eq!(reservations, 0);
    cancel.cancel();
    assert_eq!(
        inspect_expression_shape(&tree, ExpressionLimits::hard(), &cancel),
        Err(ExpressionShapeError::Cancelled)
    );
    assert!(inspect(&tree, ExpressionLimits::hard()).is_ok());
}
