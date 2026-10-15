// SPDX-License-Identifier: Apache-2.0

//! Output paths preserve future obligations and reject unavailable or ill-typed contracts.

use super::*;
use neutral_core::allocation::testing::{observe, observe_cancellation};
use neutral_ir::composition::{
    BindingValue as V, ClosedValue, CompositionAlternative, CompositionBundle,
    CompositionDefinition,
};
use neutral_semantics::{AvailabilityRequirement as A, ExpressionRoot};

/// Builds a literal optional-list-variant output with independently counted tree facts.
fn output_project(presence: FieldPresence, index: u32, nested: bool) -> ExpressionProjectIr {
    // Optional fields belong to vocabulary contracts, never source-owned records.
    let bucket = if presence == FieldPresence::Optional {
        T::VocabularyNominal {
            identity: "Domain".into(),
            version: "1.0.0".into(),
            name: "Bucket".into(),
        }
    } else {
        T::Nominal(symbol("Bucket"))
    };
    let output = Expression {
        ty: T::Bool,
        node: ExpressionNode::Output {
            target: symbol("operation"),
            result_type: bucket,
            selector: vec![
                SelectorSegment::Field("items".into()),
                SelectorSegment::Index(index),
                SelectorSegment::Alternative("ready".into()),
            ],
        },
    };
    let expression = if nested {
        Expression {
            ty: T::Bool,
            node: ExpressionNode::If {
                guard: Box::new(leaf(true)),
                then_branch: Box::new(output),
                else_branch: Box::new(leaf(false)),
            },
        }
    } else {
        output
    };
    let mut ir = project(vec![
        D {
            identity: symbol("Bucket"),
            public: true,
            signature: S::Definition(CompositionBody::Record(vec![CompositionField {
                name: "items".into(),
                ty: T::List(Box::new(T::Nominal(symbol("State")))),
                presence: if presence == FieldPresence::Optional {
                    FieldPresence::Required
                } else {
                    presence
                },
                restrictions: FieldRestrictions::default(),
                default: (presence == FieldPresence::Defaulted).then(|| ClosedValue::List(vec![])),
            }])),
            body: None,
        },
        D {
            identity: symbol("State"),
            public: true,
            signature: S::Definition(CompositionBody::Variant(vec![CompositionAlternative {
                tag: "ready".into(),
                ty: T::Bool,
            }])),
            body: None,
        },
        binding(
            "operation",
            T::Nominal(symbol("State")),
            ExpressionBody::Input,
        ),
        binding("result", T::Bool, ExpressionBody::Expression(expression)),
    ]);
    if presence == FieldPresence::Optional {
        ir.vocabularies.push(optional_catalogue());
    }
    // Output: bool/kind 10 + target 19 + nominal 23 (vocabulary 27) + selectors 31.
    ir.expression_resources = ExpressionResourceFacts {
        residual_nodes: if nested { 4 } else { 1 },
        residual_depth: if nested { 2 } else { 1 },
        input_declarations: 1,
        output_leaves: 1,
        potential_value_edges: 1,
        retained_expression_text_bytes: match (presence, nested) {
            (FieldPresence::Optional, true) => 119,
            (FieldPresence::Optional, false) => 87,
            (_, true) => 115,
            (_, false) => 83,
        },
        ..ExpressionResourceFacts::default()
    };
    ir
}

