// SPDX-License-Identifier: Apache-2.0

//! Literal semantic oracles; source/encoding/companion validity is a separate boundary.

use neutral_core::{CancellationToken, SemanticDigest, StructuralLimits, allocation::Shared};
use neutral_ir::{
    LogicalModuleIdentity, ModuleSymbolIdentity,
    composition::{
        CompositionBody, CompositionField, FieldPresence, FieldRestrictions,
        project::{CompositionResourceFacts, CompositionSignature as S},
    },
    expression::{
        profile::{self, ExpressionLimits, SemanticProfiles},
        project::{ExpressionDeclaration as D, ExpressionProjectIr, ExpressionResourceFacts},
        templates::ExpansionLimits,
        *,
    },
    project::{ProjectLimits, ProjectModule, ProjectResourceFacts},
    project_interface::ProjectPublicType as T,
};
use neutral_semantics::{
    ExpressionValidationError as E, SemanticLimits, validate_expression_model,
};
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};
mod facts;
mod hardening;

fn symbol(name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(LogicalModuleIdentity::new("1.0", "example"), name)
}
fn leaf(value: bool) -> Expression {
    Expression {
        ty: T::Bool,
        node: ExpressionNode::Value(neutral_ir::composition::BindingValue::Bool(value)),
    }
}
fn controls() -> SemanticLimits {
    SemanticLimits {
        project: ProjectLimits {
            modules: 64,
            declarations: 1024,
            import_edges: 1024,
            nodes: 65_536,
            text_bytes: 65_536,
            artifact_bytes: 1_048_576,
        },
        composition: CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(65_536, 64).unwrap(),
        )),
        expression: ExpressionLimits::hard(),
        expansion: ExpansionLimits::hard(),
    }
}
fn project(declarations: Vec<D>) -> ExpressionProjectIr {
    let limits = controls();
    ExpressionProjectIr {
        schema: profile::PROJECT_IR_SCHEMA.into(),
        modules: vec![ProjectModule {
            identity: LogicalModuleIdentity::new("1.0", "example"),
            imports: vec![],
        }],
        declarations,
        vocabularies: vec![],
        interface_digest: SemanticDigest::from_raw_bytes([0; 32]),
        sources: vec![],
        source_maps: vec![],
        provenance: vec![],
        limits: limits.project,
        resources: ProjectResourceFacts {
            source_units: 0,
            source_bytes: 0,
            vocabulary_units: 0,
            vocabulary_bytes: 0,
            declarations: 0,
            import_edges: 0,
            value_nodes: 0,
        },
        vocabulary_sources: vec![],
        origins: vec![],
        composition_limits: limits.composition.policy(),
        composition_resources: CompositionResourceFacts::from_values([0; 7]),
        semantic_profiles: SemanticProfiles::try_frozen().unwrap(),
        public_templates: vec![],
        expression_limits: limits.expression,
        expansion_limits: limits.expansion,
        expression_resources: ExpressionResourceFacts::default(),
        expression_attributions: vec![],
        expansion_attributions: vec![],
    }
}
fn binding(name: &str, ty: T, body: ExpressionBody) -> D {
    D {
        identity: symbol(name),
        public: true,
        signature: S::Binding(ty),
        body: Some(body),
    }
}
fn validate(ir: ExpressionProjectIr) -> Result<neutral_semantics::ValidatedExpressionModel, E> {
    validate_expression_model(
        Shared::try_new(ir).unwrap(),
        controls(),
        &CancellationToken::new(),
    )
}

#[test]
fn unselected_branch_forged_type_is_rejected() {
    let forged = Expression {
        ty: T::Bool,
        node: ExpressionNode::Value(neutral_ir::composition::BindingValue::String(
            "not bool".into(),
        )),
    };
    let expression = Expression {
        ty: T::Bool,
        node: ExpressionNode::If {
            guard: Box::new(leaf(true)),
            then_branch: Box::new(leaf(false)),
            else_branch: Box::new(forged),
        },
    };
    assert_eq!(
        validate(project(vec![binding(
            "result",
            T::Bool,
            ExpressionBody::Expression(expression)
        )]))
        .unwrap_err(),
        E::TypeMismatch
    );
}

#[test]
fn bound_scope_depth_slot_and_type_are_checked() {
    for depth in [0, u64::MAX] {
        let expression = Expression {
            ty: T::Bool,
            node: ExpressionNode::Bound { depth, slot: 0 },
        };
        assert_eq!(
            validate(project(vec![binding(
                "result",
                T::Bool,
                ExpressionBody::Expression(expression)
            )]))
            .unwrap_err(),
            E::TypeMismatch
        );
    }
}

