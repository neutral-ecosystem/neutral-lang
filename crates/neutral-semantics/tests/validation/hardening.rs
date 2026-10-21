// SPDX-License-Identifier: Apache-2.0

//! Request-wide limits, materialized nested values and atomic semantic validation failures.

use super::*;
use neutral_ir::composition::BindingValue as V;

fn field(name: &str, ty: T) -> CompositionField {
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

#[test]
fn nested_nominal_values_without_restrictions_are_valid() {
    let ir = project(vec![
        definition("Inner", vec![field("flag", T::Bool)]),
        definition("Outer", vec![field("inner", T::Nominal(symbol("Inner")))]),
        binding(
            "result",
            T::Nominal(symbol("Outer")),
            ExpressionBody::Value(V::Record(vec![(
                "inner".into(),
                Some(V::Record(vec![("flag".into(), Some(V::Bool(true)))])),
            )])),
        ),
    ]);
    assert!(validate(ir).is_ok());
}

#[test]
fn restriction_checks_consume_shared_work_instead_of_resetting_it() {
    let mut constrained = field("count", T::Num);
    constrained.restrictions.minimum =
        Some(neutral_ir::ExactNumber::from_unsigned_integer("1").unwrap());
    let value = V::Number(neutral_ir::ExactNumber::from_unsigned_integer("2").unwrap());
    let limits = controls().composition;
    let mut work = limits.work;
    neutral_vocabulary::composition::check_composition_field_restrictions_budgeted(
        &constrained,
        &value,
        limits,
        &mut work,
        &CancellationToken::new(),
    )
    .unwrap();
    assert!(work < limits.work);
    work = 1;
    assert!(
        neutral_vocabulary::composition::check_composition_field_restrictions_budgeted(
            &constrained,
            &value,
            limits,
            &mut work,
            &CancellationToken::new()
        )
        .is_err()
    );
}

#[test]
fn validated_field_restriction_checks_do_not_clone_contract_choices() {
    use neutral_core::allocation::testing::observe;
    let mut constrained = field("flag", T::Bool);
    constrained.restrictions.choices = Some(vec![neutral_ir::composition::ClosedValue::Bool(true)]);
    let model = validate(project(vec![definition("Choice", vec![constrained])])).unwrap();
    let mut work = controls().composition.work;
    let owner = T::Nominal(symbol("Choice"));
    let signal = CancellationToken::new();
    let (result, allocations) = observe(None, || {
        model.scope().check_field_restrictions(
            &owner,
            "flag",
            &V::Bool(true),
            controls().composition,
            &mut work,
            &signal,
        )
    });
    assert!(result.is_ok());
    assert_eq!(allocations, 0);
    assert!(work < controls().composition.work);
    assert!(
        model
            .scope()
            .check_field_restrictions(
                &T::Nominal(symbol("Choice")),
                "flag",
                &V::Bool(false),
                controls().composition,
                &mut work,
                &CancellationToken::new()
            )
            .is_err()
    );
}

#[test]
fn stale_binding_and_template_facts_cannot_be_accepted() {
    let mut ir = project(vec![binding(
        "result",
        T::Bool,
        ExpressionBody::Value(V::Bool(true)),
    )]);
    ir.expression_resources.residual_nodes = 1;
    assert_eq!(validate(ir).unwrap_err(), E::Resources);
    let mut ir = project(vec![]);
    ir.public_templates
        .push(neutral_ir::expression::templates::PublicTemplate {
            identity: symbol("Template"),
            parameters: vec![],
            result_type: T::Bool,
            body: leaf(true),
        });
    assert_eq!(validate(ir).unwrap_err(), E::Resources);
}

#[test]
fn model_validation_faults_and_cancellation_are_atomic() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    let ir = Shared::try_new(project(vec![binding(
        "external",
        T::Bool,
        ExpressionBody::Input,
    )]))
    .unwrap();
    // A declared input has no residual tree, but counts as a real input declaration.
    let mut raw = project(vec![binding("external", T::Bool, ExpressionBody::Input)]);
    raw.expression_resources.input_declarations = 1;
    let valid = Shared::try_new(raw).unwrap();
    let cancel = CancellationToken::new();
    let (result, reservations) = observe(None, || {
        validate_expression_model(Shared::clone(&valid), controls(), &cancel)
    });
    assert!(result.is_ok());
    for index in 0..reservations {
        let (result, _) = observe(Some(index), || {
            validate_expression_model(Shared::clone(&valid), controls(), &cancel)
        });
        assert_eq!(result.unwrap_err(), E::Limit);
        let signal = CancellationToken::new();
        let (result, _) = observe_cancellation(index, &signal, || {
            validate_expression_model(Shared::clone(&valid), controls(), &signal)
        });
        assert_eq!(result.unwrap_err(), E::Cancelled);
        assert!(validate_expression_model(Shared::clone(&valid), controls(), &cancel).is_ok());
    }
    assert_eq!(
        validate_expression_model(ir, controls(), &cancel).unwrap_err(),
        E::Resources
    );
}