/// Provides an exact public vocabulary revision containing the optional output field.
fn optional_catalogue() -> CompositionBundle {
    use neutral_core::VocabularyContentDigest;
    use neutral_ir::VocabularyIdentity;
    use neutral_vocabulary::composition::{ENCODING_VERSION, REQUIRED_FEATURE, SCHEMA_VERSION};
    CompositionBundle {
        identity: VocabularyIdentity::new(
            "Domain",
            "1.0.0",
            SCHEMA_VERSION,
            ENCODING_VERSION,
            VocabularyContentDigest::from_bytes(b"selector optional contract"),
            vec![REQUIRED_FEATURE.into()],
        ),
        dependencies: vec![],
        definitions: vec![
            CompositionDefinition {
                name: "Bucket".into(),
                public: true,
                body: CompositionBody::Record(vec![CompositionField {
                    name: "items".into(),
                    ty: T::List(Box::new(T::VocabularyNominal {
                        identity: "Domain".into(),
                        version: "1.0.0".into(),
                        name: "State".into(),
                    })),
                    presence: FieldPresence::Optional,
                    restrictions: FieldRestrictions::default(),
                    default: None,
                }]),
            },
            CompositionDefinition {
                name: "State".into(),
                public: true,
                body: CompositionBody::Variant(vec![CompositionAlternative {
                    tag: "ready".into(),
                    ty: T::Bool,
                }]),
            },
        ],
    }
}

/// Exposes the retained output in the direct-output fixture for malformed-path cases.
fn output_mut(ir: &mut ExpressionProjectIr) -> &mut Expression {
    let Some(ExpressionBody::Expression(output)) = &mut ir.declarations[3].body else {
        panic!("fixture must retain an expression body");
    };
    output
}

/// Replaces a direct output's exact selector without generating expected semantic facts.
fn selector_mut(ir: &mut ExpressionProjectIr) -> &mut Vec<SelectorSegment> {
    let ExpressionNode::Output { selector, .. } = &mut output_mut(ir).node else {
        panic!("fixture must retain an output");
    };
    selector
}

/// Catches lost optional-field requirements and confusing segment indices with tree paths.
#[test]
fn optional_field_list_and_variant_retain_exact_nested_obligations() {
    let model = validate(output_project(FieldPresence::Optional, u32::MAX, true)).unwrap();
    assert_eq!(model.facts().values(), [4, 2, 1, 1, 1, 0, 119]);
    let obligations = model.availability_obligations();
    assert_eq!(obligations.len(), 3);
    for (obligation, (segment, requirement)) in
        obligations
            .iter()
            .zip([(0, A::Presence), (1, A::Index), (2, A::Alternative)])
    {
        assert_eq!(obligation.segment, segment);
        assert_eq!(obligation.requirement, requirement);
        assert_eq!(obligation.site.owner, symbol("result"));
        assert_eq!(obligation.site.root, ExpressionRoot::Binding);
        assert_eq!(obligation.site.node_path, [1]);
    }
    assert_eq!(model.restriction_obligations(), []);
}

/// Catches treating required/defaulted fields as optional or validating future indices early.
#[test]
fn required_and_defaulted_fields_only_require_future_index_and_alternative() {
    for presence in [FieldPresence::Required, FieldPresence::Defaulted] {
        for index in [0, u32::MAX] {
            let model = validate(output_project(presence, index, false)).unwrap();
            assert_eq!(model.facts().values(), [1, 1, 1, 1, 1, 0, 83]);
            let obligations = model.availability_obligations();
            assert_eq!(obligations.len(), 2);
            assert_eq!(obligations[0].segment, 1);
            assert_eq!(obligations[0].requirement, A::Index);
            assert_eq!(obligations[1].segment, 2);
            assert_eq!(obligations[1].requirement, A::Alternative);
            for obligation in obligations {
                assert_eq!(obligation.site.root, ExpressionRoot::Binding);
                assert_eq!(obligation.site.node_path, [] as [u64; 0]);
            }
        }
    }
}

