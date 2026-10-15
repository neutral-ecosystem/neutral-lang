// SPDX-License-Identifier: Apache-2.0

//! Output binding needs actual typed root evidence; selected values alone cannot prove availability.

use super::*;
use neutral_ir::composition::{BindingValue as V, CompositionBundle, CompositionDefinition};
use neutral_semantics::{
    Binding, BindingKey, BindingState, EvaluationError as F, EvaluationRequest,
    EvaluationResult as R, OutputAssociation, evaluate,
};

fn program() -> neutral_semantics::ValidatedExpressionModel {
    let field = CompositionField {
        name: "items".into(),
        ty: T::List(Box::new(T::Bool)),
        presence: FieldPresence::Required,
        restrictions: FieldRestrictions::default(),
        default: None,
    };
    let mut ir = project(vec![
        D {
            identity: symbol("Operation"),
            public: true,
            signature: S::Definition(CompositionBody::Record(vec![])),
            body: None,
        },
        D {
            identity: symbol("Result"),
            public: true,
            signature: S::Definition(CompositionBody::Record(vec![field])),
            body: None,
        },
        binding("ask", T::Bool, ExpressionBody::Input),
        binding(
            "operation",
            T::Nominal(symbol("Operation")),
            ExpressionBody::Input,
        ),
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
                    else_branch: Box::new(Expression {
                        ty: T::Bool,
                        node: ExpressionNode::Output {
                            target: symbol("operation"),
                            result_type: T::Nominal(symbol("Result")),
                            selector: vec![
                                SelectorSegment::Field("items".into()),
                                SelectorSegment::Index(1),
                            ],
                        },
                    }),
                },
            }),
        ),
    ]);
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: 4,
        residual_depth: 2,
        input_declarations: 2,
        output_leaves: 1,
        potential_value_edges: 2,
        retained_expression_text_bytes: 108,
        ..ExpressionResourceFacts::default()
    };
    validate(ir).unwrap()
}

fn run<'m>(
    model: &'m neutral_semantics::ValidatedExpressionModel,
    bindings: &[Binding<'_>],
    associations: &[OutputAssociation<'_>],
) -> R<'m> {
    evaluate(
        model,
        &EvaluationRequest {
            selected: &symbol("result"),
            profiles: &model.ir().semantic_profiles,
            bindings,
            associations,
            limits: ExpressionLimits::hard(),
        },
        &CancellationToken::new(),
    )
}

fn output(result_type: T, selector: Vec<SelectorSegment>, ty: T) -> Expression {
    Expression {
        ty,
        node: ExpressionNode::Output {
            target: symbol("operation"),
            result_type,
            selector,
        },
    }
}

fn required_field(name: &str, ty: T) -> CompositionField {
    CompositionField {
        name: name.into(),
        ty,
        presence: FieldPresence::Required,
        restrictions: FieldRestrictions::default(),
        default: None,
    }
}

fn definition(name: &str, fields: Vec<CompositionField>) -> D {
    D {
        identity: symbol(name),
        public: true,
        signature: S::Definition(CompositionBody::Record(fields)),
        body: None,
    }
}

fn output_model(mut ir: ExpressionProjectIr) -> neutral_semantics::ValidatedExpressionModel {
    ir.expression_resources =
        neutral_semantics::derive_expression_resources(&ir, controls(), &CancellationToken::new())
            .unwrap();
    validate(ir).unwrap()
}

fn operation_binding() -> D {
    D {
        identity: symbol("operation"),
        public: true,
        signature: S::Binding(T::Nominal(symbol("Operation"))),
        body: Some(ExpressionBody::Input),
    }
}

fn operation_definition() -> D {
    definition("Operation", vec![])
}

fn source_contract(name: &str) -> T {
    T::Nominal(symbol(name))
}

fn vocabulary_contract(name: &str) -> T {
    T::VocabularyNominal {
        identity: "Domain".into(),
        version: "1.0.0".into(),
        name: name.into(),
    }
}

