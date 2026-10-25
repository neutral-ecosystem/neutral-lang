// SPDX-License-Identifier: Apache-2.0

//! Independent complete expression artifacts, distinct from model typing and effectful execution.
//!
//! [`ValidatedExpressionProject::from_ir`] checks semantics, resource facts,
//! original-byte companions and the public interface digest before publishing
//! authority. Entry selection and public views operate only on that authority.
//! Views retain the interpretation closure but redact private source identities;
//! guarded dependencies describe possible demand without selecting branches,
//! acquiring bindings or scheduling operations.

mod companions;
mod dependencies;
mod entries;
mod view;
pub use dependencies::{
    DependencyGuard, ExpressionDependency, ExpressionInspectionLimits, GuardCondition,
    GuardedDependencies,
};
pub use entries::{ExpressionEntryError, SelectedExpressionEntries};
use neutral_core::{CancellationToken, allocation::Shared};
use neutral_ir::expression::project::ExpressionProjectIr;
pub use neutral_ir::expression::templates::ExpansionLimits;
pub use neutral_ir::expression::{profile, project::ExpressionResourceFacts};
pub use neutral_ir::project_identity::{
    ExpressionInterfaceIdentity, ExpressionLogicalIdentity, canonical_expression_project,
    expression_interface,
};
use neutral_semantics::{SemanticLimits, ValidatedExpressionModel, validate_expression_model};
pub use view::{
    ExpressionPublicAttribution, ExpressionPublicExpansion, ExpressionPublicOrigin,
    ExpressionStructuralEdge, ExpressionView, ExpressionViewLimits, ExpressionViewRequest,
    ExpressionVocabularyView,
};

/// Closed whole-artifact failures; no private names, source text or host paths escape.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpressionReadError {
    /// Unsupported exact schema or feature/profile tuple.
    Schema,
    /// Independent resource or fallible retention bound.
    Limit,
    /// Invalid complete retained semantics.
    Semantic,
    /// Independently recomputed public-interface digest differs.
    Interface,
    /// Missing, inconsistent or forged source/origin/dependency companions.
    Companion,
    /// Claimed retained accounting differs from inspection.
    Resources,
    /// Cooperative cancellation prevents complete publication.
    Cancelled,
}
impl ExpressionReadError {
    /// Returns the separately frozen complete expression result selector.
    #[must_use]
    pub const fn schema(self) -> &'static str {
        profile::PROJECT_RESULT_SCHEMA
    }
}

/// Complete independently checked immutable artifact authority, never a raw-model downcast.
#[derive(Clone)]
pub struct ValidatedExpressionProject {
    /// Immutable raw data retained by a fallible shared owner.
    ir: Shared<ExpressionProjectIr>,
    /// Independently checked all-branch model and derived obligations.
    model: Shared<ValidatedExpressionModel>,
}
impl std::fmt::Debug for ValidatedExpressionProject {
    /// Counts only, never private identity/value/source data.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidatedExpressionProject")
            .field("declarations", &self.ir.declarations.len())
            .finish_non_exhaustive()
    }
}
impl ValidatedExpressionProject {
    /// Independently checks all retained sections before exposing any complete authority.
    /// # Errors
    /// Rejects unsupported tuples, semantic/companion/fact/digest defects and resource/cancellation failures.
    pub fn from_ir(
        ir: Shared<ExpressionProjectIr>,
        caller: SemanticLimits,
        cancellation: &CancellationToken,
    ) -> Result<Self, ExpressionReadError> {
        let model =
            validate_expression_model(ir.clone(), caller, cancellation).map_err(|e| match e {
                neutral_semantics::ExpressionValidationError::Unsupported => {
                    ExpressionReadError::Schema
                }
                neutral_semantics::ExpressionValidationError::Resources => {
                    ExpressionReadError::Resources
                }
                neutral_semantics::ExpressionValidationError::Limit => ExpressionReadError::Limit,
                neutral_semantics::ExpressionValidationError::Cancelled => {
                    ExpressionReadError::Cancelled
                }
                _ => ExpressionReadError::Semantic,
            })?;
        companions::validate(&model, cancellation)?;
        let limits = model.limits();
        let interface = neutral_ir::project_identity::expression_interface(
            &ir,
            neutral_ir::project_identity::IdentityLimits {
                bytes: limits
                    .project
                    .artifact_bytes
                    .min(limits.expression.retained_bytes),
                nodes: limits.project.nodes,
            },
            cancellation,
        )
        .map_err(|e| match e {
            neutral_ir::project_identity::IdentityError::Cancelled => {
                ExpressionReadError::Cancelled
            }
            neutral_ir::project_identity::IdentityError::Limit => ExpressionReadError::Limit,
            neutral_ir::project_identity::IdentityError::InvalidInput => {
                ExpressionReadError::Semantic
            }
        })?;
        if interface.identity().as_bytes() != ir.interface_digest.as_bytes() {
            return Err(ExpressionReadError::Interface);
        }
        let model = Shared::try_new(model).map_err(|_| ExpressionReadError::Limit)?;
        if cancellation.is_cancelled() {
            return Err(ExpressionReadError::Cancelled);
        }
        Ok(Self { ir, model })
    }
    /// Borrows complete validated data; public projections must redact private content.
    #[must_use]
    pub fn ir(&self) -> &ExpressionProjectIr {
        &self.ir
    }
    /// Borrows the complete immutable shared owner without retaining another copy.
    #[must_use]
    pub const fn complete_ir(&self) -> &Shared<ExpressionProjectIr> {
        &self.ir
    }
    /// Borrows independently derived model types and obligations.
    #[must_use]
    pub fn model(&self) -> &ValidatedExpressionModel {
        &self.model
    }
}
