// SPDX-License-Identifier: Apache-2.0

//! Shared nominal contracts, defaults and restrictions used by `/3` expressions.

mod syntax;
use syntax::{parse_name, parse_reference, split};
mod companions;
#[path = "../expressions/mod.rs"]
mod expressions;
mod resolver;
pub use expressions::cache::{ExpressionCompilationCache, ProjectCacheLimits, ProjectCacheStats};
pub use expressions::{
    ExpressionCompileFailure, compile_expression_project, format_expression_source,
};
#[path = "composition_codes.rs"]
mod codes;
#[path = "composition_graph.rs"]
mod graph;

use super::{TypeExpr, parse_complete_type_fallible};
use crate::CapturedCompositionProject;
use crate::frontend::{Token, TokenKind, lexer};
use neutral_core::allocation::RetainCapacity;
use neutral_core::allocation::{Shared as Arc, TryClone, boxed, copy_slice, text};
use neutral_core::ordered::{OrderedMap as BTreeMap, OrderedSet as BTreeSet};
use neutral_core::{
    ByteSpan, CancellationToken, SemanticDigest, SourceLocation, profile::V1_SOURCE_PROFILE,
};
use neutral_ir::composition::project::CompositionSignature as S;
use neutral_ir::{
    ExactNumber, LogicalModuleIdentity, ModuleSymbolIdentity,
    composition::{
        BindingValue as V, ClosedValue, CompositionAlternative, CompositionBody as B,
        CompositionDefinition, CompositionField, CompositionValue, FieldPresence,
        FieldRestrictions, SourceCompositionDefinition, profile,
        project::{CompositionOrigin, CompositionResourceFacts},
    },
    language::{is_protected_name, is_snake_name, is_upper_name},
    project::{
        PROJECT_MAX_DEPTH, ProjectLimits, ProjectProvenance, ProjectResourceFacts, ProjectSourceMap,
    },
    project_identity::IdentityLimits,
    project_interface::{ProjectPublicEdgeKind as Edge, ProjectPublicType as T},
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
struct Root {
    /// Stable resolved owner.
    owner: ModuleSymbolIdentity,
    /// Explicit source visibility.
    public: bool,
    /// Complete original declaration span.
    location: SourceLocation,
    /// Resolved binding type, absent for nominal declarations.
    ty: Option<TypeExpr>,
    /// Record fields or variant alternatives in source order.
    fields: Vec<(String, TypeExpr)>,
    /// Initializer/member tokens used by shared defaults.
    tokens: Vec<Token>,
    /// Nominal variant rather than record.
    variant: bool,
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
struct Resolver<'a> {
    /// Explicit captured bytes and composition-context policies.
    captured: &'a CapturedCompositionProject,
    /// Independently checked explicit import aliases.
    graph: graph::Graph,
    /// Parsed declarations needed for nominal lookup and expression resolution.
    roots: BTreeMap<ModuleSymbolIdentity, Root>,
    /// Exact resolved binding signatures.
    types: BTreeMap<ModuleSymbolIdentity, T>,
    /// Complete source contract bodies including materialized defaults.
    definitions: Vec<SourceCompositionDefinition>,
    /// Charged resolver work.
    work: u64,
    /// Explicit cooperative cancellation.
    cancel: &'a CancellationToken,
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