/// Catches missing fields/tags, wrong segment kinds, and traversal beyond primitive payloads.
#[test]
fn selectors_reject_missing_members_and_wrong_container_kinds() {
    for selector in [
        vec![SelectorSegment::Field("missing".into())],
        vec![SelectorSegment::Index(0)],
        vec![SelectorSegment::Alternative("ready".into())],
        vec![
            SelectorSegment::Field("items".into()),
            SelectorSegment::Field("ready".into()),
        ],
        vec![
            SelectorSegment::Field("items".into()),
            SelectorSegment::Index(0),
            SelectorSegment::Field("ready".into()),
        ],
        vec![
            SelectorSegment::Field("items".into()),
            SelectorSegment::Index(0),
            SelectorSegment::Alternative("missing".into()),
        ],
        vec![
            SelectorSegment::Field("items".into()),
            SelectorSegment::Index(0),
            SelectorSegment::Alternative("ready".into()),
            SelectorSegment::Index(0),
        ],
    ] {
        let mut ir = output_project(FieldPresence::Optional, 0, false);
        *selector_mut(&mut ir) = selector;
        let error = validate(ir).unwrap_err();
        assert_eq!(error, E::Restriction);
        assert_eq!(error.code(), "NEU-EXP-008");
    }
}

/// Catches output targets that are private, missing, definitions, or primitive bindings.
#[test]
fn output_requires_an_existing_public_nominal_binding_target() {
    for case in 0..4 {
        let mut ir = output_project(FieldPresence::Required, 0, false);
        match case {
            0 => ir.declarations[2].public = false,
            1 | 2 => {
                let ExpressionNode::Output { target, .. } = &mut output_mut(&mut ir).node else {
                    unreachable!();
                };
                *target = symbol(if case == 1 { "missing" } else { "Bucket" });
            }
            _ => ir.declarations[2].signature = S::Binding(T::Bool),
        }
        let error = validate(ir).unwrap_err();
        assert_eq!(error, E::UnknownOrPrivate);
        assert_eq!(error.code(), "NEU-EXP-003");
    }
}

/// Catches private/missing result contracts even when the operation's signature is public.
#[test]
fn output_result_contract_must_be_public_and_resolved() {
    for missing in [false, true] {
        let mut ir = output_project(FieldPresence::Required, 0, false);
        if missing {
            let ExpressionNode::Output { result_type, .. } = &mut output_mut(&mut ir).node else {
                unreachable!();
            };
            *result_type = T::Nominal(symbol("Missing"));
        } else {
            ir.declarations[0].public = false;
        }
        assert_eq!(validate(ir).unwrap_err(), E::UnknownOrPrivate);
    }
}

/// Catches accepting a primitive root or forged selected-result annotation.
#[test]
fn output_checks_root_and_final_selected_types() {
    let mut primitive_root = output_project(FieldPresence::Required, 0, false);
    let ExpressionNode::Output { result_type, .. } = &mut output_mut(&mut primitive_root).node
    else {
        unreachable!();
    };
    *result_type = T::Bool;
    assert_eq!(validate(primitive_root).unwrap_err(), E::TypeMismatch);

    let mut forged = output_project(FieldPresence::Optional, 0, false);
    forged.declarations[3].signature = S::Binding(T::String);
    output_mut(&mut forged).ty = T::String;
    let error = validate(forged).unwrap_err();
    assert_eq!(error, E::TypeMismatch);
    assert_eq!(error.code(), "NEU-EXP-004");
}

/// Catches implicitly unwrapping nullable output roots, selector containers, or targets.
#[test]
fn output_selectors_never_unwrap_nullable_contracts() {
    for (case, expected) in [
        (0, E::TypeMismatch),
        (1, E::Restriction),
        (2, E::UnknownOrPrivate),
    ] {
        let mut ir = output_project(FieldPresence::Required, 0, false);
        match case {
            0 => {
                let ExpressionNode::Output { result_type, .. } = &mut output_mut(&mut ir).node
                else {
                    unreachable!();
                };
                *result_type = T::Nullable(Box::new(T::Nominal(symbol("Bucket"))));
            }
            1 => {
                let S::Definition(CompositionBody::Record(fields)) =
                    &mut ir.declarations[0].signature
                else {
                    unreachable!();
                };
                fields[0].ty =
                    T::Nullable(Box::new(T::List(Box::new(T::Nominal(symbol("State"))))));
            }
            _ => {
                ir.declarations[2].signature =
                    S::Binding(T::Nullable(Box::new(T::Nominal(symbol("State")))));
            }
        }
        assert_eq!(validate(ir).unwrap_err(), expected);
    }
}

