// SPDX-License-Identifier: Apache-2.0

//! Bounded public closure retains validated authority privately and exposes only public indexes.

use super::{
    ExpressionInspectionLimits, ExpressionReadError as E, GuardedDependencies,
    ValidatedExpressionProject,
};
use neutral_core::{ByteSpan, CancellationToken, allocation::RetainCapacity};
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::{
        CompositionBody as B, CompositionDefinition, CompositionDependency, CompositionValue as V,
        ValueOriginKind, ValuePathSegment, project::CompositionSignature as S,
    },
    expression::{
        Expression, ExpressionBody, ExpressionNode as N,
        profile::{self, SemanticProfiles},
        project::ExpressionDeclaration,
        templates::PublicTemplate,
    },
    project_interface::{ProjectPublicEdgeKind, ProjectPublicType as T},
};
use neutral_semantics::{AvailabilityObligation, RestrictionObligation};

/// Exact post-compilation request; explicit empty roots select an empty view.
pub struct ExpressionViewRequest<'a> {
    /// Exact `/3` public view selector.
    pub schema: &'a str,
    /// Exact semantic profile/features, without wildcard or subset fallback.
    pub semantic_profiles: &'a SemanticProfiles,
    /// Explicit selected public declaration or public-template owners.
    pub roots: &'a [ModuleSymbolIdentity],
}

/// Independent positive public-projection controls.
#[derive(Clone, Copy, Debug)]
pub struct ExpressionViewLimits {
    /// Selected roots, independently bounded from closure size.
    pub roots: u64,
    /// All followed type, reference, input/output and bundle dependency occurrences.
    pub closure_edges: u64,
    /// Aggregate traversal, comparison and retained-index work.
    pub work: u64,
    /// Owned wrapper and index storage, excluding the privately shared validated artifact.
    pub output_bytes: u64,
}
impl Default for ExpressionViewLimits {
    /// Uses frozen hard controls; callers may reduce dimensions independently.
    fn default() -> Self {
        Self {
            roots: profile::MAX_ITEMS,
            closure_edges: profile::MAX_ITEMS,
            work: profile::MAX_WORK,
            output_bytes: profile::MAX_MODEL_BYTES,
        }
    }
}

/// Internal indexes, never a second unvalidated artifact or a public raw-data handle.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Owner {
    /// Ordinary public declaration index.
    Declaration(usize),
    /// Public template index.
    Template(usize),
    /// Public vocabulary definition indexes.
    Definition(usize, usize),
    /// Exact vocabulary dependency metadata index.
    Bundle(usize),
}

/// Immutable public interpretation projection with no complete-project accessor.
pub struct ExpressionView {
    /// Shared complete authority remains private; debug output cannot reveal it.
    project: ValidatedExpressionProject,
    /// Canonically normalized exact selected roots.
    roots: Vec<Owner>,
    /// Canonical public declaration/template/definition/dependency closure indexes.
    owners: Vec<Owner>,
}
impl std::fmt::Debug for ExpressionView {
    /// Exposes safe counts only, never private names, source IDs or captured digests.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ExpressionView")
            .field("roots", &self.roots.len())
            .field("closure", &self.owners.len())
            .finish_non_exhaustive()
    }
}

