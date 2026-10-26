// SPDX-License-Identifier: Apache-2.0

//! Public /3 meaning has its own typed digest and retains every structural section.
//!
//! Reuses the logical builder's fixed grammar with public declarations and public
//! vocabulary definitions selected explicitly. Module/import structure, catalogue
//! and dependency contracts, semantic profiles and exported templates remain part
//! of interpretation. This is a separate domain, not a truncated complete digest.
//!
//! Filtering is not a privacy validator: independent readers must establish that
//! public signatures/bodies and retained interpretation contracts have valid public
//! closure. Source/provenance redaction for a selected view is handled at its own
//! boundary; computing this fingerprint alone does not produce a safe view.

use super::{INTERFACE_DOMAIN, logical::build};
use crate::{
    expression::project::ExpressionProjectIr,
    project_identity::{
        ExpressionInterfaceIdentity, IdentityError, IdentityLimits, IdentityTranscript,
    },
};
use neutral_core::CancellationToken;

/// Recomputes public declarations/templates and their public interpretation contracts.
///
/// # Errors
/// Rejects malformed canonical framing, bounds or cancellation. Semantic visibility
/// and closure require independent reader validation before trusting this digest.
pub fn expression_interface(
    ir: &ExpressionProjectIr,
    limits: IdentityLimits,
    cancellation: &CancellationToken,
) -> Result<IdentityTranscript<ExpressionInterfaceIdentity>, IdentityError> {
    build(
        ir,
        limits,
        cancellation,
        true,
        INTERFACE_DOMAIN,
        ExpressionInterfaceIdentity,
    )
}
