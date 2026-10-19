// SPDX-License-Identifier: Apache-2.0

//! Compiler-independent pure semantic boundaries; no parser, host effects or scheduler.
//!
//! Model authority checks retained semantics only. Complete source companions,
//! transport, identities and public projections still require independent reader validation.

mod budget;
mod decimal;
mod evaluation;
mod model;
mod validation;
mod values;
pub use evaluation::{
    Binding, BindingKey, BindingState, EvaluationError, EvaluationRequest, EvaluationResult,
    NormalizedBinding, OutputAssociation, evaluate, evaluate_checked, normalize_binding,
    validate_evaluation_bindings,
};
pub use model::*;
pub use validation::{derive_expression_resources, validate_expression_model};

#[cfg(test)]
#[path = "../tests/unit/decimal.rs"]
mod decimal_tests;
#[cfg(test)]
#[path = "../tests/unit/values.rs"]
mod value_tests;