/// Public vocabulary projection; private definitions and captured bundle byte identities are inaccessible.
pub struct ExpressionVocabularyView<'a> {
    /// Private complete authority borrowed only to expose governed public fields.
    view: &'a ExpressionView,
    /// Selected exact bundle metadata index.
    bundle: usize,
}
impl ExpressionVocabularyView<'_> {
    /// Exact canonical logical vocabulary identity.
    #[must_use]
    pub fn identity(&self) -> &str {
        self.view.project.ir().vocabularies[self.bundle]
            .identity
            .identity()
    }
    /// Exact semantic revision, never a package version.
    #[must_use]
    pub fn version(&self) -> &str {
        self.view.project.ir().vocabularies[self.bundle]
            .identity
            .version()
    }
    /// Exact closed nominal schema selector.
    #[must_use]
    pub fn schema_version(&self) -> &str {
        self.view.project.ir().vocabularies[self.bundle]
            .identity
            .schema_version()
    }
    /// Exact required vocabulary feature IDs.
    #[must_use]
    pub fn required_features(&self) -> &[String] {
        self.view.project.ir().vocabularies[self.bundle]
            .identity
            .required_features()
    }
    /// Public definitions required by the selected interpretation closure, in canonical order.
    pub fn definitions(&self) -> impl Iterator<Item = &CompositionDefinition> {
        self.view.owners.iter().filter_map(|owner| match owner {
            Owner::Definition(bundle, definition) if *bundle == self.bundle => {
                Some(&self.view.project.ir().vocabularies[*bundle].definitions[*definition])
            }
            _ => None,
        })
    }
    /// Canonical exact direct dependency metadata; dependency bundles also appear in the view.
    #[must_use]
    pub fn dependencies(&self) -> &[CompositionDependency] {
        &self.view.project.ir().vocabularies[self.bundle].dependencies
    }
}

/// One safe public structural edge; original source identity and location are redacted.
#[derive(Clone, Copy, Debug)]
pub struct ExpressionStructuralEdge<'a> {
    /// Public consuming owner.
    pub from: &'a ModuleSymbolIdentity,
    /// Structural type/reference category, never guarded demand or a prerequisite policy.
    pub kind: ProjectPublicEdgeKind,
    /// Public interpretation target.
    pub to: &'a ModuleSymbolIdentity,
}
/// Safe materialized occurrence classification, with original/private attribution redacted.
#[derive(Clone, Copy, Debug)]
pub struct ExpressionPublicOrigin<'a> {
    /// Selected public binding.
    pub owner: &'a ModuleSymbolIdentity,
    /// Canonical materialized occurrence path.
    pub path: &'a [ValuePathSegment],
    /// Supplied/null/omitted/default classification.
    pub kind: ValueOriginKind,
}
/// Checked original-byte range with source digest/ID deliberately omitted.
#[derive(Clone, Copy, Debug)]
pub struct ExpressionPublicAttribution<'a> {
    /// Selected public declaration/template owner.
    pub owner: &'a ModuleSymbolIdentity,
    /// Canonical retained node occurrence path.
    pub node_path: &'a [u64],
    /// Safe original checked byte range or unavailable source evidence.
    pub span: Option<ByteSpan>,
}
/// Safe original expansion lineage; no fabricated generated source or source identity escapes.
pub struct ExpressionPublicExpansion<'a> {
    /// Selected public generated owner.
    pub owner: &'a ModuleSymbolIdentity,
    /// Original application range.
    pub application: ByteSpan,
    /// Original definition range.
    pub definition: ByteSpan,
    /// Private borrowed locations exposed only through redacted span access.
    arguments: &'a [neutral_core::SourceLocation],
    /// Original source-axis positions in declared order.
    pub original_axis_indices: &'a [u64],
}
impl ExpressionPublicExpansion<'_> {
    /// Checked original argument ranges in parameter order, without source identities.
    pub fn arguments(&self) -> impl Iterator<Item = ByteSpan> + '_ {
        self.arguments.iter().map(|location| location.span())
    }
}

