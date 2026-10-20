// SPDX-License-Identifier: Apache-2.0

//! Literal binding generations against independently decoded, typed expression authority.

use super::*;
use neutral_ir::composition::project::CompositionSignature;
use neutral_semantics::{OutputAssociation, evaluate_checked};

/// Uses exact symbols from the literal registry, never an ambient name lookup.
fn owner(parts: &J) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(parts[0].as_str().unwrap(), parts[1].as_str().unwrap()),
        parts[2].as_str().unwrap(),
    )
}

/// Checks preflight errors, selected demand, unavailable outcomes and complete value results separately.
pub(super) fn check(project: &ValidatedExpressionProject, case: &J, expected: &J, registry: &J) {
    let selected = ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(
            registry["profiles"]["source"].as_str().unwrap(),
            "example::expressions",
        ),
        case["binding"].as_str().unwrap(),
    );
    let supplied = case["bindings"]
        .as_array()
        .or_else(|| registry["defaults"]["bindings"].as_array())
        .unwrap();
    let owned: Vec<_> = supplied
        .iter()
        .map(|b| {
            let symbol = owner(&b["key"]["symbol"]);
            let ty = binding_type(project, b, &symbol);
            let value = binding_value(b, &ty);
            (symbol, ty, value, b["state"].as_str().unwrap())
        })
        .collect();
    let bindings: Vec<_> = owned
        .iter()
        .map(|(symbol, ty, value, state)| Binding {
            key: BindingKey::Input(symbol),
            ty,
            state: match *state {
                "value" => BindingState::Value(value),
                "unavailable" => BindingState::Unavailable,
                "failed" => BindingState::Failed,
                _ => panic!("unregistered state"),
            },
        })
        .collect();
    let cancel = CancellationToken::new();
    let dependencies = project
        .dependencies(&selected, ExpressionInspectionLimits::default(), &cancel)
        .unwrap();
    let associations: Vec<_> = dependencies
        .edges()
        .iter()
        .filter_map(|edge| match edge.key {
            BindingKey::Output {
                target,
                result_type,
                ..
            } => Some(OutputAssociation {
                target,
                result_type,
            }),
            BindingKey::Input(_) => None,
        })
        .collect();
    let request = EvaluationRequest {
        selected: &selected,
        profiles: &project.ir().semantic_profiles,
        bindings: &bindings,
        associations: &associations,
        limits: requests::controls(case, registry, true).0,
    };
    if case["cancelled"] == true {
        cancel.cancel();
    }
    let result = evaluate_checked(project.model(), &request, &cancel);
    match result {
        Err(error) | Ok(R::Failure(error)) => {
            assert_eq!(expected["status"], "failure");
            assert_eq!(error.code(), expected["code"].as_str().unwrap());
        }
        Ok(R::Unavailable(BindingKey::Input(symbol))) => {
            assert_eq!(expected["status"], "unavailable");
            assert_eq!(
                symbol.declaration_name(),
                expected["name"].as_str().unwrap()
            );
        }
        Ok(R::RequiredBindings(keys)) => {
            assert_eq!(expected["status"], "required-bindings");
            if let Some(names) = expected["names"].as_array() {
                let actual: Vec<_> = keys
                    .iter()
                    .map(|k| match k {
                        BindingKey::Input(s) => json!(s.declaration_name()),
                        BindingKey::Output { .. } => panic!("unexpected output"),
                    })
                    .collect();
                assert_eq!(actual, *names);
            } else {
                let actual: Vec<_> = keys.iter().map(|k| match k { BindingKey::Output { target, result_type: T::Nominal(result), selector } => json!({"kind":"output", "target":[target.module().language_behavior_version(),target.module().module_name(),target.declaration_name()], "result_type":[result.module().language_behavior_version(),result.module().module_name(),result.declaration_name()], "selector":selector.iter().map(|s| match s { neutral_ir::expression::SelectorSegment::Field(n) => json!(["field",n]), _ => panic!("unregistered segment") }).collect::<Vec<_>>() }), _ => panic!("unexpected demand") }).collect();
                assert_eq!(actual, *expected["keys"].as_array().unwrap());
            }
        }
        Ok(R::Value { value, .. }) => {
            assert_eq!(expected["status"], "value");
            assert_eq!(
                project_value(&value),
                expected["values"][case["binding"].as_str().unwrap()]
            );
        }
        other => panic!("unexpected result {other:?}"),
    }
}

/// Omitted type annotations use checked input contracts; explicit wrong types remain untrusted.
fn binding_type(
    project: &ValidatedExpressionProject,
    binding: &J,
    symbol: &ModuleSymbolIdentity,
) -> T {
    match binding["type"].as_str() {
        Some("num") => T::Num,
        Some("bool") => T::Bool,
        Some("string") => T::String,
        Some("nullable-num") => T::Nullable(Box::new(T::Num)),
        None => match &project
            .ir()
            .declarations
            .iter()
            .find(|d| &d.identity == symbol)
            .unwrap()
            .signature
        {
            CompositionSignature::Binding(ty) => ty.clone(),
            CompositionSignature::Definition(_) => panic!("not a binding"),
        },
        other => panic!("unregistered type {other:?}"),
    }
}

/// Retains supplied malformed types/values so production preflight, not this adapter, rejects them.
fn binding_value(binding: &J, ty: &T) -> V {
    if binding["value"].is_null() {
        return V::Null;
    }
    match ty {
        T::Num => V::Number(
            ExactNumber::from_source(binding["value"].as_str().unwrap(), 4096, 4096).unwrap(),
        ),
        T::Bool => V::Bool(binding["value"].as_bool().unwrap()),
        T::String => V::String(binding["value"].as_str().unwrap().to_owned()),
        _ => panic!("unregistered value"),
    }
}