fn vocabulary_result_contracts(names: &[&str]) -> CompositionBundle {
    use neutral_core::VocabularyContentDigest;
    use neutral_ir::VocabularyIdentity;
    use neutral_vocabulary::composition::{ENCODING_VERSION, REQUIRED_FEATURE, SCHEMA_VERSION};

    CompositionBundle {
        identity: VocabularyIdentity::new(
            "Domain",
            "1.0.0",
            SCHEMA_VERSION,
            ENCODING_VERSION,
            VocabularyContentDigest::from_bytes(b"evaluation output ordering"),
            vec![REQUIRED_FEATURE.into()],
        ),
        dependencies: vec![],
        definitions: names
            .iter()
            .map(|name| CompositionDefinition {
                name: (*name).into(),
                public: true,
                body: CompositionBody::Record(vec![required_field("value", T::Bool)]),
            })
            .collect(),
    }
}

#[test]
fn unselected_output_is_not_required() {
    let model = program();
    let ask = symbol("ask");
    assert_eq!(
        run(
            &model,
            &[Binding {
                key: BindingKey::Input(&ask),
                ty: &T::Bool,
                state: BindingState::Value(&V::Bool(true))
            }],
            &[]
        ),
        R::Value {
            ty: &T::Bool,
            value: V::Bool(false)
        }
    );
    assert_eq!(
        run(&model, &[], &[]),
        R::RequiredBindings(vec![BindingKey::Input(&ask)])
    );
}

#[test]
fn supplied_output_root_must_discharge_actual_index_obligations() {
    let model = program();
    let ask = symbol("ask");
    let target = symbol("operation");
    let result_type = T::Nominal(symbol("Result"));
    let selector = [
        SelectorSegment::Field("items".into()),
        SelectorSegment::Index(1),
    ];
    let key = BindingKey::Output {
        target: &target,
        result_type: &result_type,
        selector: &selector,
    };
    let association = [OutputAssociation {
        target: &target,
        result_type: &result_type,
    }];
    let valid = V::Record(vec![(
        "items".into(),
        Some(V::List(vec![V::Bool(false), V::Bool(true)])),
    )]);
    let invalid = V::Record(vec![("items".into(), Some(V::List(vec![V::Bool(true)])))]);
    let ask_false = Binding {
        key: BindingKey::Input(&ask),
        ty: &T::Bool,
        state: BindingState::Value(&V::Bool(false)),
    };
    assert_eq!(
        run(
            &model,
            &[
                ask_false,
                Binding {
                    key,
                    ty: &T::Bool,
                    state: BindingState::OutputResult(&valid)
                }
            ],
            &association
        ),
        R::Value {
            ty: &T::Bool,
            value: V::Bool(true)
        }
    );
    assert_eq!(
        run(
            &model,
            &[Binding {
                key,
                ty: &T::Bool,
                state: BindingState::OutputResult(&invalid)
            }],
            &association
        ),
        R::Failure(F::Restriction)
    );
    // Detached selected data cannot prove a parent field restriction or index availability.
    assert_eq!(
        run(
            &model,
            &[Binding {
                key,
                ty: &T::Bool,
                state: BindingState::Value(&V::Bool(true))
            }],
            &association
        ),
        R::Failure(F::BindingType)
    );
}

#[test]
fn missing_input_and_output_ports_have_canonical_kind_and_symbol_order() {
    let a_input = symbol("a_input");
    let z_input = symbol("z_input");
    let result_type = source_contract("Result");
    let ir = project(vec![
        operation_definition(),
        definition(
            "Result",
            vec![required_field("a", T::Bool), required_field("z", T::Bool)],
        ),
        binding("a_input", T::Bool, ExpressionBody::Input),
        operation_binding(),
        binding(
            "result",
            T::List(Box::new(T::Bool)),
            ExpressionBody::Expression(Expression {
                ty: T::List(Box::new(T::Bool)),
                node: ExpressionNode::List(vec![
                    Expression {
                        ty: T::Bool,
                        node: ExpressionNode::Input(z_input.clone()),
                    },
                    output(
                        result_type.clone(),
                        vec![SelectorSegment::Field("z".into())],
                        T::Bool,
                    ),
                    Expression {
                        ty: T::Bool,
                        node: ExpressionNode::Input(a_input.clone()),
                    },
                    output(
                        result_type.clone(),
                        vec![SelectorSegment::Field("a".into())],
                        T::Bool,
                    ),
                    output(
                        result_type.clone(),
                        vec![SelectorSegment::Field("a".into())],
                        T::Bool,
                    ),
                ]),
            }),
        ),
        binding("z_input", T::Bool, ExpressionBody::Input),
    ]);
    let model = output_model(ir);
    let target = symbol("operation");
    let field_a = [SelectorSegment::Field("a".into())];
    let field_z = [SelectorSegment::Field("z".into())];
    let associations = [OutputAssociation {
        target: &target,
        result_type: &result_type,
    }];

    assert_eq!(
        run(&model, &[], &associations),
        R::RequiredBindings(vec![
            BindingKey::Input(&a_input),
            BindingKey::Input(&z_input),
            BindingKey::Output {
                target: &target,
                result_type: &result_type,
                selector: &field_a,
            },
            BindingKey::Output {
                target: &target,
                result_type: &result_type,
                selector: &field_z,
            },
        ])
    );
}

