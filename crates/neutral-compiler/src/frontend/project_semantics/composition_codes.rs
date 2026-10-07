// SPDX-License-Identifier: Apache-2.0

//! Frozen successor source classifications, separate from captured catalogue errors.

/// Malformed source declaration or contextual literal.
pub const INVALID_SOURCE: &str = "NEU-COMP-001";
/// Duplicate tag/member or missing/unknown contextual variant member.
pub const DUPLICATE_MEMBER: &str = "NEU-COMP-002";
/// Unknown selected discriminator.
pub const UNKNOWN_TAG: &str = "NEU-COMP-003";
/// Invariant type/payload/reuse/reference mismatch.
pub const INCOMPATIBLE_VALUE: &str = "NEU-COMP-004";
/// Required field omitted, including a required nullable field.
pub const MISSING_REQUIRED_FIELD: &str = "NEU-COMP-005";
/// A materialized field violates a declarative restriction.
pub const RESTRICTION_VIOLATION: &str = "NEU-COMP-006";
/// Inaccessible private type or exposed private reference target.
pub const PRIVATE_EXPOSURE: &str = "NEU-COMP-007";
/// Embedded/default dependency cycle.
pub const EMBEDDED_CYCLE: &str = "NEU-COMP-008";
/// Ordinary immutable reuse evaluates cyclically.
pub const REUSE_CYCLE: &str = "NEU-COMP-009";
/// Independent work/depth/byte/allocation bound.
pub const LIMIT: &str = "NEU-COMP-010";
/// Cancellation prevented complete publication.
pub const CANCELLED: &str = "NEU-COMP-011";
