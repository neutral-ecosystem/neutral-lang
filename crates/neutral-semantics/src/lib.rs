// SPDX-License-Identifier: Apache-2.0

//! Compiler-independent pure semantic boundaries; no parser, host effects or scheduler.
//!
//! Model authority checks retained semantics only. Complete source companions,
//! transport, identities and public projections still require independent reader validation.

mod model;
mod validation;
pub use model::*;
pub use validation::validate_expression_model;