#[test]
fn missing_outputs_from_one_target_order_by_exact_public_result_contract() {
    let alpha = source_contract("Alpha");
    let zulu = source_contract("Zulu");
    let vocabulary_alpha = vocabulary_contract("Alpha");
    let vocabulary_zulu = vocabulary_contract("Zulu");
    let value_path = [SelectorSegment::Field("value".into())];
    let mut ir = project(vec![
        definition("Alpha", vec![required_field("value", T::Bool)]),
        operation_definition(),
        definition("Zulu", vec![required_field("value", T::Bool)]),
        operation_binding(),
        binding(
            "result",
            T::List(Box::new(T::Bool)),
            ExpressionBody::Expression(Expression {
                ty: T::List(Box::new(T::Bool)),
                node: ExpressionNode::List(vec![
                    output(
                        vocabulary_zulu.clone(),
                        vec![SelectorSegment::Field("value".into())],
                        T::Bool,
                    ),
                    output(
                        zulu.clone(),
                        vec![SelectorSegment::Field("value".into())],
                        T::Bool,
                    ),
                    output(
                        vocabulary_alpha.clone(),
                        vec![SelectorSegment::Field("value".into())],
                        T::Bool,
                    ),
                    output(
                        alpha.clone(),
                        vec![SelectorSegment::Field("value".into())],
                        T::Bool,
                    ),
                ]),
            }),
        ),
    ]);
    ir.vocabularies
        .push(vocabulary_result_contracts(&["Alpha", "Zulu"]));
    let model = output_model(ir);
    let target = symbol("operation");
    let associations = [
        OutputAssociation {
            target: &target,
            result_type: &alpha,
        },
        OutputAssociation {
            target: &target,
            result_type: &zulu,
        },
        OutputAssociation {
            target: &target,
            result_type: &vocabulary_alpha,
        },
        OutputAssociation {
            target: &target,
            result_type: &vocabulary_zulu,
        },
    ];

    assert_eq!(
        run(&model, &[], &associations),
        R::RequiredBindings(vec![
            BindingKey::Output {
                target: &target,
                result_type: &alpha,
                selector: &value_path,
            },
            BindingKey::Output {
                target: &target,
                result_type: &zulu,
                selector: &value_path,
            },
            BindingKey::Output {
                target: &target,
                result_type: &vocabulary_alpha,
                selector: &value_path,
            },
            BindingKey::Output {
                target: &target,
                result_type: &vocabulary_zulu,
                selector: &value_path,
            },
        ])
    );
}