/// Catches dropping the nominal result type when an empty selector returns the whole root.
#[test]
fn empty_selector_returns_the_exact_root_without_availability_checks() {
    let mut ir = output_project(FieldPresence::Required, 0, false);
    selector_mut(&mut ir).clear();
    output_mut(&mut ir).ty = T::Nominal(symbol("Bucket"));
    ir.declarations[3].signature = S::Binding(T::Nominal(symbol("Bucket")));
    // output/kind+nominal 29 + target 19 + root nominal 23.
    ir.expression_resources.retained_expression_text_bytes = 71;
    let model = validate(ir).unwrap();
    assert_eq!(model.facts().values(), [1, 1, 1, 1, 1, 0, 71]);
    assert_eq!(model.availability_obligations(), []);
}

/// Catches partially published obligations or poisoned immutable models after resource failure.
#[test]
fn selector_obligations_survive_allocation_failure_cancellation_and_recovery() {
    let raw = Shared::try_new(output_project(FieldPresence::Optional, 0, true)).unwrap();
    let cancel = CancellationToken::new();
    let (result, reservations) = observe(None, || {
        validate_expression_model(Shared::clone(&raw), controls(), &cancel)
    });
    let accepted = result.unwrap();
    assert!(reservations > 0);
    for index in 0..reservations {
        let (failed, _) = observe(Some(index), || {
            validate_expression_model(Shared::clone(&raw), controls(), &cancel)
        });
        let error = failed.unwrap_err();
        assert_eq!(error, E::Limit);
        assert_eq!(error.code(), "NEU-EXP-009");
        let signal = CancellationToken::new();
        let (cancelled, _) = observe_cancellation(index, &signal, || {
            validate_expression_model(Shared::clone(&raw), controls(), &signal)
        });
        let error = cancelled.unwrap_err();
        assert_eq!(error, E::Cancelled);
        assert_eq!(error.code(), "NEU-EXP-010");
        let recovered =
            validate_expression_model(Shared::clone(&raw), controls(), &cancel).unwrap();
        assert_eq!(
            recovered.availability_obligations(),
            accepted.availability_obligations()
        );
        assert_eq!(recovered.facts().values(), [4, 2, 1, 1, 1, 0, 119]);
    }
    assert_eq!(accepted.availability_obligations().len(), 3);
}

/// Catches relaxed retained caller/producer limits and debug exposure of private semantic data.
#[test]
fn accepted_model_retains_intersected_authority_and_redacts_private_debug_data() {
    let mut ir = output_project(FieldPresence::Optional, 0, true);
    let mut private = binding(
        "secret_binding",
        T::String,
        ExpressionBody::Value(V::String("private payload".into())),
    );
    private.public = false;
    ir.declarations.push(private);
    ir.limits.nodes = 100;
    ir.expression_limits.nodes = 8;
    ir.expansion_limits.applications = 17;
    let mut producer_composition = controls().composition;
    producer_composition.total_types = 6;
    ir.composition_limits = producer_composition.policy();
    let mut caller = controls();
    caller.project.nodes = 90;
    caller.expression.nodes = 4;
    caller.expansion.applications = 19;
    caller.composition.total_types = 4;
    let model = validate_expression_model(
        Shared::try_new(ir).unwrap(),
        caller,
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(model.limits().project.nodes, 90);
    assert_eq!(model.limits().expression.nodes, 4);
    assert_eq!(model.limits().expansion.applications, 17);
    assert_eq!(model.limits().composition.total_types, 4);
    assert_eq!(model.scope().sources().len(), 2);
    assert_eq!(
        model.ir().declarations[4].identity,
        symbol("secret_binding")
    );
    let debug = format!("{model:?}");
    assert_eq!(
        debug,
        "ValidatedExpressionModel { declarations: 5, availability: 3, restrictions: 0, .. }"
    );
    assert!(!debug.contains("secret_binding"));
    assert!(!debug.contains("private payload"));
}
