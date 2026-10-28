// SPDX-License-Identifier: Apache-2.0

//! Explicit data-only consumer requests over independently decoded authority.
//!
//! Names here belong to the registered consumer fixture, not universal operation
//! conventions. Other fuzzed projects may reject this policy; that is expected.
//! Byte zero selects the guard, duplicate or unknown key; byte one supplies a
//! bounded number; byte two selects unavailable/failed/valid/malformed/missing output.

use super::{SOURCE_MODULE, semantic_limits};
use neutral_core::{CancellationToken, profile::LanguageProfile};
use neutral_ir::{
    ExactNumber, LogicalModuleIdentity, ModuleSymbolIdentity, composition::BindingValue as V,
    expression::SelectorSegment, project_interface::ProjectPublicType as T,
};
use neutral_reader::{
    ValidatedExpressionProject,
    consumer::{
        ConsumerContext, ConsumerLimits, ConsumerProtocol,
        expression::{
            ExpressionConsumerBoundary, ExpressionConsumerError, ExpressionConsumerPolicy,
            ExpressionEvaluationReport,
        },
    },
};
use neutral_semantics::{Binding, BindingKey, BindingState, EvaluationRequest, OutputAssociation};

/// Creates an exact symbol in the fuzz fixture's logical namespace.
fn symbol(name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(LanguageProfile::V1_0.source_version(), SOURCE_MODULE),
        name,
    )
}

/// Validates explicit envelopes and requests pure evaluation, without scheduling or acquiring inputs.
pub fn consume<'p>(
    project: &'p ValidatedExpressionProject,
    bytes: &[u8],
) -> Result<ExpressionEvaluationReport<'p, 'static>, ExpressionConsumerError> {
    let cancel = CancellationToken::new();
    let flags = bytes.first().copied().unwrap_or_default();
    let enabled = symbol(if flags & 4 == 0 { "enabled" } else { "unknown" });
    let producer = symbol("producer");
    let selected = symbol("entry");
    let result = T::Nominal(symbol("Result"));
    let report = T::Nominal(symbol("Report"));
    let associations = [OutputAssociation {
        target: &producer,
        result_type: &result,
    }];
    let boundary = ExpressionConsumerBoundary::new(
        project,
        ExpressionConsumerPolicy {
            protocol: ConsumerProtocol {
                report_contract: &report,
                supported_features: neutral_ir::expression::profile::REQUIRED_FEATURES,
                diagnostic_codes: &["FUZZ-001"],
            },
            associations: &associations,
        },
        ConsumerLimits::default(),
        &cancel,
    )?;
    let boolean = V::Bool(flags & 1 != 0);
    let number = V::Number(
        ExactNumber::from_source(
            &bytes.get(1).copied().unwrap_or_default().to_string(),
            16,
            16,
        )
        .unwrap(),
    );
    let output = V::Record(vec![(
        "values".into(),
        Some(V::List(vec![number.clone(), number])),
    )]);
    let malformed = V::Bool(false);
    let selector = [
        SelectorSegment::Field("values".into()),
        SelectorSegment::Index(1),
    ];
    let state = bytes.get(2).copied().unwrap_or_default() % 6;
    let mut bindings = vec![Binding {
        key: BindingKey::Input(&enabled),
        ty: &T::Bool,
        state: BindingState::Value(&boolean),
    }];
    if state != 5 {
        bindings.push(Binding {
            key: BindingKey::Output {
                target: &producer,
                result_type: &result,
                selector: &selector,
            },
            ty: &T::Num,
            state: match state {
                0 => BindingState::Unavailable,
                1 => BindingState::Failed,
                2 => BindingState::OutputResult(&output),
                3 => BindingState::OutputResult(&malformed),
                _ => BindingState::Value(&output),
            },
        });
    }
    if flags & 2 != 0 {
        bindings.push(bindings[0]);
    }
    let request = EvaluationRequest {
        selected: &selected,
        profiles: &project.ir().semantic_profiles,
        bindings: &bindings,
        associations: &[],
        limits: semantic_limits().expression,
    };
    boundary.validate_bindings(&request, &cancel)?;
    let evaluated = boundary.evaluate(
        &request,
        ConsumerContext {
            adapter_revision: "fuzz-adapter",
            target_revision: "fuzz-target",
            input_revision: "fuzz-input",
        },
        &cancel,
    )?;
    assert_eq!(evaluated.project_identity, boundary.project_identity());
    Ok(evaluated)
}