impl ExpressionView {
    /// Exact successor view schema.
    #[must_use]
    pub const fn schema(&self) -> &'static str {
        profile::PROJECT_VIEW_SCHEMA
    }
    /// Exact complete frozen semantic tuple and feature set.
    #[must_use]
    pub fn semantic_profiles(&self) -> &SemanticProfiles {
        &self.project.ir().semantic_profiles
    }
    /// Canonical explicit roots; empty remains empty.
    pub fn roots(&self) -> impl Iterator<Item = &ModuleSymbolIdentity> {
        self.roots
            .iter()
            .map(|owner| owner_symbol(&self.project, *owner))
    }
    /// Public source bindings and nominal contracts required by all potential branches.
    pub fn declarations(&self) -> impl Iterator<Item = &ExpressionDeclaration> {
        self.owners.iter().filter_map(|owner| {
            if let Owner::Declaration(index) = owner {
                Some(&self.project.ir().declarations[*index])
            } else {
                None
            }
        })
    }
    /// Selected non-callable public template contracts, including defaults and definition-scoped bodies.
    pub fn templates(&self) -> impl Iterator<Item = &PublicTemplate> {
        self.owners.iter().filter_map(|owner| {
            if let Owner::Template(index) = owner {
                Some(&self.project.ir().public_templates[*index])
            } else {
                None
            }
        })
    }
    /// Exact dependency bundles exposing only required public definitions.
    pub fn vocabularies(&self) -> impl Iterator<Item = ExpressionVocabularyView<'_>> {
        self.owners.iter().filter_map(|owner| {
            if let Owner::Bundle(bundle) = owner {
                Some(ExpressionVocabularyView {
                    view: self,
                    bundle: *bundle,
                })
            } else {
                None
            }
        })
    }
    /// Structural reference/type evidence, separate from guarded ports and consumer prerequisites.
    pub fn structural_edges(&self) -> impl Iterator<Item = ExpressionStructuralEdge<'_>> {
        self.project
            .ir()
            .provenance
            .iter()
            .filter(|edge| {
                self.contains(&edge.from)
                    && self.contains(&edge.to)
                    && matches!(
                        edge.kind,
                        ProjectPublicEdgeKind::Type
                            | ProjectPublicEdgeKind::ReferenceType
                            | ProjectPublicEdgeKind::Reference
                    )
            })
            .map(|edge| ExpressionStructuralEdge {
                from: &edge.from,
                kind: edge.kind,
                to: &edge.to,
            })
    }
    /// Safe occurrence classifications without complete source/vocabulary attribution objects.
    pub fn origins(&self) -> impl Iterator<Item = ExpressionPublicOrigin<'_>> {
        self.project
            .ir()
            .origins
            .iter()
            .filter(|origin| self.contains(&origin.binding))
            .map(|origin| ExpressionPublicOrigin {
                owner: &origin.binding,
                path: &origin.path,
                kind: origin.kind,
            })
    }
    /// Real checked source byte ranges for public retained nodes, with source identity redacted.
    pub fn expression_attributions(&self) -> impl Iterator<Item = ExpressionPublicAttribution<'_>> {
        self.project
            .ir()
            .expression_attributions
            .iter()
            .filter(|attribution| self.contains(&attribution.owner))
            .map(|attribution| ExpressionPublicAttribution {
                owner: &attribution.owner,
                node_path: &attribution.node_path,
                span: attribution
                    .occurrence
                    .map(neutral_core::SourceLocation::span),
            })
    }
    /// Original application/definition/argument/axis lineage, with source identities redacted.
    pub fn expansion_attributions(&self) -> impl Iterator<Item = ExpressionPublicExpansion<'_>> {
        self.project
            .ir()
            .expansion_attributions
            .iter()
            .filter(|attribution| self.contains(&attribution.owner))
            .map(|attribution| ExpressionPublicExpansion {
                owner: &attribution.owner,
                application: attribution.application.span(),
                definition: attribution.definition.span(),
                arguments: &attribution.arguments,
                original_axis_indices: &attribution.original_axis_indices,
            })
    }
    /// Independently validated future-selector checks belonging to selected public trees.
    pub fn availability_obligations(&self) -> impl Iterator<Item = &AvailabilityObligation> {
        self.project
            .model()
            .availability_obligations()
            .iter()
            .filter(|obligation| self.contains(&obligation.site.owner))
    }
    /// Independently validated nominal field restrictions retained until future value supply.
    pub fn restriction_obligations(&self) -> impl Iterator<Item = &RestrictionObligation> {
        self.project
            .model()
            .restriction_obligations()
            .iter()
            .filter(|obligation| self.contains(&obligation.site.owner))
    }
    /// Derives guarded public ports for an owner already in this view; never evaluates a guard.
    /// # Errors
    /// Rejects unselected owners, independent inspection limits, allocation or cancellation.
    pub fn dependencies<'a>(
        &'a self,
        owner: &'a ModuleSymbolIdentity,
        limits: ExpressionInspectionLimits,
        cancel: &CancellationToken,
    ) -> Result<GuardedDependencies<'a>, E> {
        if !self.contains(owner) {
            return Err(E::Semantic);
        }
        self.project.dependencies(owner, limits, cancel)
    }
    /// Checks selected source/template indexes without exposing complete authority.
    fn contains(&self, owner: &ModuleSymbolIdentity) -> bool {
        self.owners.iter().any(|index| {
            matches!(index, Owner::Declaration(_) | Owner::Template(_))
                && owner_symbol(&self.project, *index) == owner
        })
    }
}

