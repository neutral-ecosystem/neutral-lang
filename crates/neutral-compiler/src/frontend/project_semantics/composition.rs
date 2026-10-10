// SPDX-License-Identifier: Apache-2.0

//! Coordinate compilation of captured projects with composition contracts.
//!
//! This entry point owns request state, phase checkpoints and fail-closed
//! publication. Syntax extraction, bounded resolution and IR/companion assembly
//! live in private child modules; cache, graph and attribution helpers retain
//! separate responsibilities. The capture/2 path does not change older entry points.

mod syntax;
use syntax::{parse, parse_name, parse_reference, split};
mod assembly;
use assembly::assemble;
mod resolver;
use resolver::type_edges;

use super::{
    Token, TokenKind, TypeExpr, is_requirement_or_import, lexer, parse_complete_type_fallible,
};
use crate::CapturedCompositionProject;
use neutral_core::allocation::RetainCapacity;
use neutral_core::allocation::{Shared as Arc, TryClone, boxed, copy_slice, text};
use neutral_core::ordered::{OrderedMap as BTreeMap, OrderedSet as BTreeSet};
use neutral_core::{
    ByteSpan, CancellationToken, SemanticDigest, SourceLocation, profile::V1_SOURCE_PROFILE,
};
use neutral_ir::{
    ExactNumber, LogicalModuleIdentity, ModuleSymbolIdentity,
    composition::{
        BindingValue as V, ClosedValue, CompositionAlternative, CompositionBinding,
        CompositionBody as B, CompositionDefinition, CompositionField, CompositionValue,
        FieldPresence, FieldRestrictions, SourceCompositionDefinition, profile,
        project::{
            CompositionDeclaration, CompositionOrigin, CompositionProjectIr as Ir,
            CompositionResourceFacts, CompositionSignature as S, inspect_composition_ir,
        },
    },
    language::{is_protected_name, is_snake_name, is_upper_name},
    project::{
        PROJECT_MAX_DEPTH, ProjectLimits, ProjectModule, ProjectProvenance, ProjectResourceFacts,
        ProjectSource, ProjectSourceMap, ProjectVocabularySource,
    },
    project_identity::{IdentityLimits, composition_interface},
    project_interface::{ProjectPublicEdgeKind as Edge, ProjectPublicType as T},
};
use neutral_vocabulary::composition::{
    CompositionError, ValidatedCompositionScope, validate_composition_bindings,
    validate_composition_scope,
};

/// Safe successor compilation failure; no variant publishes partial logical output.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionCompileFailure {
    /// Frozen source diagnostic classification.
    pub code: &'static str,
    /// Checked original-byte occurrence when available.
    pub location: Option<SourceLocation>,
}
impl CompositionCompileFailure {
    /// Returns the explicitly selected successor whole-request envelope.
    #[must_use]
    pub const fn schema(&self) -> &'static str {
        profile::PROJECT_RESULT_SCHEMA
    }
}
/// Short request-local diagnostic result name.
type E = CompositionCompileFailure;
/// Borrowed source name; resolving a qualifier never allocates temporary spellings.
struct BorrowedOccurrence<'a> {
    /// Source-local qualifier, if present.
    alias: Option<&'a str>,
    /// Exact declaration spelling.
    name: &'a str,
    /// Original occurrence for checked diagnostics.
    span: ByteSpan,
}
/// Checked occurrence locations; raw source stays private to the compiler.
#[path = "composition_attribution.rs"]
mod attribution;
/// Exact-source syntax reuse; every request still rebuilds complete semantics and companions.
#[path = "composition_cache.rs"]
mod cache;
/// Fallibly retained successor topology, independent of old-profile public shared-owner types.
#[path = "composition_graph.rs"]
mod graph;
pub use cache::CompositionCompilationCache;
/// Source diagnostic spellings; package releases never select them.
#[path = "composition_codes.rs"]
pub mod codes;

