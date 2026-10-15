// SPDX-License-Identifier: Apache-2.0

//! Bounded typed reuse inspection rejects oversized targets before recursive equality.

use super::*;
use neutral_core::CancellationToken;

/// Internal reuse scans enforce independent type and retained-value work bounds.
#[test]
fn reuse_target_preflight_rejects_exhausted_type_and_value_work() {
    let ir = super::super::tests::empty();
    let cancel = CancellationToken::new();
    let ty = T::String;
    let value = V::String("target".repeat(100));
    let type_work = neutral_ir::expression::inspect_expression_type_retention(
        &ty,
        neutral_ir::expression::profile::ExpressionLimits::hard(),
        &cancel,
    )
    .unwrap()
    .work;
    for work in [0, type_work] {
        let mut budget = Budget {
            work,
            cancel: &cancel,
        };
        assert_eq!(
            contains_reuse(&ir, &value, &ty, &value, &ty, &mut budget),
            Err(E::Limit)
        );
    }
    let mut budget = Budget {
        work: neutral_ir::expression::profile::ExpressionLimits::hard().work,
        cancel: &cancel,
    };
    assert_eq!(
        contains_reuse(&ir, &value, &ty, &value, &ty, &mut budget),
        Ok(true)
    );
    cancel.cancel();
    assert_eq!(
        contains_reuse(&ir, &value, &ty, &value, &ty, &mut budget),
        Err(E::Cancelled)
    );
}

/// Reuse inside a selected variant payload must match the exact tagged alternative type.
#[test]
fn variant_reuse_checks_the_selected_payload() {
    let mut ir = super::super::tests::empty();
    let owner =
        ModuleSymbolIdentity::new(neutral_ir::LogicalModuleIdentity::new("1.0", "m"), "Choice");
    ir.declarations
        .push(neutral_ir::expression::project::ExpressionDeclaration {
            identity: owner.clone(),
            public: true,
            signature: S::Definition(B::Variant(vec![
                neutral_ir::composition::CompositionAlternative {
                    tag: "ready".into(),
                    ty: T::String,
                },
            ])),
            body: None,
        });
    let ty = T::Nominal(owner);
    let target = V::String("retained".into());
    let cancel = CancellationToken::new();
    for (tag, expected) in [("ready", Ok(true)), ("unknown", Err(E::Semantic))] {
        let value = V::Variant {
            tag: tag.into(),
            payload: Box::new(V::String("retained".into())),
        };
        assert_eq!(
            contains_reuse(
                &ir,
                &value,
                &ty,
                &target,
                &T::String,
                &mut Budget {
                    work: 1000,
                    cancel: &cancel
                }
            ),
            expected
        );
    }
}