/// Shared bounded work/storage policy for root selection and public projection.
pub(super) struct Budget<'a> {
    /// Remaining independent controls after intersecting validated/hard policy.
    limits: ExpressionViewLimits,
    /// Cooperative cancellation.
    cancel: &'a CancellationToken,
}
impl<'a> Budget<'a> {
    /// Intersects each dimension and rejects zero controls before any retention.
    pub(super) fn new(
        project: &ValidatedExpressionProject,
        limits: ExpressionViewLimits,
        cancel: &'a CancellationToken,
    ) -> Result<Self, E> {
        let accepted = project.model().limits().expression;
        let limits = ExpressionViewLimits {
            roots: limits
                .roots
                .min(profile::MAX_ITEMS)
                .min(project.model().limits().project.declarations),
            closure_edges: limits.closure_edges.min(profile::MAX_ITEMS),
            work: limits.work.min(accepted.work).min(profile::MAX_WORK),
            output_bytes: limits
                .output_bytes
                .min(accepted.retained_bytes)
                .min(project.model().limits().project.artifact_bytes)
                .min(profile::MAX_MODEL_BYTES),
        };
        if [
            limits.roots,
            limits.closure_edges,
            limits.work,
            limits.output_bytes,
        ]
        .contains(&0)
        {
            return Err(E::Limit);
        }
        let mut budget = Self { limits, cancel };
        budget.work(1)?;
        Ok(budget)
    }
    /// Charges before traversal, comparison or fallible reservation.
    pub(super) fn work(&mut self, n: u64) -> Result<(), E> {
        if self.cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        self.limits.work = self.limits.work.checked_sub(n).ok_or(E::Limit)?;
        Ok(())
    }
    /// Charges owned retained storage independently from traversal work.
    pub(super) fn storage(&mut self, bytes: usize) -> Result<(), E> {
        self.work((bytes as u64).checked_add(1).ok_or(E::Limit)?)?;
        self.limits.output_bytes = self
            .limits
            .output_bytes
            .checked_sub(bytes as u64)
            .ok_or(E::Limit)?;
        Ok(())
    }
    /// Charges every scanned canonical symbol byte before lookup or sorting.
    pub(super) fn symbol(&mut self, owner: &ModuleSymbolIdentity, count: usize) -> Result<(), E> {
        let bytes = owner
            .module()
            .language_behavior_version()
            .len()
            .checked_add(owner.module().module_name().len())
            .and_then(|n| n.checked_add(owner.declaration_name().len()))
            .ok_or(E::Limit)?;
        self.work(
            (bytes as u64 + 1)
                .checked_mul(u64::from(count.bit_width()) + 1)
                .ok_or(E::Limit)?,
        )
    }
    /// Charges bounded annotation/type comparison, including every nominal owner key.
    pub(super) fn ty(&mut self, ty: &T, depth: usize) -> Result<(), E> {
        self.work(1)?;
        if depth > profile::MAX_DEPTH {
            return Err(E::Limit);
        }
        match ty {
            T::List(inner) | T::Nullable(inner) | T::Ref(inner) => self.ty(inner, depth + 1),
            T::Nominal(owner) => self.symbol(owner, 1),
            T::VocabularyNominal {
                identity,
                version,
                name,
            } => self.work((identity.len() + version.len() + name.len()) as u64 + 1),
            _ => Ok(()),
        }
    }
    /// Retains the remaining controls for the next stage of the same request.
    pub(super) const fn remaining(&self) -> ExpressionViewLimits {
        self.limits
    }
}

