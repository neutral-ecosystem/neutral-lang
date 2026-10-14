// SPDX-License-Identifier: Apache-2.0

//! Explicit successor source semantics, sharing resolved nominal contracts and pure evaluation.
//!
//! Parsing retains captured byte spans; resolution checks names and types in every
//! branch. Lowering folds only static paths through the shared pure evaluator and
//! preserves deferred trees rather than executing operations. Expansion shares the
//! same request budget, so nested templates cannot reset work or retention limits.
//! Successful raw assembly still requires independent reader validation; none of
//! these private phases activates a profile or acquires external inputs.

use super::{
    Arc, B, BTreeMap, BTreeSet, ByteSpan, CancellationToken, ClosedValue, CompositionOrigin,
    CompositionResourceFacts, Edge, ExactNumber, FieldPresence, IdentityLimits,
    LogicalModuleIdentity, ModuleSymbolIdentity, ProjectLimits, ProjectProvenance,
    ProjectResourceFacts, ProjectSourceMap, RetainCapacity, Root, S, SemanticDigest,
    SourceLocation, T, Token, TokenKind, TryClone, TypeExpr, V, V1_SOURCE_PROFILE, boxed, codes,
    copy_slice, is_protected_name, is_snake_name, is_upper_name, lexer,
    parse_complete_type_fallible, text, validate_composition_scope,
};
use crate::CapturedExpressionProject;
use crate::frontend::expression_codes as ec;
use neutral_ir::expression::{
    BinaryOperator as Op, Expression, ExpressionBody, ExpressionNode as N, MatchArm,
    SelectorSegment, UnaryOperator,
    attribution::{ExpansionAttribution, ExpressionAttribution},
    profile::{self as expression_profile, ExpressionLimits},
    project::{ExpressionDeclaration, ExpressionProjectIr, ExpressionResourceFacts},
    templates::{ExpansionLimits, ParameterPhase, PublicTemplate, TemplateParameter},
};
use neutral_semantics::{
    NormalizedBinding, SemanticLimits, derive_expression_resources, normalize_binding,
    validate_expression_model,
};
#[path = "../expansion/mod.rs"]
/// Definition-scoped templates and finite matrices.
mod expansion;
/// Complete producer assembly and shared static evaluation.
mod lowering;
/// Contextual successor lexical/source syntax.
mod parser;
/// Exact all-branch expression typing and stable names.
mod resolution;
use parser::{BindingSyntax, Lex, MatrixSyntax, TemplateSyntax};

/// Safe atomic successor compilation failure with checked original-byte occurrence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpressionCompileFailure {
    /// Frozen safe source-phase classification.
    pub code: &'static str,
    /// Original captured byte occurrence when known.
    pub location: Option<SourceLocation>,
}
impl ExpressionCompileFailure {
    /// Returns the explicitly selected successor whole-request schema.
    #[must_use]
    pub const fn schema(&self) -> &'static str {
        expression_profile::PROJECT_RESULT_SCHEMA
    }
}
/// Request-local safe error shorthand.
type E = ExpressionCompileFailure;
/// Constructs an atomic error without retaining raw supplied text.
fn error(code: &'static str, location: Option<SourceLocation>) -> E {
    E { code, location }
}
/// Preserves graph/nominal boundary failure classes under the successor source registry.
fn composition_error(e: super::E) -> E {
    error(
        match e.code {
            codes::CANCELLED => ec::EXP_CANCELLED,
            codes::LIMIT => ec::EXP_LIMIT,
            codes::PRIVATE_EXPOSURE => ec::EXP_NAME,
            _ => ec::EXP_SYNTAX,
        },
        e.location,
    )
}
/// Maps every fallible allocation checkpoint to safe request failure.
fn alloc<T>(value: Result<T, neutral_core::allocation::AllocationError>) -> Result<T, E> {
    value.map_err(|_| error(ec::EXP_LIMIT, None))
}
/// Reserves vector storage before insertion.
fn grow<T>(values: &mut Vec<T>, count: usize) -> Result<(), E> {
    alloc(values.try_retain(count))
}
/// Builds a stable owner from already checked logical names.
fn symbol(module: &str, name: &str) -> Result<ModuleSymbolIdentity, E> {
    Ok(ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(alloc(text(V1_SOURCE_PROFILE))?, alloc(text(module))?),
        alloc(text(name))?,
    ))
}