#[test]
fn import_edges_obey_aggregate_exact_boundary() {
    let mut ir = project(vec![]);
    ir.modules = ["alpha", "beta", "gamma"]
        .into_iter()
        .map(|name| ProjectModule {
            identity: LogicalModuleIdentity::new("1.0", name),
            imports: if name == "gamma" {
                vec![]
            } else {
                vec!["gamma".into()]
            },
        })
        .collect();
    ir.limits.import_edges = 2;
    assert!(validate(ir).is_ok());
    let mut ir = project(vec![]);
    ir.modules = ["alpha", "beta", "gamma"]
        .into_iter()
        .map(|name| ProjectModule {
            identity: LogicalModuleIdentity::new("1.0", name),
            imports: if name == "gamma" {
                vec![]
            } else {
                vec!["gamma".into()]
            },
        })
        .collect();
    ir.limits.import_edges = 1;
    assert_eq!(validate(ir).unwrap_err(), E::Limit);
}

#[test]
fn retained_contracts_cannot_differ_from_canonical_authority() {
    let ir = project(vec![definition(
        "Pair",
        vec![field("z", T::Bool), field("a", T::Bool)],
    )]);
    assert_eq!(validate(ir).unwrap_err(), E::Malformed);
    let mut choice = field("flag", T::Bool);
    choice.restrictions.choices = Some(vec![
        neutral_ir::composition::ClosedValue::Bool(true),
        neutral_ir::composition::ClosedValue::Bool(false),
    ]);
    assert_eq!(
        validate(project(vec![definition("Choice", vec![choice])])).unwrap_err(),
        E::Malformed
    );
}

#[test]
fn materialized_values_share_retention_budget() {
    let make = || {
        project(vec![
            binding(
                "first",
                T::String,
                ExpressionBody::Value(V::String("x".repeat(64))),
            ),
            binding(
                "second",
                T::String,
                ExpressionBody::Value(V::String("x".repeat(64))),
            ),
        ])
    };
    // The independently owned catalogue/scope payloads also survive validation.
    let context_bytes =
        (std::mem::size_of::<neutral_vocabulary::composition::ValidatedCompositionScope>()
            + std::mem::size_of::<neutral_vocabulary::composition::ValidatedComposition>())
            as u64;
    let bytes = context_bytes + 2 * (std::mem::size_of::<V>() as u64 + 64);
    let mut exact = make();
    exact.expression_limits.retained_bytes = bytes;
    assert!(validate(exact).is_ok());
    let mut over = make();
    over.expression_limits.retained_bytes = bytes - 1;
    assert_eq!(validate(over).unwrap_err(), E::Limit);
}

#[test]
fn deferred_restrictions_are_derived_and_fault_safe() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    let mut count = field("count", T::Num);
    count.restrictions.minimum = Some(neutral_ir::ExactNumber::from_unsigned_integer("1").unwrap());
    let mut raw = project(vec![
        definition("Bucket", vec![count]),
        binding("external", T::Num, ExpressionBody::Input),
        binding(
            "result",
            T::Nominal(symbol("Bucket")),
            ExpressionBody::Expression(Expression {
                ty: T::Nominal(symbol("Bucket")),
                node: ExpressionNode::Record(vec![(
                    "count".into(),
                    Some(Expression {
                        ty: T::Num,
                        node: ExpressionNode::Input(symbol("external")),
                    }),
                )]),
            }),
        ),
    ]);
    raw.expression_resources = ExpressionResourceFacts {
        residual_nodes: 2,
        residual_depth: 2,
        input_declarations: 1,
        potential_value_edges: 1,
        retained_expression_text_bytes: 60,
        ..ExpressionResourceFacts::default()
    };
    let raw = Shared::try_new(raw).unwrap();
    let cancel = CancellationToken::new();
    let (result, count) = observe(None, || {
        validate_expression_model(Shared::clone(&raw), controls(), &cancel)
    });
    let model = result.unwrap();
    let obligations = model.restriction_obligations();
    assert_eq!(obligations.len(), 1);
    assert_eq!(obligations[0].site.owner, symbol("result"));
    assert_eq!(obligations[0].site.node_path, [0]);
    assert_eq!(obligations[0].contract, T::Nominal(symbol("Bucket")));
    assert_eq!(obligations[0].field, "count");
    for index in 0..count {
        let (result, _) = observe(Some(index), || {
            validate_expression_model(Shared::clone(&raw), controls(), &cancel)
        });
        assert_eq!(result.unwrap_err(), E::Limit);
        let signal = CancellationToken::new();
        let (result, _) = observe_cancellation(index, &signal, || {
            validate_expression_model(Shared::clone(&raw), controls(), &signal)
        });
        assert_eq!(result.unwrap_err(), E::Cancelled);
        assert!(validate_expression_model(Shared::clone(&raw), controls(), &cancel).is_ok());
    }
    assert_eq!(model.restriction_obligations().len(), 1);
}
