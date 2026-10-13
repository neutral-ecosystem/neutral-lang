// SPDX-License-Identifier: Apache-2.0

//! Default/body obligations have distinct roots and survive request failure independently.

use super::*;
use neutral_ir::expression::templates::{ParameterPhase, PublicTemplate, TemplateParameter};
use neutral_semantics::ExpressionRoot;

fn template_record() -> Expression {
    Expression {
        ty: T::Nominal(symbol("Bucket")),
        node: ExpressionNode::Record(vec![(
            "count".into(),
            Some(Expression {
                ty: T::Num,
                node: ExpressionNode::Bound { depth: 0, slot: 0 },
            }),
        )]),
    }
}

#[test]
fn template_default_and_body_obligations_are_distinct_and_fault_safe() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    let mut raw = project(vec![D {
        identity: symbol("Bucket"),
        public: true,
        signature: S::Definition(CompositionBody::Record(vec![CompositionField {
            name: "count".into(),
            ty: T::Num,
            presence: FieldPresence::Required,
            restrictions: FieldRestrictions {
                minimum: Some(neutral_ir::ExactNumber::from_unsigned_integer("1").unwrap()),
                ..FieldRestrictions::default()
            },
            default: None,
        }])),
        body: None,
    }]);
    raw.public_templates.push(PublicTemplate {
        identity: symbol("Make"),
        parameters: vec![
            TemplateParameter {
                name: "first".into(),
                phase: ParameterPhase::Deferred,
                ty: T::Num,
                default: None,
            },
            TemplateParameter {
                name: "second".into(),
                phase: ParameterPhase::Deferred,
                ty: T::Nominal(symbol("Bucket")),
                default: Some(template_record()),
            },
        ],
        result_type: T::Nominal(symbol("Bucket")),
        body: template_record(),
    });
    // Each record has 29 type/kind bytes + five field-name bytes + eight bound/num bytes.
    raw.expression_resources = ExpressionResourceFacts {
        residual_nodes: 4,
        residual_depth: 2,
        public_templates: 1,
        retained_expression_text_bytes: 84,
        ..ExpressionResourceFacts::default()
    };
    let raw = Shared::try_new(raw).unwrap();
    let cancel = CancellationToken::new();
    let (result, reservations) = observe(None, || {
        validate_expression_model(Shared::clone(&raw), controls(), &cancel)
    });
    let accepted = result.unwrap();
    let obligations = accepted.restriction_obligations();
    assert_eq!(obligations.len(), 2);
    assert_eq!(obligations[0].site.root, ExpressionRoot::TemplateDefault(1));
    assert_eq!(obligations[1].site.root, ExpressionRoot::TemplateBody);
    for obligation in obligations {
        assert_eq!(obligation.site.owner, symbol("Make"));
        assert_eq!(obligation.site.node_path, [0]);
        assert_eq!(obligation.contract, T::Nominal(symbol("Bucket")));
        assert_eq!(obligation.field, "count");
    }
    for index in 0..reservations {
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
    assert_eq!(accepted.restriction_obligations().len(), 2);
}