/// One request shares every aggregate counter; nested applications cannot reset controls.
struct Budget<'a> {
    /// Effective independent expression ceilings.
    expression: ExpressionLimits,
    /// Effective independent expansion ceilings.
    expansion: ExpansionLimits,
    /// Cooperative request signal, never global state.
    cancel: &'a CancellationToken,
    /// Aggregate charged source/resolution/copy work.
    work: u64,
    /// Aggregate retained private syntax/tree storage.
    bytes: u64,
    /// Complete visited source expression occurrences.
    nodes: u64,
    /// Applications including nested definition typing.
    applications: u64,
    /// Full unfiltered Cartesian tuple sum.
    tuples: u64,
    /// Retained generated bindings.
    expanded: u64,
    /// Aggregate generated expression occurrences.
    expanded_nodes: u64,
    /// Expansion-only work, including filtered tuples.
    expansion_work: u64,
    /// Expansion-only model retention.
    expansion_bytes: u64,
}
impl Budget<'_> {
    /// Charges before traversal and checks cancellation.
    fn charge(&mut self, count: u64) -> Result<(), E> {
        if self.cancel.is_cancelled() {
            return Err(error(ec::EXP_CANCELLED, None));
        }
        self.work = self
            .work
            .checked_add(count)
            .ok_or_else(|| error(ec::EXP_LIMIT, None))?;
        if self.work > self.expression.work {
            return Err(error(ec::EXP_LIMIT, None));
        }
        Ok(())
    }
    /// Charges retained storage independently from decoded result output bytes.
    fn bytes(&mut self, count: u64) -> Result<(), E> {
        self.charge(
            count
                .checked_add(1)
                .ok_or_else(|| error(ec::EXP_LIMIT, None))?,
        )?;
        self.bytes = self
            .bytes
            .checked_add(count)
            .ok_or_else(|| error(ec::EXP_LIMIT, None))?;
        if self.bytes > self.expression.retained_bytes {
            return Err(error(ec::EXP_LIMIT, None));
        }
        Ok(())
    }
    /// Checks model byte multiplication before reserving typed storage.
    fn retain<T>(&mut self, count: usize) -> Result<(), E> {
        self.bytes(
            (std::mem::size_of::<T>() as u64)
                .checked_mul(count as u64)
                .ok_or_else(|| error(ec::EXP_LIMIT, None))?,
        )
    }
    /// Checks complete source tree occurrences before recursive descent.
    fn node(&mut self, depth: u64) -> Result<(), E> {
        self.charge(1)?;
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or_else(|| error(ec::EXP_LIMIT, None))?;
        if self.nodes > self.expression.nodes || depth > self.expression.syntax_depth {
            return Err(error(ec::EXP_LIMIT, None));
        }
        self.retain::<Expression>(1)
    }
    /// Charges finite expansion work and retention without borrowing expression counters.
    fn expansion(&mut self, work: u64, bytes: u64) -> Result<(), E> {
        self.charge(work)?;
        self.expansion_work = self
            .expansion_work
            .checked_add(work)
            .ok_or_else(|| error(ec::TPL_LIMIT, None))?;
        self.expansion_bytes = self
            .expansion_bytes
            .checked_add(bytes)
            .ok_or_else(|| error(ec::TPL_LIMIT, None))?;
        if self.expansion_work > self.expansion.work
            || self.expansion_bytes > self.expansion.retained_bytes
        {
            return Err(error(ec::TPL_LIMIT, None));
        }
        Ok(())
    }
}
/// Lexical frames preserve binder identity; values carry already resolved caller identities.
enum Local<'a> {
    /// A real retained template/match lexical frame slot.
    Bound(&'a str, T, ParameterPhase),
    /// Already resolved substitution without a new retained lexical frame.
    Value(&'a str, Expression),
}
/// Ordered lexical frame; parameter slots retain declared order.
type Frame<'a> = Vec<Local<'a>>;
/// Complete request-local compilation state; none of it survives failed publication.
struct State<'a, 'c> {
    /// Exact captured closure and independent policy.
    captured: &'a CapturedExpressionProject,
    /// Existing graph/name/type/default resolver.
    resolver: super::Resolver<'a>,
    /// Ordinary input/initializer source syntax.
    bindings: BTreeMap<ModuleSymbolIdentity, BindingSyntax<'a>>,
    /// Definition-scoped original template syntax.
    templates: BTreeMap<ModuleSymbolIdentity, TemplateSyntax<'a>>,
    /// Complete typed non-callable template contracts.
    typed_templates: BTreeMap<ModuleSymbolIdentity, PublicTemplate>,
    /// Pending source matrices before static shape evaluation.
    matrices: Vec<MatrixSyntax<'a>>,
    /// Request-only complete raw binding bodies.
    bodies: BTreeMap<ModuleSymbolIdentity, ExpressionBody>,
    /// Ordinary reuse cycle detection stack.
    active: BTreeSet<ModuleSymbolIdentity>,
    /// All-branch template application cycle detection stack.
    active_templates: BTreeSet<ModuleSymbolIdentity>,
    /// Checked real application/definition/argument/axis lineage.
    expansions: Vec<ExpansionAttribution>,
    /// Static arguments checked after complete all-branch typing, including unused ones.
    pending_static: Vec<(Root, Expression)>,
    /// Outermost real binding owner for nested application lineage.
    application_root: Option<Root>,
    /// Shared aggregate independent work/storage controls.
    budget: Budget<'c>,
    /// Definition typing does not execute arithmetic in unused definitions.
    typing_template: bool,
}

/// Compiles exactly captured /3 bytes into complete raw producer data without acquiring inputs.
///
/// # Errors
/// Rejects graph, complete all-branch typing, cycles, expansion/shape, selected arithmetic,
/// independent bounds, allocation and cancellation without publishing a partial project.
pub fn compile_expression_project(
    captured: &CapturedExpressionProject,
    cancel: &CancellationToken,
) -> Result<Arc<ExpressionProjectIr>, E> {
    let mut state = initial_state(captured, cancel)?;
    for source in captured.sources() {
        parser::parse_source(source, &mut state)?;
    }
    let roots = state.resolver.retained_roots().map_err(composition_error)?;
    state
        .resolver
        .signatures(&roots)
        .map_err(composition_error)?;
    validate_composition_scope(
        captured.catalogue(),
        alloc(state.resolver.definitions.try_clone())?,
        captured.composition_limits(),
        cancel,
    )
    .map_err(|e| error(e.diagnostic_code(), None))?;
    state
        .resolver
        .source_defaults(&roots)
        .map_err(composition_error)?;
    let scope = validate_composition_scope(
        captured.catalogue(),
        alloc(state.resolver.definitions.try_clone())?,
        captured.composition_limits(),
        cancel,
    )
    .map_err(|e| error(e.diagnostic_code(), None))?;
    state.resolver.definitions = alloc(copy_slice(scope.sources()))?;
    let owners: Vec<_> = state
        .templates
        .keys()
        .map(|o| alloc(o.try_clone()))
        .collect::<Result<_, _>>()?;
    for owner in owners {
        state.template(&owner)?;
    }
    state.expand_matrices()?;
    let owners: Vec<_> = state
        .bindings
        .keys()
        .map(|o| alloc(o.try_clone()))
        .collect::<Result<_, _>>()?;
    for owner in &owners {
        state.binding(owner)?;
    }
    let ir = state.assemble(false, None)?;
    let model =
        validate_expression_model(alloc(Arc::try_new(ir))?, state.semantic_limits(), cancel)
            .map_err(|e| error(e.code(), None))?;
    for (root, argument) in std::mem::take(&mut state.pending_static) {
        if state.static_value(&root, &argument)?.is_none() {
            return Err(error(ec::TPL_TYPE, Some(root.location)));
        }
    }
    let mut normalized = BTreeMap::new();
    for owner in owners {
        state.budget.charge(1)?;
        let body = match normalize_binding(&model, &owner, state.budget.expression, cancel)
            .map_err(|e| {
                error(
                    e.code(),
                    state.resolver.roots.get(&owner).map(|r| r.location),
                )
            })? {
            NormalizedBinding::Value(v) => ExpressionBody::Value(v),
            NormalizedBinding::Residual(body) => body,
        };
        alloc(normalized.insert(owner, body))?;
    }
    state.bodies = normalized;
    let mut ir = state.assemble(false, None)?;
    ir.interface_digest = SemanticDigest::from_raw_bytes(
        neutral_ir::project_identity::expression_interface(
            &ir,
            IdentityLimits {
                bytes: neutral_ir::project_identity::MAX_TRANSCRIPT_BYTES,
                nodes: state.budget.expression.work,
            },
            cancel,
        )
        .map_err(|_| {
            error(
                if cancel.is_cancelled() {
                    ec::EXP_CANCELLED
                } else {
                    ec::EXP_LIMIT
                },
                None,
            )
        })?
        .identity()
        .as_bytes(),
    );
    let authority = alloc(Arc::try_new(ir))?;
    validate_expression_model(authority.clone(), state.semantic_limits(), cancel)
        .map_err(|e| error(e.code(), None))?;
    state.budget.charge(1)?;
    Ok(authority)
}

/// Creates isolated compilation state after checking the explicit retained module graph.
fn initial_state<'a>(
    captured: &'a CapturedExpressionProject,
    cancel: &'a CancellationToken,
) -> Result<State<'a, 'a>, E> {
    let graph =
        super::graph::build_expression(&captured.composition, cancel).map_err(composition_error)?;
    Ok(State {
        captured,
        resolver: super::Resolver {
            captured: &captured.composition,
            graph,
            roots: BTreeMap::new(),
            types: BTreeMap::new(),
            definitions: Vec::new(),
            values: BTreeMap::new(),
            active: BTreeSet::new(),
            provenance: Vec::new(),
            work: 0,
            cancel,
        },
        bindings: BTreeMap::new(),
        templates: BTreeMap::new(),
        typed_templates: BTreeMap::new(),
        matrices: Vec::new(),
        bodies: BTreeMap::new(),
        active: BTreeSet::new(),
        active_templates: BTreeSet::new(),
        expansions: Vec::new(),
        pending_static: Vec::new(),
        application_root: None,
        budget: Budget {
            expression: captured
                .expression_limits()
                .intersect(ExpressionLimits::hard()),
            expansion: captured
                .expansion_limits()
                .intersect(ExpansionLimits::hard()),
            cancel,
            work: 0,
            bytes: 0,
            nodes: 0,
            applications: 0,
            tuples: 0,
            expanded: 0,
            expanded_nodes: 0,
            expansion_work: 0,
            expansion_bytes: 0,
        },
        typing_template: false,
    })
}

