// SPDX-License-Identifier: Apache-2.0

//! Shared nominal contracts, defaults and restrictions used by `/3` expressions.

pub(super) mod syntax;
use syntax::{parse_name, parse_reference, split};
pub(super) mod codes;
pub(super) mod companions;
pub(super) mod graph;
mod resolver;

use super::{TypeExpr, parse_complete_type_fallible};
use crate::CapturedCompositionProject;
use crate::frontend::{Token, TokenKind};
use neutral_core::allocation::RetainCapacity;
use neutral_core::allocation::{TryClone, boxed, text};
use neutral_core::ordered::OrderedMap as BTreeMap;
use neutral_core::{ByteSpan, CancellationToken, SourceLocation, profile::V1_SOURCE_PROFILE};
use neutral_ir::{
    ExactNumber, LogicalModuleIdentity, ModuleSymbolIdentity,
    composition::{
        BindingValue as V, ClosedValue, CompositionAlternative, CompositionBody as B,
        CompositionDefinition, CompositionField, CompositionValue, FieldPresence,
        FieldRestrictions, SourceCompositionDefinition, profile,
    },
    language::{is_protected_name, is_snake_name, is_upper_name},
    project::PROJECT_MAX_DEPTH,
    project_interface::ProjectPublicType as T,
};
use neutral_vocabulary::composition::CompositionError;

/// Private error envelope shared by current nominal/default resolution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct SemanticFailure {
    /// Stable safe classification.
    pub(super) code: &'static str,
    /// Original-byte occurrence when known.
    pub(super) location: Option<SourceLocation>,
}
/// Request-local shared error shorthand.
type E = SemanticFailure;

/// Parsed declaration retained only while `/3` resolves shared nominal contracts.
#[derive(Clone)]
pub(super) struct Root {
    /// Stable resolved owner.
    pub(super) owner: ModuleSymbolIdentity,
    /// Explicit source visibility.
    pub(super) public: bool,
    /// Complete original declaration span.
    pub(super) location: SourceLocation,
    /// Resolved binding type, absent for nominal declarations.
    pub(super) ty: Option<TypeExpr>,
    /// Record fields or variant alternatives in source order.
    pub(super) fields: Vec<(String, TypeExpr)>,
    /// Initializer/member tokens used by shared defaults.
    pub(super) tokens: Vec<Token>,
    /// Nominal variant rather than record.
    pub(super) variant: bool,
}

impl TryClone for Root {
    /// Copies syntax into a current request without infallible recursive clones.
    fn try_clone(&self) -> Result<Self, neutral_core::allocation::AllocationError> {
        Ok(Self {
            owner: self.owner.try_clone()?,
            public: self.public,
            location: self.location,
            ty: self.ty.try_clone()?,
            fields: self.fields.try_clone()?,
            tokens: self.tokens.try_clone()?,
            variant: self.variant,
        })
    }
}

/// Current `/3` request's shared nominal/default resolver state.
pub(super) struct Resolver<'a> {
    /// Explicit captured bytes and composition-context policies.
    pub(super) captured: &'a CapturedCompositionProject,
    /// Independently checked explicit import aliases.
    pub(super) graph: graph::Graph,
    /// Parsed declarations needed for nominal lookup and expression resolution.
    pub(super) roots: BTreeMap<ModuleSymbolIdentity, Root>,
    /// Exact resolved binding signatures.
    pub(super) types: BTreeMap<ModuleSymbolIdentity, T>,
    /// Complete source contract bodies including materialized defaults.
    pub(super) definitions: Vec<SourceCompositionDefinition>,
    /// Charged resolver work.
    pub(super) work: u64,
    /// Explicit cooperative cancellation.
    pub(super) cancel: &'a CancellationToken,
}

/// Maps shared catalogue validation failures to the current expression envelope.
fn contract(e: &CompositionError, location: Option<SourceLocation>) -> E {
    fail(
        match e {
            CompositionError::Cancelled => codes::CANCELLED,
            CompositionError::Limit
            | CompositionError::Allocation
            | CompositionError::InvalidLimits => codes::LIMIT,
            CompositionError::PrivateType => codes::PRIVATE_EXPOSURE,
            CompositionError::EmbeddedCycle => codes::EMBEDDED_CYCLE,
            CompositionError::InvalidRestrictions | CompositionError::DuplicateChoice => {
                codes::RESTRICTION_VIOLATION
            }
            _ => codes::INCOMPATIBLE_VALUE,
        },
        location,
    )
}

/// Maps fallible retention failures to the frozen bounded-resource source envelope.
fn retain<T>(result: Result<T, neutral_core::allocation::AllocationError>) -> Result<T, E> {
    result.map_err(|_| fail(codes::LIMIT, None))
}

/// Reserves unpublished vector growth before insertion, preserving existing state on failure.
fn reserve<T>(values: &mut Vec<T>, additional: usize) -> Result<(), E> {
    values
        .try_retain(additional)
        .map_err(|_| fail(codes::LIMIT, None))
}

/// Creates one non-authoritative original-byte diagnostic.
fn fail(code: &'static str, location: Option<SourceLocation>) -> E {
    E { code, location }
}
