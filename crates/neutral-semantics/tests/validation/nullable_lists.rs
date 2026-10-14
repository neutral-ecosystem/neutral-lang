// SPDX-License-Identifier: Apache-2.0

//! Contextual nullable lists retain exact annotations and enforce their element contract.

use super::*;
use neutral_ir::composition::BindingValue as V;
use neutral_semantics::{
    Binding, BindingKey, BindingState, EvaluationRequest, EvaluationResult as R, NormalizedBinding,
    evaluate, normalize_binding,
};

/// Catches rejecting an explicitly typed non-null empty nullable list before normalization.
#[test]
fn nullable_empty_list_retains_its_declared_type() {
    let ty = T::Nullable(Box::new(T::List(Box::new(T::Num))));
    let mut ir = project(vec![binding(
        "result",
        ty.clone(),
        ExpressionBody::Expression(Expression {
            ty: ty.clone(),
            node: ExpressionNode::List(vec![]),
        }),
    )]);
    // list kind 4 + nullable tag 8 + list tag 4 + num tag 3.
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: 1,
        residual_depth: 1,
        retained_expression_text_bytes: 19,
        ..ExpressionResourceFacts::default()
    };
    let model = validate(ir).unwrap();
    assert_eq!(model.facts().values(), [1, 1, 0, 0, 0, 0, 19]);
    assert_eq!(
        normalize_binding(
            &model,
            &symbol("result"),
            ExpressionLimits::hard(),
            &CancellationToken::new()
        ),
        Ok(NormalizedBinding::Value(V::List(vec![])))
    );
    assert_eq!(
        evaluate(
            &model,
            &EvaluationRequest {
                selected: &symbol("result"),
                profiles: &model.ir().semantic_profiles,
                bindings: &[],
                associations: &[],
                limits: ExpressionLimits::hard()
            },
            &CancellationToken::new()
        ),
        R::Value {
            ty: &ty,
            value: V::List(vec![])
        }
    );
}

/// Catches stripping deferred nullable-list annotations or accepting the wrong element type.
#[test]
fn nullable_list_with_deferred_elements_requires_the_exact_inner_type() {
    let ty = T::Nullable(Box::new(T::List(Box::new(T::Bool))));
    let make = |element: T| {
        let mut ir = project(vec![
            binding("external", element.clone(), ExpressionBody::Input),
            binding(
                "result",
                ty.clone(),
                ExpressionBody::Expression(Expression {
                    ty: ty.clone(),
                    node: ExpressionNode::List(vec![Expression {
                        ty: element,
                        node: ExpressionNode::Input(symbol("external")),
                    }]),
                }),
            ),
        ]);
        // nullable-list-bool constructor 20 + input-bool symbol 27.
        ir.expression_resources = ExpressionResourceFacts {
            residual_nodes: 2,
            residual_depth: 2,
            input_declarations: 1,
            potential_value_edges: 1,
            retained_expression_text_bytes: 47,
            ..ExpressionResourceFacts::default()
        };
        ir
    };
    assert_eq!(validate(make(T::String)).unwrap_err(), E::TypeMismatch);
    let model = validate(make(T::Bool)).unwrap();
    let external = symbol("external");
    let result = symbol("result");
    let bindings = [Binding {
        key: BindingKey::Input(&external),
        ty: &T::Bool,
        state: BindingState::Value(&V::Bool(true)),
    }];
    let mut request = EvaluationRequest {
        selected: &result,
        profiles: &model.ir().semantic_profiles,
        bindings: &[],
        associations: &[],
        limits: ExpressionLimits::hard(),
    };
    assert_eq!(
        evaluate(&model, &request, &CancellationToken::new()),
        R::RequiredBindings(vec![BindingKey::Input(&external)])
    );
    request.bindings = &bindings;
    assert_eq!(
        evaluate(&model, &request, &CancellationToken::new()),
        R::Value {
            ty: &ty,
            value: V::List(vec![V::Bool(true)])
        }
    );
}

/// Catches treating arbitrary nullable primitives as valid list constructor annotations.
#[test]
fn nullable_non_list_annotations_reject_list_constructors() {
    for inner in [T::Bool, T::String, T::Num] {
        let ty = T::Nullable(Box::new(inner));
        let ir = project(vec![binding(
            "result",
            ty.clone(),
            ExpressionBody::Expression(Expression {
                ty,
                node: ExpressionNode::List(vec![]),
            }),
        )]);
        assert_eq!(validate(ir).unwrap_err(), E::TypeMismatch);
    }
}
