// SPDX-License-Identifier: Apache-2.0

//! Independent numeric/output vectors and complete binding-generation qualification.

use super::*;
use neutral_core::allocation::testing::{observe, observe_cancellation};
use neutral_ir::{ExactNumber, composition::BindingValue as V};
use neutral_semantics::{
    Binding, BindingKey, BindingState, EvaluationError as F, EvaluationRequest,
    EvaluationResult as R, NormalizedBinding, ValidatedExpressionModel, evaluate,
    normalize_binding,
};

/// Exact source-independent fixture value, never host floating-point expectations.
fn number(literal: &str) -> V {
    V::Number(ExactNumber::from_source(literal, 4096, 4096).unwrap())
}

/// Types the same numeric operator using static leaves or separately declared explicit inputs.
fn numeric_model(
    operator: BinaryOperator,
    a: &str,
    b: &str,
    deferred: bool,
) -> ValidatedExpressionModel {
    let operand = |name, literal| Expression {
        ty: T::Num,
        node: if deferred {
            ExpressionNode::Input(symbol(name))
        } else {
            ExpressionNode::Value(number(literal))
        },
    };
    let result = binding(
        "result",
        T::Num,
        ExpressionBody::Expression(Expression {
            ty: T::Num,
            node: ExpressionNode::Binary {
                operator,
                left: Box::new(operand("left", a)),
                right: Box::new(operand("right", b)),
            },
        }),
    );
    let declarations = if deferred {
        vec![
            binding("left", T::Num, ExpressionBody::Input),
            result,
            binding("right", T::Num, ExpressionBody::Input),
        ]
    } else {
        vec![result]
    };
    validate_expression_model(
        qualification::ready(project(declarations)),
        controls(),
        &CancellationToken::new(),
    )
    .unwrap()
}

/// Runs the public evaluator with a fresh explicit complete generation and no acquisition.
fn run<'m>(
    model: &'m ValidatedExpressionModel,
    bindings: &[Binding<'_>],
    limits: ExpressionLimits,
    cancel: &CancellationToken,
) -> R<'m> {
    evaluate(
        model,
        &EvaluationRequest {
            selected: &symbol("result"),
            profiles: &model.ir().semantic_profiles,
            bindings,
            associations: &[],
            limits,
        },
        cancel,
    )
}

#[test]
fn independent_arithmetic_vectors_match_static_folding_and_explicit_evaluation() {
    for (operator, a, b, expected) in [
        (BinaryOperator::Add, "0.1", "0.2", "0.3"),
        (BinaryOperator::Subtract, "-1.2", "3.4", "-4.6"),
        (BinaryOperator::Multiply, "-1.25", "0.8", "-1"),
        (BinaryOperator::Divide, "1", "8", "0.125"),
        (
            BinaryOperator::Divide,
            "1",
            "3",
            "0.3333333333333333333333333333333333",
        ),
        (
            BinaryOperator::Divide,
            "-1",
            "6",
            "-0.1666666666666666666666666666666667",
        ),
        (
            BinaryOperator::Divide,
            "99999999999999999999999999999999999998",
            "99999999999999999999999999999999999999",
            "1",
        ),
        (
            BinaryOperator::Divide,
            "1",
            "1099511627776",
            "0.0000000000009094947017729282379150390625",
        ),
    ] {
        let a_value = number(a);
        let b_value = number(b);
        let left = symbol("left");
        let right = symbol("right");
        let bindings = [
            Binding {
                key: BindingKey::Input(&left),
                ty: &T::Num,
                state: BindingState::Value(&a_value),
            },
            Binding {
                key: BindingKey::Input(&right),
                ty: &T::Num,
                state: BindingState::Value(&b_value),
            },
        ];
        let static_model = numeric_model(operator, a, b, false);
        assert_eq!(
            normalize_binding(
                &static_model,
                &symbol("result"),
                ExpressionLimits::hard(),
                &CancellationToken::new()
            ),
            Ok(NormalizedBinding::Value(number(expected)))
        );
        assert_eq!(
            run(
                &static_model,
                &[],
                ExpressionLimits::hard(),
                &CancellationToken::new()
            ),
            R::Value {
                ty: &T::Num,
                value: number(expected)
            }
        );
        let deferred_model = numeric_model(operator, a, b, true);
        assert_eq!(
            run(
                &deferred_model,
                &bindings,
                ExpressionLimits::hard(),
                &CancellationToken::new()
            ),
            R::Value {
                ty: &T::Num,
                value: number(expected)
            }
        );
    }
}

