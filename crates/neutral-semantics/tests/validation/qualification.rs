// SPDX-License-Identifier: Apache-2.0

//! Qualification of exact types, applicable model controls and immutable request recovery.

use super::*;
use neutral_core::allocation::testing::{observe, observe_cancellation};
use neutral_ir::{ExactNumber, composition::BindingValue as V};
use neutral_semantics::derive_expression_resources;

/// Produces coherent raw facts, without treating them as trusted validation authority.
pub(super) fn ready(mut ir: ExpressionProjectIr) -> Shared<ExpressionProjectIr> {
    ir.expression_resources =
        derive_expression_resources(&ir, controls(), &CancellationToken::new()).unwrap();
    Shared::try_new(ir).unwrap()
}

#[test]
fn vocabulary_types_require_the_exact_public_revision() {
    use neutral_core::VocabularyContentDigest;
    use neutral_ir::{
        VocabularyIdentity,
        composition::{CompositionBundle, CompositionDefinition},
    };
    use neutral_vocabulary::composition::{ENCODING_VERSION, REQUIRED_FEATURE, SCHEMA_VERSION};
    for (revision, public, accepted) in [
        ("1.0.0", true, true),
        ("1.0.1", true, false),
        ("1.0.0", false, false),
    ] {
        let mut ir = project(vec![binding(
            "external",
            T::VocabularyNominal {
                identity: "Domain".into(),
                version: revision.into(),
                name: "Item".into(),
            },
            ExpressionBody::Input,
        )]);
        ir.vocabularies.push(CompositionBundle {
            identity: VocabularyIdentity::new(
                "Domain",
                "1.0.0",
                SCHEMA_VERSION,
                ENCODING_VERSION,
                VocabularyContentDigest::from_bytes(b"literal catalogue"),
                vec![REQUIRED_FEATURE.into()],
            ),
            dependencies: vec![],
            definitions: vec![CompositionDefinition {
                name: "Item".into(),
                public,
                body: CompositionBody::Record(vec![]),
            }],
        });
        ir.expression_resources.input_declarations = 1;
        let result = validate(ir);
        if accepted {
            assert!(result.is_ok());
        } else {
            assert_eq!(result.unwrap_err(), E::UnknownOrPrivate);
        }
    }
}

#[test]
fn input_signatures_reject_non_nominal_refs_and_nested_nullable_wrappers() {
    for ty in [
        T::Ref(Box::new(T::Num)),
        T::Ref(Box::new(T::List(Box::new(T::Bool)))),
        T::Nullable(Box::new(T::Nullable(Box::new(T::Bool)))),
    ] {
        let mut ir = project(vec![binding("external", ty, ExpressionBody::Input)]);
        ir.expression_resources.input_declarations = 1;
        assert_eq!(validate(ir).unwrap_err(), E::TypeMismatch);
    }
    let definition = D {
        identity: symbol("Node"),
        public: true,
        signature: S::Definition(CompositionBody::Record(vec![])),
        body: None,
    };
    let mut ir = project(vec![
        definition,
        binding(
            "external",
            T::List(Box::new(T::Nullable(Box::new(T::Ref(Box::new(
                T::Nominal(symbol("Node")),
            )))))),
            ExpressionBody::Input,
        ),
    ]);
    ir.expression_resources.input_declarations = 1;
    assert!(validate(ir).is_ok());
}

#[test]
fn equality_and_concatenation_require_exact_types_not_similar_payloads() {
    for operator in [BinaryOperator::Equal, BinaryOperator::Concat] {
        let left_ty = T::List(Box::new(T::Bool));
        let right_ty = T::List(Box::new(T::Num));
        let ir = project(vec![
            binding("left", left_ty.clone(), ExpressionBody::Input),
            binding(
                "result",
                if operator == BinaryOperator::Equal {
                    T::Bool
                } else {
                    left_ty.clone()
                },
                ExpressionBody::Expression(Expression {
                    ty: if operator == BinaryOperator::Equal {
                        T::Bool
                    } else {
                        left_ty.clone()
                    },
                    node: ExpressionNode::Binary {
                        operator,
                        left: Box::new(Expression {
                            ty: left_ty,
                            node: ExpressionNode::Input(symbol("left")),
                        }),
                        right: Box::new(Expression {
                            ty: right_ty.clone(),
                            node: ExpressionNode::Input(symbol("right")),
                        }),
                    },
                }),
            ),
            binding("right", right_ty, ExpressionBody::Input),
        ]);
        assert_eq!(validate(ir).unwrap_err(), E::TypeMismatch);
    }
    let record = |name| D {
        identity: symbol(name),
        public: true,
        signature: S::Definition(CompositionBody::Record(vec![])),
        body: None,
    };
    let a = T::Nominal(symbol("A"));
    let b = T::Nominal(symbol("B"));
    let ir = project(vec![
        record("A"),
        record("B"),
        binding(
            "result",
            T::Bool,
            ExpressionBody::Expression(Expression {
                ty: T::Bool,
                node: ExpressionNode::Binary {
                    operator: BinaryOperator::Equal,
                    left: Box::new(Expression {
                        ty: a,
                        node: ExpressionNode::Value(V::Record(vec![])),
                    }),
                    right: Box::new(Expression {
                        ty: b,
                        node: ExpressionNode::Value(V::Record(vec![])),
                    }),
                },
            }),
        ),
    ]);
    assert_eq!(validate(ir).unwrap_err(), E::TypeMismatch);
}

