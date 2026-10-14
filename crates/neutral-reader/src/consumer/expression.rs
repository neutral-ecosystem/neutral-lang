// SPDX-License-Identifier: Apache-2.0

//! Explicit consumer policy and binding reports; no scheduler, callback or ambient lookup.

use super::{
    ConsumerContext, ConsumerError, ConsumerLimits, ConsumerProtocol,
    validation::{Budget, public_scope_type},
};
use crate::expression::ValidatedExpressionProject;
use neutral_core::CancellationToken;
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::project::CompositionSignature as S,
    project_identity::{
        ExpressionLogicalIdentity, IdentityError, IdentityLimits, canonical_expression_project,
    },
    project_interface::ProjectPublicType as T,
};
use neutral_semantics::{
    EvaluationError, EvaluationRequest, EvaluationResult, OutputAssociation, evaluate,
    validate_evaluation_bindings,
};

/// Exact report/features registration and consumer-owned public output associations.
#[derive(Clone, Copy)]
pub struct ExpressionConsumerPolicy<'a> {
    /// Closed report contract and explicitly registered consumer features/codes.
    pub protocol: ConsumerProtocol<'a>,
    /// Public associations; core cannot prove execution or authorization.
    pub associations: &'a [OutputAssociation<'a>],
}
/// Safe consumer/pure-evaluation failure without private payloads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpressionConsumerError {
    /// Consumer policy/compatibility/envelope rejection.
    Consumer(ConsumerError),
    /// Explicit binding or pure-evaluation failure.
    Evaluation(EvaluationError),
}
impl From<ConsumerError> for ExpressionConsumerError {
    /// Preserves the separate consumer-phase classification.
    fn from(error: ConsumerError) -> Self {
        Self::Consumer(error)
    }
}
impl From<EvaluationError> for ExpressionConsumerError {
    /// Preserves safe evaluator codes without compiler impersonation.
    fn from(error: EvaluationError) -> Self {
        Self::Evaluation(error)
    }
}
/// Explicit result bound to immutable project meaning and opaque consumer revisions.
pub struct ExpressionEvaluationReport<'p, 'r> {
    /// Complete logical identity, independent of roots and external values.
    pub project_identity: ExpressionLogicalIdentity,
    /// Canonical public selected entry.
    pub entry: &'p ModuleSymbolIdentity,
    /// Consumer context; it never replaces or rewrites project identity.
    pub context: ConsumerContext<'r>,
    /// Complete value, selected demand, unavailable state or structured failure.
    pub outcome: EvaluationResult<'p>,
}
/// Explicit consumer policy over complete independently checked artifact authority.
pub struct ExpressionConsumerBoundary<'p, 'a> {
    /// Complete artifact authority, not raw-model typing alone.
    project: &'p ValidatedExpressionProject,
    /// Immutable registered policy; never a callable backend.
    policy: ExpressionConsumerPolicy<'a>,
    /// Independently intersected consumer controls.
    limits: ConsumerLimits,
    /// Complete logical identity computed without consumer context or roots.
    identity: ExpressionLogicalIdentity,
}
impl<'p, 'a> ExpressionConsumerBoundary<'p, 'a> {
    /// Checks public report/result contracts, exact features and explicit associations.
    /// # Errors
    /// Rejects malformed/private/unsupported registrations and independent limits/cancellation.
    pub fn new(
        project: &'p ValidatedExpressionProject,
        policy: ExpressionConsumerPolicy<'a>,
        limits: ConsumerLimits,
        cancellation: &CancellationToken,
    ) -> Result<Self, ExpressionConsumerError> {
        let limits = limits.effective(project.model().limits().expression.work)?;
        let mut budget = Budget::new(limits, cancellation);
        budget.list(policy.protocol.supported_features, limits.features)?;
        budget.list(policy.protocol.diagnostic_codes, limits.diagnostics)?;
        if policy
            .protocol
            .diagnostic_codes
            .iter()
            .any(|c| c.starts_with("NEU-"))
        {
            return Err(ConsumerError::InvalidInput.into());
        }
        public_scope_type(
            project.model().scope(),
            policy.protocol.report_contract,
            &mut budget,
        )?;
        for feature in &project.ir().semantic_profiles.required_features {
            budget.text(feature)?;
            budget.work(policy.protocol.supported_features.len() as u64 + 1)?;
            if !policy
                .protocol
                .supported_features
                .contains(&feature.as_str())
            {
                return Err(ConsumerError::Unsupported.into());
            }
        }
        if policy.associations.len() as u64 > limits.closure_edges {
            return Err(ConsumerError::Limit.into());
        }
        for (index, association) in policy.associations.iter().enumerate() {
            budget.symbol(association.target)?;
            public_scope_type(
                project.model().scope(),
                association.result_type,
                &mut budget,
            )?;
            let d = project
                .ir()
                .declarations
                .binary_search_by(|d| d.identity.cmp(association.target))
                .ok()
                .map(|i| &project.ir().declarations[i])
                .ok_or(ConsumerError::PrivateTarget)?;
            if !d.public
                || !matches!(
                    &d.signature,
                    S::Binding(T::Nominal(_) | T::VocabularyNominal { .. })
                )
            {
                return Err(ConsumerError::PrivateTarget.into());
            }
            budget.work(index as u64 + 1)?;
            if policy.associations[..index]
                .iter()
                .any(|a| a == association)
            {
                return Err(ConsumerError::InvalidInput.into());
            }
        }
        let identity = canonical_expression_project(
            project.ir(),
            IdentityLimits {
                bytes: limits.bytes,
                nodes: limits
                    .work
                    .min(neutral_ir::project_identity::MAX_TRANSCRIPT_NODES),
            },
            cancellation,
        )
        .map_err(|e| match e {
            IdentityError::Cancelled => ConsumerError::Cancelled,
            IdentityError::Limit => ConsumerError::Limit,
            IdentityError::InvalidInput => ConsumerError::IncompleteContract,
        })?
        .identity();
        budget.work(0)?;
        Ok(Self {
            project,
            policy,
            limits,
            identity,
        })
    }
    /// Returns complete project identity, not an evaluation-generation fingerprint.
    #[must_use]
    pub const fn project_identity(&self) -> ExpressionLogicalIdentity {
        self.identity
    }
    /// Validates all supplied envelopes without evaluating or scheduling anything.
    /// # Errors
    /// Rejects policy overrides, malformed data and independent limits/cancellation.
    pub fn validate_bindings(
        &self,
        request: &EvaluationRequest<'_>,
        cancellation: &CancellationToken,
    ) -> Result<(), ExpressionConsumerError> {
        let request = self.request(request, cancellation)?;
        validate_evaluation_bindings(self.project.model(), &request, cancellation)?;
        Ok(())
    }
    /// Explicitly evaluates one public entry under registered policy and opaque context.
    /// # Errors
    /// Rejects malformed registration, envelopes or revisions before report publication.
    pub fn evaluate<'r>(
        &self,
        request: &EvaluationRequest<'_>,
        context: ConsumerContext<'r>,
        cancellation: &CancellationToken,
    ) -> Result<ExpressionEvaluationReport<'p, 'r>, ExpressionConsumerError> {
        let mut budget = Budget::new(self.limits, cancellation);
        budget.text(context.adapter_revision)?;
        budget.text(context.target_revision)?;
        budget.text(context.input_revision)?;
        let request = self.request(request, cancellation)?;
        validate_evaluation_bindings(self.project.model(), &request, cancellation)?;
        let entry = self
            .project
            .ir()
            .declarations
            .binary_search_by(|d| d.identity.cmp(request.selected))
            .ok()
            .map(|i| &self.project.ir().declarations[i].identity)
            .ok_or(ConsumerError::PrivateTarget)?;
        let outcome = evaluate(self.project.model(), &request, cancellation);
        budget.work(0)?;
        Ok(ExpressionEvaluationReport {
            project_identity: self.identity,
            entry,
            context,
            outcome,
        })
    }
    /// Prevents caller policy overrides and intersects every independent request control.
    fn request<'r>(
        &'r self,
        request: &'r EvaluationRequest<'r>,
        cancellation: &CancellationToken,
    ) -> Result<EvaluationRequest<'r>, ExpressionConsumerError> {
        let mut budget = Budget::new(self.limits, cancellation);
        if request.associations.len() as u64 > self.limits.closure_edges {
            return Err(ConsumerError::Limit.into());
        }
        for association in request.associations {
            budget.symbol(association.target)?;
            public_scope_type(
                self.project.model().scope(),
                association.result_type,
                &mut budget,
            )?;
            budget.work(self.policy.associations.len() as u64 + 1)?;
            if !self.policy.associations.contains(association) {
                return Err(ConsumerError::InvalidInput.into());
            }
        }
        let limits = neutral_ir::expression::profile::ExpressionLimits {
            bindings: request.limits.bindings.min(self.limits.candidates),
            dependency_edges: request
                .limits
                .dependency_edges
                .min(self.limits.closure_edges),
            work: request.limits.work.min(self.limits.work),
            retained_bytes: request.limits.retained_bytes.min(self.limits.bytes),
            output_bytes: request.limits.output_bytes.min(self.limits.output_bytes),
            diagnostics: request.limits.diagnostics.min(self.limits.diagnostics),
            ..request.limits
        };
        Ok(EvaluationRequest {
            selected: request.selected,
            profiles: request.profiles,
            bindings: request.bindings,
            associations: self.policy.associations,
            limits,
        })
    }
}