#[test]
fn missing_outputs_with_one_contract_order_by_selector_payload_and_prefix() {
    let result_type = source_contract("Result");
    let state_type = source_contract("State");
    let choice_path = [SelectorSegment::Field("choice".into())];
    let choice_ready = [
        SelectorSegment::Field("choice".into()),
        SelectorSegment::Alternative("ready".into()),
    ];
    let choice_waiting = [
        SelectorSegment::Field("choice".into()),
        SelectorSegment::Alternative("waiting".into()),
    ];
    let items_path = [SelectorSegment::Field("items".into())];
    let item_two = [
        SelectorSegment::Field("items".into()),
        SelectorSegment::Index(2),
    ];
    let item_ten = [
        SelectorSegment::Field("items".into()),
        SelectorSegment::Index(10),
    ];
    let name_path = [SelectorSegment::Field("name".into())];
    let ir = project(vec![
        definition(
            "Answer",
            vec![
                required_field("choice", state_type.clone()),
                required_field("choice_ready", T::Bool),
                required_field("choice_waiting", T::Bool),
                required_field("item_ten", T::Bool),
                required_field("item_two", T::Bool),
                required_field("items", T::List(Box::new(T::Bool))),
                required_field("name", T::Bool),
                required_field("name_copy", T::Bool),
            ],
        ),
        operation_definition(),
        definition(
            "Result",
            vec![
                required_field("choice", state_type.clone()),
                required_field("items", T::List(Box::new(T::Bool))),
                required_field("name", T::Bool),
            ],
        ),
        D {
            identity: symbol("State"),
            public: true,
            signature: S::Definition(CompositionBody::Variant(vec![
                neutral_ir::composition::CompositionAlternative {
                    tag: "ready".into(),
                    ty: T::Bool,
                },
                neutral_ir::composition::CompositionAlternative {
                    tag: "waiting".into(),
                    ty: T::Bool,
                },
            ])),
            body: None,
        },
        operation_binding(),
        binding(
            "result",
            source_contract("Answer"),
            ExpressionBody::Expression(Expression {
                ty: source_contract("Answer"),
                node: ExpressionNode::Record(vec![
                    (
                        "choice".into(),
                        Some(output(
                            result_type.clone(),
                            vec![SelectorSegment::Field("choice".into())],
                            state_type.clone(),
                        )),
                    ),
                    (
                        "choice_ready".into(),
                        Some(output(
                            result_type.clone(),
                            vec![
                                SelectorSegment::Field("choice".into()),
                                SelectorSegment::Alternative("ready".into()),
                            ],
                            T::Bool,
                        )),
                    ),
                    (
                        "choice_waiting".into(),
                        Some(output(
                            result_type.clone(),
                            vec![
                                SelectorSegment::Field("choice".into()),
                                SelectorSegment::Alternative("waiting".into()),
                            ],
                            T::Bool,
                        )),
                    ),
                    (
                        "item_ten".into(),
                        Some(output(
                            result_type.clone(),
                            vec![
                                SelectorSegment::Field("items".into()),
                                SelectorSegment::Index(10),
                            ],
                            T::Bool,
                        )),
                    ),
                    (
                        "item_two".into(),
                        Some(output(
                            result_type.clone(),
                            vec![
                                SelectorSegment::Field("items".into()),
                                SelectorSegment::Index(2),
                            ],
                            T::Bool,
                        )),
                    ),
                    (
                        "items".into(),
                        Some(output(
                            result_type.clone(),
                            vec![SelectorSegment::Field("items".into())],
                            T::List(Box::new(T::Bool)),
                        )),
                    ),
                    (
                        "name".into(),
                        Some(output(
                            result_type.clone(),
                            vec![SelectorSegment::Field("name".into())],
                            T::Bool,
                        )),
                    ),
                    (
                        "name_copy".into(),
                        Some(output(
                            result_type.clone(),
                            vec![SelectorSegment::Field("name".into())],
                            T::Bool,
                        )),
                    ),
                ]),
            }),
        ),
    ]);
    let model = output_model(ir);
    let target = symbol("operation");
    let associations = [OutputAssociation {
        target: &target,
        result_type: &result_type,
    }];

    assert_eq!(
        run(&model, &[], &associations),
        R::RequiredBindings(vec![
            BindingKey::Output {
                target: &target,
                result_type: &result_type,
                selector: &choice_path,
            },
            BindingKey::Output {
                target: &target,
                result_type: &result_type,
                selector: &choice_ready,
            },
            BindingKey::Output {
                target: &target,
                result_type: &result_type,
                selector: &choice_waiting,
            },
            BindingKey::Output {
                target: &target,
                result_type: &result_type,
                selector: &items_path,
            },
            BindingKey::Output {
                target: &target,
                result_type: &result_type,
                selector: &item_two,
            },
            BindingKey::Output {
                target: &target,
                result_type: &result_type,
                selector: &item_ten,
            },
            BindingKey::Output {
                target: &target,
                result_type: &result_type,
                selector: &name_path,
            },
        ])
    );
}