#[test]
fn dangling_input_and_reference_targets_never_become_valid_values() {
    let input = Expression {
        ty: T::Bool,
        node: ExpressionNode::Input(symbol("absent")),
    };
    assert_eq!(
        validate(project(vec![binding(
            "result",
            T::Bool,
            ExpressionBody::Expression(input)
        )]))
        .unwrap_err(),
        E::UnknownOrPrivate
    );
    let node = D {
        identity: symbol("Node"),
        public: true,
        signature: S::Definition(CompositionBody::Record(vec![])),
        body: None,
    };
    let reference = binding(
        "result",
        T::Ref(Box::new(T::Nominal(symbol("Node")))),
        ExpressionBody::Value(V::Reference(symbol("absent"))),
    );
    assert_eq!(
        validate(project(vec![node, reference])).unwrap_err(),
        E::UnknownOrPrivate
    );
    let mut ir = project(vec![
        binding(
            "a",
            T::Bool,
            ExpressionBody::Expression(Expression {
                ty: T::Bool,
                node: ExpressionNode::Input(symbol("b")),
            }),
        ),
        binding(
            "b",
            T::Bool,
            ExpressionBody::Expression(Expression {
                ty: T::Bool,
                node: ExpressionNode::Input(symbol("a")),
            }),
        ),
    ]);
    // Ordinary expression bodies cannot masquerade as external inputs to form a value cycle.
    assert_eq!(validate(ir).unwrap_err(), E::TypeMismatch);
    ir = project(vec![
        binding(
            "result",
            T::Bool,
            ExpressionBody::Expression(Expression {
                ty: T::Bool,
                node: ExpressionNode::Input(symbol("value")),
            }),
        ),
        binding("value", T::Bool, ExpressionBody::Value(V::Bool(true))),
    ]);
    assert_eq!(validate(ir).unwrap_err(), E::TypeMismatch);
}

#[test]
fn output_selectors_reject_unknown_members_wrong_kinds_and_private_targets() {
    for (selector, ty, public, expected) in [
        (
            vec![SelectorSegment::Field("unknown".into())],
            T::Bool,
            true,
            E::Restriction,
        ),
        (
            vec![SelectorSegment::Index(0)],
            T::Bool,
            true,
            E::Restriction,
        ),
        (
            vec![SelectorSegment::Alternative("ready".into())],
            T::Bool,
            true,
            E::Restriction,
        ),
        (
            vec![
                SelectorSegment::Field("items".into()),
                SelectorSegment::Field("unknown".into()),
            ],
            T::Bool,
            true,
            E::Restriction,
        ),
        (
            vec![
                SelectorSegment::Field("items".into()),
                SelectorSegment::Index(0),
            ],
            T::Num,
            true,
            E::TypeMismatch,
        ),
        (
            vec![
                SelectorSegment::Field("items".into()),
                SelectorSegment::Index(0),
            ],
            T::Bool,
            false,
            E::UnknownOrPrivate,
        ),
    ] {
        let definition = D {
            identity: symbol("Result"),
            public: true,
            signature: S::Definition(CompositionBody::Record(vec![CompositionField {
                name: "items".into(),
                ty: T::List(Box::new(T::Bool)),
                presence: FieldPresence::Required,
                restrictions: FieldRestrictions::default(),
                default: None,
            }])),
            body: None,
        };
        let mut target = binding(
            "operation",
            T::Nominal(symbol("Result")),
            ExpressionBody::Input,
        );
        target.public = public;
        let expression = Expression {
            ty: ty.clone(),
            node: ExpressionNode::Output {
                target: symbol("operation"),
                result_type: T::Nominal(symbol("Result")),
                selector,
            },
        };
        let ir = project(vec![
            definition,
            target,
            binding("result", ty, ExpressionBody::Expression(expression)),
        ]);
        assert_eq!(validate(ir).unwrap_err(), expected);
    }
}