#[test]
fn every_request_control_rejects_zero_without_a_partial_value() {
    let model = numeric_model(BinaryOperator::Add, "1", "2", false);
    for select in 0..11 {
        let mut limits = ExpressionLimits::hard();
        match select {
            0 => limits.nodes = 0,
            1 => limits.syntax_depth = 0,
            2 => limits.evaluation_depth = 0,
            3 => limits.bindings = 0,
            4 => limits.dependency_edges = 0,
            5 => limits.work = 0,
            6 => limits.retained_bytes = 0,
            7 => limits.output_bytes = 0,
            8 => limits.coefficient_digits = 0,
            9 => limits.absolute_scale = 0,
            _ => limits.diagnostics = 0,
        }
        assert_eq!(
            run(&model, &[], limits, &CancellationToken::new()),
            R::Failure(F::Limit)
        );
    }
}

#[test]
fn evaluator_work_and_retention_have_inclusive_request_local_boundaries() {
    let model = numeric_model(BinaryOperator::Add, "1", "2", false);
    for retention in [false, true] {
        let mut low = 1;
        let mut high = if retention {
            ExpressionLimits::hard().retained_bytes
        } else {
            ExpressionLimits::hard().work
        };
        let check = |limit| {
            let mut limits = ExpressionLimits::hard();
            if retention {
                limits.retained_bytes = limit;
            } else {
                limits.work = limit;
            }
            run(&model, &[], limits, &CancellationToken::new())
        };
        while low < high {
            let mid = low + (high - low) / 2;
            match check(mid) {
                R::Value { value, .. } => {
                    assert_eq!(value, number("3"));
                    high = mid;
                }
                R::Failure(F::Limit) => low = mid + 1,
                result => panic!("unexpected result {result:?}"),
            }
        }
        assert!(low > 1);
        assert_eq!(
            check(low),
            R::Value {
                ty: &T::Num,
                value: number("3")
            }
        );
        assert_eq!(check(low - 1), R::Failure(F::Limit));
        assert_eq!(
            run(
                &model,
                &[],
                ExpressionLimits::hard(),
                &CancellationToken::new()
            ),
            R::Value {
                ty: &T::Num,
                value: number("3")
            }
        );
    }
}

#[test]
fn evaluated_utf8_output_boundary_matches_the_literal_oracle() {
    let result = Expression {
        ty: T::String,
        node: ExpressionNode::Binary {
            operator: BinaryOperator::Concat,
            left: Box::new(Expression {
                ty: T::String,
                node: ExpressionNode::Value(V::String("é".into())),
            }),
            right: Box::new(Expression {
                ty: T::String,
                node: ExpressionNode::Value(V::String("a".into())),
            }),
        },
    };
    let model = validate_expression_model(
        qualification::ready(project(vec![binding(
            "result",
            T::String,
            ExpressionBody::Expression(result),
        )])),
        controls(),
        &CancellationToken::new(),
    )
    .unwrap();
    for output_bytes in [2, 3] {
        let limits = ExpressionLimits {
            output_bytes,
            ..ExpressionLimits::hard()
        };
        let actual = run(&model, &[], limits, &CancellationToken::new());
        assert_eq!(
            actual,
            if output_bytes == 3 {
                R::Value {
                    ty: &T::String,
                    value: V::String("éa".into()),
                }
            } else {
                R::Failure(F::Limit)
            }
        );
    }
}

