// SPDX-License-Identifier: Apache-2.0

//! Ignored implementation backlog for meaningful gaps in the measured compiler frontend.
//!
//! These placeholders deliberately execute no behavior and are not coverage evidence.
//! Private-helper cases must use the owning unit-test scope when implementation begins.

#[path = "coverage_gaps/composition_assembly.rs"]
mod composition_assembly;
#[path = "coverage_gaps/composition_attribution.rs"]
mod composition_attribution;
#[path = "coverage_gaps/expression_capture.rs"]
mod expression_capture;
#[path = "coverage_gaps/expressions_parser.rs"]
mod expressions_parser;
#[path = "coverage_gaps/expressions_resolution.rs"]
mod expressions_resolution;
#[path = "coverage_gaps/frontend_expansion.rs"]
mod frontend_expansion;
