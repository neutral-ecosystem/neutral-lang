// SPDX-License-Identifier: Apache-2.0

//! Public /3 meaning has its own typed digest and retains every structural section.

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
