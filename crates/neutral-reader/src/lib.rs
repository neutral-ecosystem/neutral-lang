// SPDX-License-Identifier: Apache-2.0

//! Validated, immutable access to current Neutral project artifacts.

pub mod composition;
pub mod consumer;
pub mod expression;
pub use expression::{ExpressionReadError, ValidatedExpressionProject};
pub use neutral_semantics::SemanticLimits;

pub use neutral_ir::project_identity::{
    IdentityError, IdentityLimits, MAX_TRANSCRIPT_BYTES, MAX_TRANSCRIPT_NODES,
};

pub use neutral_core::profile::{
    LanguageCapability, LanguageProfile, LanguageProfileDescriptor, ProfileAvailability,
};
pub use neutral_ir::ModuleSymbolIdentity;

/// Returns the shared deterministic language-profile capability catalogue.
#[must_use]
pub const fn language_profiles() -> &'static [LanguageProfileDescriptor] {
    neutral_core::profile::language_profiles()
}
