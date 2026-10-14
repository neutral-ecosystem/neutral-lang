// SPDX-License-Identifier: Apache-2.0

//! Reader-only /3 inspection; no compiler, source reparse, evaluator or acquisition path.

use neutral_core::CancellationToken;
use neutral_encoding::{DecodeError, DecodeLimits, expression::decode_expression_project};
use neutral_reader::{
    IdentityError, IdentityLimits, SemanticLimits,
    consumer::EntryRequest,
    expression::{
        ExpressionEntryError, ExpressionInterfaceIdentity, ExpressionLogicalIdentity,
        ExpressionResourceFacts, ExpressionView, ExpressionViewLimits,
        canonical_expression_project, expression_interface, profile::SemanticProfiles,
    },
};

/// Exact independently bounded public entry selection after complete wire validation.
pub struct ExpressionEntryProbeRequest<'a> {
    /// Complete exact semantic profile/features, never inferred from source or package version.
    pub semantic_profiles: &'a SemanticProfiles,
    /// Exact nominal public entry contract and cardinality policy.
    pub entry: EntryRequest<'a>,
    /// Independent public selection/projection controls.
    pub limits: ExpressionViewLimits,
}
/// Atomic inspection failures without private artifact values, names or source identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpressionProbeError {
    /// Complete hostile-wire or independent reader validation failure.
    Decode(DecodeError),
    /// Exact public contract/profile/selection or projection failure.
    Entry(ExpressionEntryError),
    /// Independent typed identity framing failure.
    Identity(IdentityError),
}
/// Safe complete counts and independently derived typed identities with a redacted public view.
#[derive(Debug)]
pub struct ExpressionProbeSummary {
    /// Complete module count, never private module names.
    pub modules: u64,
    /// Complete declaration count, independent of selected roots.
    pub declarations: u64,
    /// Independently recomputed complete retained successor facts.
    pub resources: ExpressionResourceFacts,
    /// Complete typed logical meaning, never rewritten by roots or consumer bindings.
    pub logical_identity: ExpressionLogicalIdentity,
    /// Independently checked typed complete public interpretive digest.
    pub interface_identity: ExpressionInterfaceIdentity,
    /// Selected public interpretation closure without raw complete-project access.
    pub view: ExpressionView,
}

/// Independently decodes a complete /3 artifact and projects exact public entry contracts.
/// # Errors
/// Rejects unsupported/malformed bytes, profiles, absent contracts, invalid selections,
/// independent limits, allocation or cancellation without publishing a partial summary.
pub fn inspect_expression_encoded(
    bytes: &[u8],
    wire: DecodeLimits,
    semantic: SemanticLimits,
    request: &ExpressionEntryProbeRequest<'_>,
    cancel: &CancellationToken,
) -> Result<ExpressionProbeSummary, ExpressionProbeError> {
    if cancel.is_cancelled() {
        return Err(ExpressionProbeError::Entry(ExpressionEntryError::Cancelled));
    }
    if !request.semantic_profiles.matches_frozen() {
        return Err(ExpressionProbeError::Entry(
            ExpressionEntryError::Unsupported,
        ));
    }
    let project = decode_expression_project(bytes, wire, semantic, cancel)
        .map_err(ExpressionProbeError::Decode)?;
    let overhead = std::mem::size_of::<ExpressionProbeSummary>() as u64;
    let limits = ExpressionViewLimits {
        output_bytes: request
            .limits
            .output_bytes
            .checked_sub(overhead)
            .ok_or(ExpressionProbeError::Entry(ExpressionEntryError::Limit))?,
        work: request
            .limits
            .work
            .checked_sub(overhead + 1)
            .ok_or(ExpressionProbeError::Entry(ExpressionEntryError::Limit))?,
        ..request.limits
    };
    let identity_limits = IdentityLimits {
        bytes: project.model().limits().project.artifact_bytes,
        nodes: project.model().limits().project.nodes,
    };
    let logical_identity = canonical_expression_project(project.ir(), identity_limits, cancel)
        .map_err(ExpressionProbeError::Identity)?
        .identity();
    let interface_identity = expression_interface(project.ir(), identity_limits, cancel)
        .map_err(ExpressionProbeError::Identity)?
        .identity();
    let selected = project
        .entries(&request.entry, request.semantic_profiles, limits, cancel)
        .map_err(ExpressionProbeError::Entry)?;
    let view = selected
        .derive_view(cancel)
        .map_err(ExpressionProbeError::Entry)?;
    if cancel.is_cancelled() {
        return Err(ExpressionProbeError::Entry(ExpressionEntryError::Cancelled));
    }
    Ok(ExpressionProbeSummary {
        modules: project.ir().modules.len() as u64,
        declarations: project.ir().declarations.len() as u64,
        resources: project.model().facts(),
        logical_identity,
        interface_identity,
        view,
    })
}