impl State<'_, '_> {
    /// Checks all source/generated declarations share one collision namespace.
    fn check_name(&mut self, owner: &ModuleSymbolIdentity) -> Result<(), E> {
        self.budget
            .charge((self.resolver.roots.len() + self.templates.len()) as u64 + 1)?;
        if self.resolver.roots.contains_key(owner)
            || self.templates.contains_key(owner)
            || self.matrices.iter().any(|m| m.root.owner == *owner)
        {
            return Err(error(ec::TPL_COLLISION, None));
        }
        Ok(())
    }
    /// Retains one checked source root under complete declaration controls.
    fn insert_root(&mut self, root: Root) -> Result<(), E> {
        self.check_name(&root.owner)?;
        if self.resolver.roots.len() as u64
            >= self
                .captured
                .limits()
                .values()
                .declarations
                .min(self.budget.expression.bindings)
        {
            return Err(error(ec::EXP_LIMIT, Some(root.location)));
        }
        alloc(
            self.resolver
                .roots
                .insert(alloc(root.owner.try_clone())?, root),
        )?;
        Ok(())
    }
    /// Projects independent source/nominal/expression/expansion ceilings for shared validation.
    fn semantic_limits(&self) -> SemanticLimits {
        let cap = self.captured.limits().values();
        SemanticLimits {
            project: ProjectLimits {
                modules: cap.source_units,
                declarations: cap.declarations,
                import_edges: cap.import_edges,
                nodes: self.captured.composition_limits().value_nodes,
                text_bytes: cap
                    .total_source_bytes
                    .saturating_add(cap.total_vocabulary_bytes),
                artifact_bytes: cap.output_bytes,
            },
            composition: self.captured.composition_limits(),
            expression: self.budget.expression,
            expansion: self.budget.expansion,
        }
    }
    /// Preflights and charges every retained subtree copy.
    fn clone_expression(&mut self, e: &Expression) -> Result<Expression, E> {
        let cost = neutral_ir::expression::inspect_expression_retention(
            e,
            self.budget.expression,
            self.budget.cancel,
        )
        .map_err(|e| {
            error(
                match e {
                    neutral_ir::expression::ExpressionShapeError::Cancelled => ec::EXP_CANCELLED,
                    _ => ec::EXP_LIMIT,
                },
                None,
            )
        })?;
        self.budget.charge(cost.work)?;
        self.budget.bytes(cost.retained_bytes)?;
        e.try_clone_bounded(self.budget.expression, self.budget.cancel)
            .map_err(|_| error(ec::EXP_LIMIT, None))
    }
    /// Copies a body sum without fabricated input values or lossy tree conversion.
    fn copy_body(&mut self, body: &ExpressionBody) -> Result<ExpressionBody, E> {
        Ok(match body {
            ExpressionBody::Input => ExpressionBody::Input,
            ExpressionBody::Value(v) => ExpressionBody::Value(
                neutral_ir::expression::try_clone_value_bounded(
                    v,
                    self.budget.expression,
                    self.budget.cancel,
                )
                .map_err(|_| error(ec::EXP_LIMIT, None))?,
            ),
            ExpressionBody::Expression(e) => ExpressionBody::Expression(self.clone_expression(e)?),
        })
    }
}

#[cfg(test)]
#[path = "../../../tests/expression_source/mod.rs"]
/// Literal source and finite expansion regressions.
mod tests;