/// Parsed declaration; initializer/member tokens are private and reused within this request only.
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
    /// Initializer or field/default body tokens.
    tokens: Vec<Token>,
    /// Nominal variant rather than record.
    variant: bool,
}

impl neutral_core::allocation::TryClone for Root {
    /// Copies preflighted syntax into a new cache generation without infallible recursive clones.
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

/// Request-local resolver; no filesystem, URL acquisition, ambient environment or persistent cache.
struct Resolver<'a> {
    /// Complete explicit captured closure.
    captured: &'a CapturedCompositionProject,
    /// Independently graph-checked explicit imports/aliases.
    graph: graph::Graph,
    /// Canonical parsed declarations, including private/disconnected units.
    roots: BTreeMap<ModuleSymbolIdentity, Root>,
    /// Exact resolved binding signatures.
    types: BTreeMap<ModuleSymbolIdentity, T>,
    /// Complete source contract bodies, before or after default materialization.
    definitions: Vec<SourceCompositionDefinition>,
    /// Request-only memoized immutable initializer values.
    values: BTreeMap<ModuleSymbolIdentity, V>,
    /// Active ordinary reuse chain; references never enter it.
    active: BTreeSet<ModuleSymbolIdentity>,
    /// Source-accounted ordinary reuse occurrences.
    provenance: Vec<ProjectProvenance>,
    /// Charged source/resolution traversal work.
    work: u64,
    /// Explicit cooperative cancellation.
    cancel: &'a CancellationToken,
}

/// Compiles every capture/2 source under the shared record/variant/default/constraint model.
///
/// The caller must independently open the returned producer data through the
/// successor reader before encoding it. No root selection or host acquisition is
/// part of compilation, and no package release or source header selects this API.
///
/// # Errors
/// Rejects graph, grammar, visibility, cycles, type/default/value, bounds or
/// cancellation failures atomically without returning any successful prefix.
pub fn compile_composition_project(
    captured: &CapturedCompositionProject,
    cancel: &CancellationToken,
) -> Result<Arc<Ir>, E> {
    compile_with_parser(captured, cancel, &mut |source| {
        parse(source, captured, cancel)
    })
}

/// Shares the complete clean pipeline with cache-backed private parsing, never cached semantic authority.
fn compile_with_parser(
    captured: &CapturedCompositionProject,
    cancel: &CancellationToken,
    parser: &mut impl FnMut(&crate::CapturedProjectSource) -> Result<Vec<Root>, E>,
) -> Result<Arc<Ir>, E> {
    compile_observed(captured, cancel, parser, &mut |_| Ok(()))
}

/// Private phase boundaries for cooperative publication checks and deterministic fault tests.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Phase {
    /// Complete import/SCC validation before private parsing.
    Graph,
    /// Real or exact-cache source parser invocation.
    Parse,
    /// Resolve source signatures and nominal owners.
    Signatures,
    /// Check embedded/public contract closure before defaults.
    InitialScope,
    /// Materialize source-owned closed defaults.
    Defaults,
    /// Recheck finalized source and vocabulary contracts together.
    FinalScope,
    /// Resolve complete immutable binding initializers.
    Values,
    /// Validate materialized values, restrictions and typed references.
    Bindings,
    /// Construct complete IR and independently retained companions.
    Assemble,
    /// Final atomic publication, also guarding pending cache generations.
    Publish,
}

/// Runs a private observer and checks cancellation on both sides of a phase boundary.
fn checkpoint(
    phase: Phase,
    cancel: &CancellationToken,
    observer: &mut impl FnMut(Phase) -> Result<(), E>,
) -> Result<(), E> {
    if cancel.is_cancelled() {
        return Err(fail(codes::CANCELLED, None));
    }
    observer(phase)?;
    if cancel.is_cancelled() {
        return Err(fail(codes::CANCELLED, None));
    }
    Ok(())
}