/// Iterative nominal/reference dependency closure avoids recursion through owner cycles.
struct Closure<'p, 'c> {
    /// Complete independently checked authority; no raw input is admitted here.
    project: &'p ValidatedExpressionProject,
    /// Aggregate request work/storage/edge controls.
    budget: Budget<'c>,
    /// Every retained public owner/contract/dependency index.
    owners: Vec<Owner>,
    /// Newly discovered owners still requiring structural expansion.
    pending: Vec<Owner>,
}
impl Closure<'_, '_> {
    /// Follows one interpretation edge, retaining each target once while charging repeated occurrences.
    fn add(&mut self, owner: Owner) -> Result<(), E> {
        self.budget.work(self.owners.len() as u64 + 1)?;
        self.budget.limits.closure_edges = self
            .budget
            .limits
            .closure_edges
            .checked_sub(1)
            .ok_or(E::Limit)?;
        if self.owners.contains(&owner) {
            return Ok(());
        }
        self.budget.storage(std::mem::size_of::<Owner>() * 2)?;
        self.owners.try_retain_exact(1).map_err(|_| E::Limit)?;
        self.pending.try_retain_exact(1).map_err(|_| E::Limit)?;
        self.owners.push(owner);
        self.pending.push(owner);
        self.budget.work(0)
    }
    /// Includes exact nominal owners behind every wrapper.
    fn ty(&mut self, ty: &T) -> Result<(), E> {
        self.budget.ty(ty, 0)?;
        match ty {
            T::List(inner) | T::Nullable(inner) | T::Ref(inner) => self.ty(inner),
            T::Nominal(owner) => {
                let index = lookup(self.project, owner, &mut self.budget)?;
                self.add(index)
            }
            T::VocabularyNominal {
                identity,
                version,
                name,
            } => {
                let ir = self.project.ir();
                self.budget.work(ir.vocabularies.len() as u64 + 1)?;
                let bundle = ir
                    .vocabularies
                    .iter()
                    .position(|b| {
                        b.identity.identity() == identity && b.identity.version() == version
                    })
                    .ok_or(E::Semantic)?;
                self.budget
                    .work(ir.vocabularies[bundle].definitions.len() as u64 + 1)?;
                let definition = ir.vocabularies[bundle]
                    .definitions
                    .iter()
                    .position(|d| d.name == *name && d.public)
                    .ok_or(E::Semantic)?;
                self.add(Owner::Definition(bundle, definition))?;
                self.add(Owner::Bundle(bundle))
            }
            _ => Ok(()),
        }
    }
    /// Includes every nominal field/alternative/default interpretation contract.
    fn body(&mut self, body: &B) -> Result<(), E> {
        match body {
            B::Record(fields) => {
                for field in fields {
                    self.ty(&field.ty)?;
                }
            }
            B::Variant(alternatives) => {
                for alternative in alternatives {
                    self.ty(&alternative.ty)?;
                }
            }
        }
        Ok(())
    }
    /// Includes actual structural references without following ordinary reuse provenance.
    fn value(&mut self, value: &V<ModuleSymbolIdentity>, depth: usize) -> Result<(), E> {
        self.budget.work(1)?;
        if depth > profile::MAX_DEPTH {
            return Err(E::Limit);
        }
        match value {
            V::Reference(owner) => {
                let index = lookup(self.project, owner, &mut self.budget)?;
                self.add(index)
            }
            V::List(items) => {
                for item in items {
                    self.value(item, depth + 1)?;
                }
                Ok(())
            }
            V::Record(fields) => {
                for (_, value) in fields {
                    if let Some(value) = value {
                        self.value(value, depth + 1)?;
                    }
                }
                Ok(())
            }
            V::Variant { payload, .. } => self.value(payload, depth + 1),
            _ => Ok(()),
        }
    }
    /// Visits all potential branches and selectors; no guard is evaluated or topology selected.
    fn tree(&mut self, tree: &Expression, depth: usize) -> Result<(), E> {
        self.budget.work(1)?;
        if depth > profile::MAX_DEPTH {
            return Err(E::Limit);
        }
        self.ty(&tree.ty)?;
        match &tree.node {
            N::Value(value) => self.value(value, 0),
            N::Input(owner) => {
                let index = lookup(self.project, owner, &mut self.budget)?;
                self.add(index)
            }
            N::Output {
                target,
                result_type,
                ..
            } => {
                let index = lookup(self.project, target, &mut self.budget)?;
                self.add(index)?;
                self.ty(result_type)
            }
            N::Unary { operand, .. } | N::Text(operand) => self.tree(operand, depth + 1),
            N::Binary { left, right, .. } => {
                self.tree(left, depth + 1)?;
                self.tree(right, depth + 1)
            }
            N::If {
                guard,
                then_branch,
                else_branch,
            } => {
                self.tree(guard, depth + 1)?;
                self.tree(then_branch, depth + 1)?;
                self.tree(else_branch, depth + 1)
            }
            N::Match { scrutinee, arms } => {
                self.tree(scrutinee, depth + 1)?;
                for arm in arms {
                    self.ty(&arm.payload_type)?;
                    self.tree(&arm.body, depth + 1)?;
                }
                Ok(())
            }
            N::Interpolate(items) | N::List(items) => {
                for item in items {
                    self.tree(item, depth + 1)?;
                }
                Ok(())
            }
            N::Record(fields) => {
                for (_, value) in fields {
                    if let Some(value) = value {
                        self.tree(value, depth + 1)?;
                    }
                }
                Ok(())
            }
            N::Variant { payload, .. } => self.tree(payload, depth + 1),
            N::Bound { .. } => Ok(()),
        }
    }
    /// Expands a queued exact public source/template/vocabulary owner.
    fn expand(&mut self, owner: Owner) -> Result<(), E> {
        self.budget.work(1)?;
        let ir = self.project.ir();
        match owner {
            Owner::Declaration(index) => {
                let declaration = &ir.declarations[index];
                match &declaration.signature {
                    S::Binding(ty) => self.ty(ty)?,
                    S::Definition(body) => self.body(body)?,
                }
                match &declaration.body {
                    Some(ExpressionBody::Value(value)) => self.value(value, 0)?,
                    Some(ExpressionBody::Expression(tree)) => self.tree(tree, 1)?,
                    Some(ExpressionBody::Input) | None => {}
                }
            }
            Owner::Template(index) => {
                let template = &ir.public_templates[index];
                self.ty(&template.result_type)?;
                for parameter in &template.parameters {
                    self.ty(&parameter.ty)?;
                    if let Some(default) = &parameter.default {
                        self.tree(default, 1)?;
                    }
                }
                self.tree(&template.body, 1)?;
            }
            Owner::Definition(bundle, definition) => {
                self.body(&ir.vocabularies[bundle].definitions[definition].body)?;
            }
            Owner::Bundle(bundle) => {
                for dependency in &ir.vocabularies[bundle].dependencies {
                    self.budget.work(
                        ir.vocabularies.len() as u64
                            + (dependency.identity.len() + dependency.version.len()) as u64
                            + 1,
                    )?;
                    let target = ir
                        .vocabularies
                        .iter()
                        .position(|b| {
                            b.identity.identity() == dependency.identity
                                && b.identity.version() == dependency.version
                        })
                        .ok_or(E::Semantic)?;
                    self.add(Owner::Bundle(target))?;
                }
            }
        }
        Ok(())
    }
}