#[test]
fn typed_record_equality_distinguishes_omission_null_and_present_values() {
    use neutral_core::VocabularyContentDigest;
    use neutral_ir::{
        VocabularyIdentity,
        composition::{CompositionBundle, CompositionDefinition},
    };
    use neutral_vocabulary::composition::{ENCODING_VERSION, REQUIRED_FEATURE, SCHEMA_VERSION};
    let make = |left, right| {
        let ty = T::VocabularyNominal {
            identity: "Domain".into(),
            version: "1.0.0".into(),
            name: "Item".into(),
        };
        let definition = CompositionDefinition {
            name: "Item".into(),
            public: true,
            body: CompositionBody::Record(vec![CompositionField {
                name: "value".into(),
                ty: T::Nullable(Box::new(T::String)),
                presence: FieldPresence::Optional,
                restrictions: FieldRestrictions::default(),
                default: None,
            }]),
        };
        let result = binding(
            "result",
            T::Bool,
            ExpressionBody::Expression(Expression {
                ty: T::Bool,
                node: ExpressionNode::Binary {
                    operator: BinaryOperator::Equal,
                    left: Box::new(Expression {
                        ty: ty.clone(),
                        node: ExpressionNode::Value(V::Record(vec![("value".into(), left)])),
                    }),
                    right: Box::new(Expression {
                        ty,
                        node: ExpressionNode::Value(V::Record(vec![("value".into(), right)])),
                    }),
                },
            }),
        );
        let mut ir = project(vec![result]);
        ir.vocabularies.push(CompositionBundle {
            identity: VocabularyIdentity::new(
                "Domain",
                "1.0.0",
                SCHEMA_VERSION,
                ENCODING_VERSION,
                VocabularyContentDigest::from_bytes(b"optional record literal"),
                vec![REQUIRED_FEATURE.into()],
            ),
            dependencies: vec![],
            definitions: vec![definition],
        });
        validate_expression_model(
            qualification::ready(ir),
            controls(),
            &CancellationToken::new(),
        )
        .unwrap()
    };
    for (left, right, expected) in [
        (None, None, true),
        (None, Some(V::Null), false),
        (Some(V::Null), Some(V::Null), true),
        (Some(V::Null), Some(V::String(String::new())), false),
        (
            Some(V::String("é".into())),
            Some(V::String("e\u{301}".into())),
            false,
        ),
    ] {
        let model = make(left, right);
        assert_eq!(
            run(
                &model,
                &[],
                ExpressionLimits::hard(),
                &CancellationToken::new()
            ),
            R::Value {
                ty: &T::Bool,
                value: V::Bool(expected)
            }
        );
    }
}

#[test]
fn evaluator_nodes_ports_edges_digits_and_scale_accept_exact_boundaries() {
    let model = numeric_model(BinaryOperator::Add, "1", "2", true);
    let left = symbol("left");
    let right = symbol("right");
    let a = number("1.23");
    let b = number("2");
    let bindings = [
        Binding {
            key: BindingKey::Input(&left),
            ty: &T::Num,
            state: BindingState::Value(&a),
        },
        Binding {
            key: BindingKey::Input(&right),
            ty: &T::Num,
            state: BindingState::Value(&b),
        },
    ];
    for select in 0..8 {
        let exact = match select {
            0 | 4 | 7 => 3,
            _ => 2,
        };
        for bound in [exact - 1, exact] {
            let mut limits = ExpressionLimits::hard();
            match select {
                0 => limits.nodes = bound,
                1 => limits.evaluation_depth = bound,
                2 => limits.bindings = bound,
                3 => limits.dependency_edges = bound,
                4 => limits.coefficient_digits = bound,
                5 => limits.absolute_scale = bound,
                6 => limits.syntax_depth = bound,
                _ => limits.output_bytes = bound,
            }
            let actual = run(&model, &bindings, limits, &CancellationToken::new());
            if bound == exact {
                assert_eq!(
                    actual,
                    R::Value {
                        ty: &T::Num,
                        value: number("3.23")
                    }
                );
            } else {
                assert_eq!(actual, R::Failure(F::Limit), "control {select}");
            }
        }
    }
}