#[test]
fn match_payload_frame_is_typed_and_cannot_escape_its_slot() {
    let make = |slot| {
        let mut ir = project(vec![
            D {
                identity: symbol("State"),
                public: true,
                signature: S::Definition(CompositionBody::Variant(vec![
                    neutral_ir::composition::CompositionAlternative {
                        tag: "ready".into(),
                        ty: T::Bool,
                    },
                ])),
                body: None,
            },
            binding(
                "result",
                T::Bool,
                ExpressionBody::Expression(Expression {
                    ty: T::Bool,
                    node: ExpressionNode::Match {
                        scrutinee: Box::new(Expression {
                            ty: T::Nominal(symbol("State")),
                            node: ExpressionNode::Value(
                                neutral_ir::composition::BindingValue::Variant {
                                    tag: "ready".into(),
                                    payload: Box::new(neutral_ir::composition::BindingValue::Bool(
                                        true,
                                    )),
                                },
                            ),
                        }),
                        arms: vec![MatchArm {
                            tag: "ready".into(),
                            payload_type: T::Bool,
                            body: Expression {
                                ty: T::Bool,
                                node: ExpressionNode::Bound { depth: 0, slot },
                            },
                        }],
                    },
                }),
            ),
        ]);
        ir.expression_resources = ExpressionResourceFacts {
            residual_nodes: 3,
            residual_depth: 2,
            retained_expression_text_bytes: 70,
            ..ExpressionResourceFacts::default()
        };
        ir
    };
    assert!(validate(make(0)).is_ok());
    assert_eq!(validate(make(1)).unwrap_err(), E::TypeMismatch);
}

#[test]
fn public_expression_cannot_leak_private_owners() {
    let definition = D {
        identity: symbol("Private"),
        public: false,
        signature: S::Definition(CompositionBody::Record(vec![])),
        body: None,
    };
    let input = binding(
        "input",
        T::Nominal(symbol("Private")),
        ExpressionBody::Input,
    );
    assert_eq!(
        validate(project(vec![definition, input])).unwrap_err(),
        E::UnknownOrPrivate
    );
}

#[test]
fn match_requires_every_exact_alternative() {
    let definition = D {
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
    };
    let input = binding("input", T::Nominal(symbol("State")), ExpressionBody::Input);
    let expression = Expression {
        ty: T::Bool,
        node: ExpressionNode::Match {
            scrutinee: Box::new(Expression {
                ty: T::Nominal(symbol("State")),
                node: ExpressionNode::Input(symbol("input")),
            }),
            arms: vec![MatchArm {
                tag: "ready".into(),
                payload_type: T::Bool,
                body: leaf(true),
            }],
        },
    };
    assert_eq!(
        validate(project(vec![
            definition,
            input,
            binding("result", T::Bool, ExpressionBody::Expression(expression))
        ]))
        .unwrap_err(),
        E::MatchAlternatives
    );
}

#[test]
fn output_selector_retains_future_obligations() {
    let definition = D {
        identity: symbol("Result"),
        public: true,
        signature: S::Definition(CompositionBody::Record(vec![CompositionField {
            name: "items".into(),
            ty: T::List(Box::new(T::String)),
            presence: FieldPresence::Required,
            restrictions: FieldRestrictions::default(),
            default: None,
        }])),
        body: None,
    };
    let op = binding(
        "operation",
        T::Nominal(symbol("Result")),
        ExpressionBody::Input,
    );
    let expression = Expression {
        ty: T::String,
        node: ExpressionNode::Output {
            target: symbol("operation"),
            result_type: T::Nominal(symbol("Result")),
            selector: vec![
                SelectorSegment::Field("items".into()),
                SelectorSegment::Index(3),
            ],
        },
    };
    let mut ir = project(vec![
        definition,
        op,
        binding("result", T::String, ExpressionBody::Expression(expression)),
    ]);
    // output(6) + string(6) + target(3+7+9) + nominal(7)+result symbol(3+7+6)+field(5)+items(5)+index(5).
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: 1,
        residual_depth: 1,
        input_declarations: 1,
        output_leaves: 1,
        potential_value_edges: 1,
        public_templates: 0,
        retained_expression_text_bytes: 69,
    };
    let model = validate(ir).unwrap();
    let obligations = model.availability_obligations();
    assert_eq!(obligations.len(), 1);
    assert_eq!(obligations[0].segment, 1);
    assert_eq!(obligations[0].site.owner, symbol("result"));
}
