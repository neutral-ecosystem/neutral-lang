// SPDX-License-Identifier: Apache-2.0

//! Source-phase expression and expansion diagnostic registry.
//!
//! Capture, parsing, resolution and lowering share these frozen classifications.
//! Package versions do not select diagnostic behavior. Evaluator-only failures
//! remain owned by the evaluator; this module does not activate a source profile.
//!
//! These identifiers describe whole source-request rejection categories, not
//! rendered messages or arbitrary host errors. Phase adapters preserve cancellation/
//! resource distinctions and attach checked locations when available; consumer
//! binding/availability decisions retain their separate report registry. Renaming
//! a Rust constant is not permission to change its frozen external spelling or
//! reinterpret existing fixture oracles.

/// Unsupported exact expression profile or feature tuple.
pub(crate) const EXP_UNSUPPORTED: &str = "NEU-EXP-001";
/// Malformed expression, operator or intrinsic.
pub(crate) const EXP_SYNTAX: &str = "NEU-EXP-002";
/// Unknown or inaccessible symbol/type.
pub(crate) const EXP_NAME: &str = "NEU-EXP-003";
/// Operand, result or branch type mismatch.
pub(crate) const EXP_TYPE: &str = "NEU-EXP-004";
/// Missing, duplicate or unknown match alternative.
pub(crate) const EXP_MATCH: &str = "NEU-EXP-005";
/// Cyclic immutable value dependency.
pub(crate) const EXP_CYCLE: &str = "NEU-EXP-007";
/// Restriction or selector-contract violation.
pub(crate) const EXP_RESTRICTION: &str = "NEU-EXP-008";
/// Resource, checked-arithmetic or fallible-allocation failure.
pub(crate) const EXP_LIMIT: &str = "NEU-EXP-009";
/// Cancellation prevented complete publication.
pub(crate) const EXP_CANCELLED: &str = "NEU-EXP-010";
/// Malformed template or expansion grammar.
pub(crate) const TPL_SYNTAX: &str = "NEU-TPL-002";
/// Invalid template argument, name or lexical scope.
pub(crate) const TPL_NAME: &str = "NEU-TPL-003";
/// Template argument type or static-phase mismatch.
pub(crate) const TPL_TYPE: &str = "NEU-TPL-004";
/// Recursive template application.
pub(crate) const TPL_RECURSION: &str = "NEU-TPL-005";
/// Generated binding name collision.
pub(crate) const TPL_COLLISION: &str = "NEU-TPL-006";
/// Invalid matrix axis, filter or exclusion.
pub(crate) const TPL_AXIS: &str = "NEU-TPL-007";
/// Expansion resource or fallible-allocation failure.
pub(crate) const TPL_LIMIT: &str = "NEU-TPL-008";