#[test]
fn failed_and_cancelled_numeric_generations_do_not_poison_recovery() {
    let model = numeric_model(BinaryOperator::Divide, "1", "8", true);
    let left = symbol("left");
    let right = symbol("right");
    let a = number("1");
    let b = number("8");
    let bindings = [
        Binding {
            key: BindingKey::Input(&left),
            ty: &T::Num,
            state: BindingState::Value(&a),
        },
        Binding {
            key: BindingKey::Input(&right),
            ty: &T::Num,
            state: BindingState::Value(&b),
        },
    ];
    let (accepted, count) = observe(None, || {
        run(
            &model,
            &bindings,
            ExpressionLimits::hard(),
            &CancellationToken::new(),
        )
    });
    assert_eq!(
        accepted,
        R::Value {
            ty: &T::Num,
            value: number("0.125")
        }
    );
    for index in 0..count {
        let (failed, _) = observe(Some(index), || {
            run(
                &model,
                &bindings,
                ExpressionLimits::hard(),
                &CancellationToken::new(),
            )
        });
        assert_eq!(failed, R::Failure(F::Limit));
        let signal = CancellationToken::new();
        let (failed, _) = observe_cancellation(index, &signal, || {
            run(&model, &bindings, ExpressionLimits::hard(), &signal)
        });
        assert_eq!(failed, R::Failure(F::Cancelled));
        assert_eq!(
            run(
                &model,
                &bindings,
                ExpressionLimits::hard(),
                &CancellationToken::new()
            ),
            R::Value {
                ty: &T::Num,
                value: number("0.125")
            }
        );
    }
    assert_eq!(
        accepted,
        R::Value {
            ty: &T::Num,
            value: number("0.125")
        }
    );
}

#[test]
fn numeric_binding_generations_keep_missing_failed_unavailable_and_values_distinct() {
    let model = numeric_model(BinaryOperator::Divide, "1", "8", true);
    let left = symbol("left");
    let right = symbol("right");
    let a = number("1");
    let b = number("8");
    let mut bindings = [
        Binding {
            key: BindingKey::Input(&left),
            ty: &T::Num,
            state: BindingState::Value(&a),
        },
        Binding {
            key: BindingKey::Input(&right),
            ty: &T::Num,
            state: BindingState::Value(&b),
        },
    ];
    let accepted = run(
        &model,
        &bindings,
        ExpressionLimits::hard(),
        &CancellationToken::new(),
    );
    for state in [
        BindingState::Failed,
        BindingState::Unavailable,
        BindingState::Value(&b),
    ] {
        bindings[1].state = state;
        let expected = match state {
            BindingState::Failed => R::Failure(F::BindingFailed),
            BindingState::Unavailable => R::Unavailable(BindingKey::Input(&right)),
            _ => R::Value {
                ty: &T::Num,
                value: number("0.125"),
            },
        };
        assert_eq!(
            run(
                &model,
                &bindings,
                ExpressionLimits::hard(),
                &CancellationToken::new()
            ),
            expected
        );
    }
    assert_eq!(
        run(
            &model,
            &[],
            ExpressionLimits::hard(),
            &CancellationToken::new()
        ),
        R::RequiredBindings(vec![BindingKey::Input(&left), BindingKey::Input(&right)])
    );
    let replacement = number("2");
    bindings[1].state = BindingState::Value(&replacement);
    assert_eq!(
        run(
            &model,
            &bindings,
            ExpressionLimits::hard(),
            &CancellationToken::new()
        ),
        R::Value {
            ty: &T::Num,
            value: number("0.5")
        }
    );
    assert_eq!(
        accepted,
        R::Value {
            ty: &T::Num,
            value: number("0.125")
        }
    );
}