#[test]
fn model_number_module_and_type_depth_boundaries_are_independent() {
    for select in 0..3 {
        let mut ir = match select {
            0 => project(vec![binding(
                "value",
                T::Num,
                ExpressionBody::Value(V::Number(ExactNumber::from_source("1.23", 3, 2).unwrap())),
            )]),
            1 => project(vec![binding(
                "value",
                T::List(Box::new(T::List(Box::new(T::Bool)))),
                ExpressionBody::Input,
            )]),
            _ => {
                let mut ir = project(vec![]);
                ir.modules.push(ProjectModule {
                    identity: LogicalModuleIdentity::new("1.0", "second"),
                    imports: vec![],
                });
                ir
            }
        };
        ir.expression_resources =
            derive_expression_resources(&ir, controls(), &CancellationToken::new()).unwrap();
        let ir = Shared::try_new(ir).unwrap();
        let exact = if select == 0 { 3 } else { 2 };
        for boundary in [exact - 1, exact] {
            let mut limits = controls();
            match select {
                0 => limits.expression.coefficient_digits = boundary,
                1 => limits.composition.type_depth = boundary,
                _ => limits.project.modules = boundary,
            }
            let result = validate_expression_model(ir.clone(), limits, &CancellationToken::new());
            if boundary == exact {
                assert!(result.is_ok(), "dimension {select}");
            } else {
                assert_eq!(result.unwrap_err(), E::Limit);
            }
        }
        if select == 0 {
            let mut limits = controls();
            limits.expression.absolute_scale = 1;
            assert_eq!(
                validate_expression_model(ir.clone(), limits, &CancellationToken::new())
                    .unwrap_err(),
                E::Limit
            );
            limits.expression.absolute_scale = 2;
            assert!(validate_expression_model(ir, limits, &CancellationToken::new()).is_ok());
        }
    }
}

#[test]
fn model_work_and_retention_boundaries_preserve_accepted_authority() {
    let ir = ready(project(vec![binding(
        "result",
        T::Bool,
        ExpressionBody::Expression(leaf(true)),
    )]));
    let accepted =
        validate_expression_model(ir.clone(), controls(), &CancellationToken::new()).unwrap();
    for retention in [false, true] {
        let mut low = 1;
        let mut high = if retention {
            controls().expression.retained_bytes
        } else {
            controls().expression.work
        };
        let check = |limit| {
            let mut caller = controls();
            if retention {
                caller.expression.retained_bytes = limit;
            } else {
                caller.expression.work = limit;
            }
            validate_expression_model(ir.clone(), caller, &CancellationToken::new())
        };
        // Algorithmic accounting may evolve; the control must still have an inclusive nonzero boundary.
        while low < high {
            let mid = low + (high - low) / 2;
            match check(mid) {
                Ok(_) => high = mid,
                Err(E::Limit) => low = mid + 1,
                Err(error) => panic!("{error:?}"),
            }
        }
        assert!(low > 1);
        assert!(check(low).is_ok());
        assert_eq!(check(low - 1).unwrap_err(), E::Limit);
        assert_eq!(accepted.facts().values(), [1, 1, 0, 0, 0, 0, 13]);
        assert!(check(high).is_ok());
    }
    let (result, count) = observe(None, || {
        validate_expression_model(ir.clone(), controls(), &CancellationToken::new())
    });
    assert!(result.is_ok());
    for index in 0..count {
        let (result, _) = observe(Some(index), || {
            validate_expression_model(ir.clone(), controls(), &CancellationToken::new())
        });
        assert_eq!(result.unwrap_err(), E::Limit);
        let signal = CancellationToken::new();
        let (result, _) = observe_cancellation(index, &signal, || {
            validate_expression_model(ir.clone(), controls(), &signal)
        });
        assert_eq!(result.unwrap_err(), E::Cancelled);
        assert!(
            validate_expression_model(ir.clone(), controls(), &CancellationToken::new()).is_ok()
        );
    }
    assert_eq!(accepted.facts().values(), [1, 1, 0, 0, 0, 0, 13]);
}