/// Borrows a public source/template owner from a retained index, never a vocabulary owner.
fn owner_symbol(project: &ValidatedExpressionProject, owner: Owner) -> &ModuleSymbolIdentity {
    match owner {
        Owner::Declaration(index) => &project.ir().declarations[index].identity,
        Owner::Template(index) => &project.ir().public_templates[index].identity,
        Owner::Definition(_, _) | Owner::Bundle(_) => {
            unreachable!("only source/template indexes have symbols")
        }
    }
}

/// Checks exact source/template visibility and bounded canonical lookup keys.
fn lookup(
    project: &ValidatedExpressionProject,
    owner: &ModuleSymbolIdentity,
    budget: &mut Budget<'_>,
) -> Result<Owner, E> {
    let ir = project.ir();
    budget.symbol(owner, ir.declarations.len() + ir.public_templates.len())?;
    if let Ok(index) = ir.declarations.binary_search_by(|d| d.identity.cmp(owner)) {
        if ir.declarations[index].public {
            return Ok(Owner::Declaration(index));
        }
        return Err(E::Semantic);
    }
    ir.public_templates
        .binary_search_by(|t| t.identity.cmp(owner))
        .map(Owner::Template)
        .map_err(|_| E::Semantic)
}

/// Common projection entry used by explicit roots and already-selected public bindings.
pub(super) fn derive<'p>(
    project: &ValidatedExpressionProject,
    roots: impl Iterator<Item = &'p ModuleSymbolIdentity>,
    root_count: usize,
    limits: ExpressionViewLimits,
    cancel: &CancellationToken,
) -> Result<ExpressionView, E> {
    let mut closure = Closure {
        project,
        budget: Budget::new(project, limits, cancel)?,
        owners: Vec::new(),
        pending: Vec::new(),
    };
    if root_count as u64 > closure.budget.limits.roots {
        return Err(E::Limit);
    }
    closure
        .budget
        .storage(std::mem::size_of::<ExpressionView>())?;
    let mut selected = Vec::new();
    for root in roots {
        let owner = lookup(project, root, &mut closure.budget)?;
        closure.budget.storage(std::mem::size_of::<Owner>())?;
        selected.try_retain_exact(1).map_err(|_| E::Limit)?;
        selected.push(owner);
        closure.add(owner)?;
    }
    for owner in &selected {
        closure
            .budget
            .symbol(owner_symbol(project, *owner), selected.len())?;
    }
    selected.sort_unstable_by(|a, b| owner_symbol(project, *a).cmp(owner_symbol(project, *b)));
    if selected.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(E::Semantic);
    }
    while let Some(owner) = closure.pending.pop() {
        closure.expand(owner)?;
    }
    closure
        .budget
        .work(closure.owners.len() as u64 * (u64::from(closure.owners.len().bit_width()) + 1))?;
    closure.owners.sort_unstable_by_key(|owner| match owner {
        Owner::Declaration(index) => (0, *index, 0),
        Owner::Template(index) => (1, *index, 0),
        Owner::Definition(bundle, definition) => (2, *bundle, *definition),
        Owner::Bundle(bundle) => (3, *bundle, 0),
    });
    closure.budget.work(0)?;
    Ok(ExpressionView {
        project: project.clone(),
        roots: selected,
        owners: closure.owners,
    })
}

impl ValidatedExpressionProject {
    /// Derives an explicit public closure under frozen hard projection controls.
    /// # Errors
    /// Rejects unsupported tuples, invalid/duplicate/private roots, bounds, allocation or cancellation.
    pub fn derive_view(
        &self,
        request: &ExpressionViewRequest<'_>,
        cancel: &CancellationToken,
    ) -> Result<ExpressionView, E> {
        self.derive_view_with_limits(request, ExpressionViewLimits::default(), cancel)
    }
    /// Derives every public interpretation dependency while redacting source identity and private owners.
    /// # Errors
    /// Rejects unsupported tuples, invalid roots, independent controls, allocation or cancellation atomically.
    pub fn derive_view_with_limits(
        &self,
        request: &ExpressionViewRequest<'_>,
        limits: ExpressionViewLimits,
        cancel: &CancellationToken,
    ) -> Result<ExpressionView, E> {
        if cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        if request.schema != profile::PROJECT_VIEW_SCHEMA
            || !request.semantic_profiles.matches_frozen()
            || request.semantic_profiles != &self.ir().semantic_profiles
        {
            return Err(E::Schema);
        }
        derive(
            self,
            request.roots.iter(),
            request.roots.len(),
            limits,
            cancel,
        )
    }
}