/// Executes every complete-project phase with no public hooks or partial-output publication.
fn compile_observed(
    captured: &CapturedCompositionProject,
    cancel: &CancellationToken,
    parser: &mut impl FnMut(&crate::CapturedProjectSource) -> Result<Vec<Root>, E>,
    observer: &mut impl FnMut(Phase) -> Result<(), E>,
) -> Result<Arc<Ir>, E> {
    checkpoint(Phase::Graph, cancel, observer)?;
    let graph = graph::build(captured, cancel)?;
    let mut resolver = Resolver {
        captured,
        graph,
        roots: BTreeMap::new(),
        types: BTreeMap::new(),
        definitions: Vec::new(),
        values: BTreeMap::new(),
        active: BTreeSet::new(),
        provenance: Vec::new(),
        work: 0,
        cancel,
    };
    resolver.step(1, None)?;
    checkpoint(Phase::Parse, cancel, observer)?;
    for source in captured.sources() {
        resolver.step(source.bytes().len() as u64, None)?;
        for root in parser(source)? {
            if resolver.roots.len() as u64 >= captured.limits().values().declarations {
                return Err(fail(codes::LIMIT, Some(root.location)));
            }
            resolver.step(resolver.roots.len() as u64 + 1, Some(root.location))?;
            let location = root.location;
            if resolver
                .roots
                .insert(retain(root.owner.try_clone())?, root)
                .map_err(|_| fail(codes::LIMIT, Some(location)))?
                .is_some()
            {
                return Err(fail(codes::DUPLICATE_MEMBER, Some(location)));
            }
        }
    }
    let roots = resolver.retained_roots()?;
    checkpoint(Phase::Signatures, cancel, observer)?;
    resolver.signatures(&roots)?;
    // Visibility and embedded closure precede all source default/value interpretation.
    checkpoint(Phase::InitialScope, cancel, observer)?;
    validate_composition_scope(
        captured.catalogue(),
        retain(resolver.definitions.try_clone())?,
        captured.composition_limits(),
        cancel,
    )
    .map_err(|e| contract(&e, None))?;
    checkpoint(Phase::Defaults, cancel, observer)?;
    resolver.source_defaults(&roots)?;
    checkpoint(Phase::FinalScope, cancel, observer)?;
    let scope = Arc::try_new(
        validate_composition_scope(
            captured.catalogue(),
            retain(resolver.definitions.try_clone())?,
            captured.composition_limits(),
            cancel,
        )
        .map_err(|e| contract(&e, None))?,
    )
    .map_err(|_| fail(codes::LIMIT, None))?;
    resolver.definitions = retain(copy_slice(scope.sources()))?;
    checkpoint(Phase::Values, cancel, observer)?;
    let mut bindings = Vec::new();
    for root in roots.iter().filter(|r| r.ty.is_some()) {
        let value = resolver.binding(&root.owner, 0)?;
        resolver.step(1, Some(root.location))?;
        reserve(&mut bindings, 1)?;
        bindings.push(CompositionBinding {
            owner: retain(root.owner.try_clone())?,
            public: root.public,
            ty: retain(resolver.types[&root.owner].try_clone())?,
            value,
        });
    }
    checkpoint(Phase::Bindings, cancel, observer)?;
    let bindings = validate_composition_bindings(
        scope.clone(),
        &bindings,
        captured.composition_limits(),
        cancel,
    )
    .map_err(|e| contract(&e, None))?;
    checkpoint(Phase::Assemble, cancel, observer)?;
    let ir = assemble(&mut resolver, &scope, &bindings)?;
    checkpoint(Phase::Publish, cancel, observer)?;
    Ok(ir)
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
/// Maps shared semantic failures without exposing private contract text.
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
/// Creates one non-authoritative original-byte diagnostic.
fn fail(code: &'static str, location: Option<SourceLocation>) -> E {
    E { code, location }
}
