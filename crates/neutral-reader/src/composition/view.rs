// SPDX-License-Identifier: Apache-2.0

//! Post-validation public interpretive closure; source/provenance evidence never escapes.

use super::{CompositionReadError as E, ValidatedCompositionProject};
mod retention;
use neutral_core::CancellationToken;
use neutral_core::allocation::RetainCapacity;
use neutral_core::allocation::{TryClone, copy_slice, text};
use neutral_core::ordered::OrderedSet as BTreeSet;
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::{
        CompositionBindingReference, CompositionBody as B, CompositionBundle, profile,
        project::{
            CompositionAttribution as A, CompositionDeclaration, CompositionOrigin,
            CompositionSignature as S,
        },
    },
    project_interface::ProjectPublicType as T,
};
use retention::Retention;

/// Explicit post-compilation selection, never a capture or compiler input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionViewRequest {
    /// Exact successor view selector.
    pub schema: String,
    /// Public roots; empty means an empty view, not all exports.
    pub roots: Vec<ModuleSymbolIdentity>,
}

/// Independent positive public-projection controls, intersected with reader and hard policy.
#[derive(Clone, Copy, Debug)]
pub struct CompositionViewLimits {
    /// Selected public roots, independent of closure size.
    pub roots: u64,
    /// Type, source and vocabulary dependency traversal edges, including repeated edges.
    pub closure_edges: u64,
    /// Total closure, comparison and deep-retention preflight work.
    pub work: u64,
    /// Checked owned model storage; borrowed authority/transient indexes and transport bytes are separate.
    pub output_bytes: u64,
}
impl Default for CompositionViewLimits {
    fn default() -> Self {
        Self {
            roots: profile::MAX_ITEMS,
            closure_edges: profile::MAX_ITEMS,
            work: profile::MAX_VIEW_WORK,
            output_bytes: profile::MAX_VIEW_OUTPUT_BYTES,
        }
    }
}

/// Immutable public interpretive closure without private source IDs, spans or implementation edges.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionView {
    /// Normalized explicit selection.
    roots: Vec<ModuleSymbolIdentity>,
    /// All public source signatures/values needed to interpret selected roots.
    declarations: Vec<CompositionDeclaration>,
    /// Public contract definitions and exact transitive dependency metadata.
    vocabularies: Vec<CompositionBundle>,
    /// Redacted occurrence facts; source attribution is explicitly unavailable.
    origins: Vec<CompositionOrigin>,
    /// Actual public binding references, keyed by their consuming owner.
    references: Vec<(ModuleSymbolIdentity, CompositionBindingReference)>,
}
impl CompositionView {
    /// Returns exact governed profile/features; actual vocabulary schemas/revisions are in `vocabularies()`.
    #[must_use]
    pub const fn profile_facts(&self) -> profile::CompositionProfileDescriptor<'static> {
        profile::COMPOSITION_PROFILE
    }
    /// Returns the successor selection schema, never complete-project authority.
    #[must_use]
    pub const fn schema(&self) -> &'static str {
        profile::PROJECT_VIEW_SCHEMA
    }
    /// Returns canonical selected roots.
    #[must_use]
    pub fn roots(&self) -> &[ModuleSymbolIdentity] {
        &self.roots
    }
    /// Returns complete public source interpretation contracts and materialized values.
    #[must_use]
    pub fn declarations(&self) -> &[CompositionDeclaration] {
        &self.declarations
    }
    /// Returns public vocabulary facts, including all alternative/reference types and restrictions/defaults.
    #[must_use]
    pub fn vocabularies(&self) -> &[CompositionBundle] {
        &self.vocabularies
    }
    /// Returns safe classifications and canonical vocabulary attribution without private source evidence.
    #[must_use]
    pub fn origins(&self) -> &[CompositionOrigin] {
        &self.origins
    }
    /// Returns every actual exposed reference occurrence and target in canonical order.
    #[must_use]
    pub fn references(&self) -> &[(ModuleSymbolIdentity, CompositionBindingReference)] {
        &self.references
    }
}

/// Bounded closure state over independently validated complete data.
struct Closure<'a> {
    /// Immutable complete reader authority.
    project: &'a ValidatedCompositionProject,
    /// Cooperative cancellation.
    cancel: &'a CancellationToken,
    /// Remaining independently intersected work budget.
    work: u64,
    /// Remaining independent edge bound, charged before following dependencies.
    edges: u64,
    /// Deep owned output accounting before every clone.
    retention: Retention<'a>,
    /// Required source owners, including reference targets.
    sources: BTreeSet<ModuleSymbolIdentity>,
    /// Iterative binding/type worklist; reference chains do not consume Rust stack.
    pending: Vec<ModuleSymbolIdentity>,
    /// Required public vocabulary definitions.
    types: BTreeSet<(String, String, String)>,
    /// Iterative nominal worklist; reference-type chains do not consume Rust stack.
    pending_types: Vec<(String, String, String)>,
    /// Exact transitive bundle closure.
    bundles: BTreeSet<(String, String)>,
}
impl Closure<'_> {
    /// Charges every scanned key, including unselected declarations/occurrences, before membership lookup.
    fn scan_symbol(&mut self, owner: &ModuleSymbolIdentity) -> Result<(), E> {
        self.step()?;
        let bytes = (owner.module().language_behavior_version().len()
            + owner.module().module_name().len()
            + owner.declaration_name().len()) as u64
            + 1;
        let comparisons = u64::from(self.sources.len().bit_width()) + 1;
        self.work = self
            .work
            .checked_sub(bytes.checked_mul(comparisons).ok_or(E::Limit)?)
            .ok_or(E::Limit)?;
        Ok(())
    }
    /// Charges an interpretation dependency before following it, including already visited targets.
    fn edge(&mut self) -> Result<(), E> {
        self.step()?;
        self.edges = self.edges.checked_sub(1).ok_or(E::Limit)?;
        Ok(())
    }
    /// Charges traversal before following another interpretation dependency.
    fn step(&mut self) -> Result<(), E> {
        if self.cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        let ir = self.project.complete_ir();
        let scan = 1_u64
            + ir.declarations.len() as u64
            + ir.vocabularies
                .iter()
                .map(|b| b.definitions.len() as u64)
                .sum::<u64>();
        self.work = self.work.checked_sub(scan).ok_or(E::Limit)?;
        Ok(())
    }
    /// Includes nominal definitions behind every wrapper, not just selected variant payloads.
    fn ty(&mut self, ty: &T) -> Result<(), E> {
        self.edge()?;
        match ty {
            T::List(inner) | T::Ref(inner) | T::Nullable(inner) => self.ty(inner),
            T::Nominal(owner) => self.source(owner),
            T::VocabularyNominal {
                identity,
                version,
                name,
            } => {
                self.shift(self.types.len())?;
                let key = (owned(identity)?, owned(version)?, owned(name)?);
                if self.types.insert(copy(&key)?).map_err(|_| E::Limit)? {
                    self.pending_types.try_retain(1).map_err(|_| E::Limit)?;
                    self.pending_types.push(key);
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
    /// Expands one vocabulary contract; nominal reference targets are queued instead of recursively expanded.
    fn expand_type(&mut self, identity: &str, version: &str, name: &str) -> Result<(), E> {
        self.step()?;
        let bundle = self
            .project
            .complete_ir()
            .vocabularies
            .iter()
            .find(|b| b.identity.identity() == identity && b.identity.version() == version)
            .ok_or(E::Semantic)?;
        let definition = bundle
            .definitions
            .iter()
            .find(|d| d.name == name && d.public)
            .ok_or(E::Semantic)?;
        self.body(&definition.body)?;
        self.bundle(identity, version)
    }
    /// Includes every field and alternative; closed defaults cannot introduce binding references.
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
    /// Includes public source definitions and actual binding-reference targets, never ordinary reuse provenance.
    fn source(&mut self, owner: &ModuleSymbolIdentity) -> Result<(), E> {
        self.edge()?;
        self.shift(self.sources.len())?;
        if !self.sources.insert(copy(owner)?).map_err(|_| E::Limit)? {
            return Ok(());
        }
        self.pending.try_retain(1).map_err(|_| E::Limit)?;
        self.pending.push(copy(owner)?);
        Ok(())
    }
    /// Expands one queued public declaration without recursively following binding references.
    fn expand(&mut self, owner: &ModuleSymbolIdentity) -> Result<(), E> {
        let declaration = self
            .project
            .complete_ir()
            .declarations
            .iter()
            .find(|d| d.identity == *owner && d.public)
            .ok_or(E::Semantic)?;
        match &declaration.signature {
            S::Definition(body) => self.body(body)?,
            S::Binding(ty) => {
                self.ty(ty)?;
                let binding = self
                    .project
                    .bindings()
                    .bindings()
                    .iter()
                    .find(|b| b.binding().owner == *owner)
                    .ok_or(E::Semantic)?;
                for edge in binding.references() {
                    self.source(&edge.target)?;
                }
            }
        }
        Ok(())
    }
    /// Includes exact transitive bundle locks even when a dependency has no selected type.
    fn bundle(&mut self, identity: &str, version: &str) -> Result<(), E> {
        self.edge()?;
        self.shift(self.bundles.len())?;
        if !self
            .bundles
            .insert((owned(identity)?, owned(version)?))
            .map_err(|_| E::Limit)?
        {
            return Ok(());
        }
        let bundle = self
            .project
            .complete_ir()
            .vocabularies
            .iter()
            .find(|b| b.identity.identity() == identity && b.identity.version() == version)
            .ok_or(E::Semantic)?;
        for dependency in &bundle.dependencies {
            self.bundle(&dependency.identity, &dependency.version)?;
        }
        Ok(())
    }
    /// Charges worst-case sorted-index shifts before fallible insertion.
    fn shift(&mut self, count: usize) -> Result<(), E> {
        if self.cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        self.work = self.work.checked_sub(count as u64).ok_or(E::Limit)?;
        Ok(())
    }
}

impl ValidatedCompositionProject {
    /// Derives a public dependency-closed view only after complete validation, without changing identities.
    ///
    /// # Errors
    /// Rejects selector mismatch, private/unknown/duplicate roots, bounds or cancellation atomically.
    pub fn derive_view(
        &self,
        request: &CompositionViewRequest,
        cancel: &CancellationToken,
    ) -> Result<CompositionView, E> {
        self.derive_view_with_limits(request, CompositionViewLimits::default(), cancel)
    }

    /// Derives complete public interpretation closure under independent pre-retention controls.
    ///
    /// # Errors
    /// Rejects schema/roots, invalid controls, missing closure, allocation, bounds or cancellation atomically.
    pub fn derive_view_with_limits(
        &self,
        request: &CompositionViewRequest,
        limits: CompositionViewLimits,
        cancel: &CancellationToken,
    ) -> Result<CompositionView, E> {
        if cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        if request.schema != profile::PROJECT_VIEW_SCHEMA {
            return Err(E::Schema);
        }
        let hard = CompositionViewLimits::default();
        let limits = CompositionViewLimits {
            roots: limits.roots.min(hard.roots),
            closure_edges: limits.closure_edges.min(hard.closure_edges),
            work: limits
                .work
                .min(hard.work)
                .min(self.effective_composition_limits().work),
            output_bytes: limits.output_bytes.min(hard.output_bytes),
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
        let ir = self.complete_ir();
        if request.roots.len() > ir.declarations.len() || request.roots.len() as u64 > limits.roots
        {
            return Err(E::Limit);
        }
        let key_bytes = request
            .roots
            .iter()
            .try_fold(0_u64, |sum, owner| {
                sum.checked_add(
                    (owner.module().language_behavior_version().len()
                        + owner.module().module_name().len()
                        + owner.declaration_name().len()) as u64,
                )
            })
            .ok_or(E::Limit)?;
        if key_bytes > limits.work {
            return Err(E::Limit);
        }
        let mut work = limits.work;
        let mut retention = Retention::new(limits.output_bytes, cancel);
        retention.storage::<CompositionView>(1, &mut work)?;
        retention.storage::<ModuleSymbolIdentity>(request.roots.len(), &mut work)?;
        for root in &request.roots {
            retention.symbol(root, &mut work)?;
        }
        let sorting = key_bytes
            .checked_mul(u64::from(request.roots.len().bit_width()) + 1)
            .ok_or(E::Limit)?;
        work = work.checked_sub(sorting).ok_or(E::Limit)?;
        let mut roots = copy_slice(&request.roots).map_err(|_| E::Limit)?;
        roots.sort_unstable();
        if roots.windows(2).any(|pair| pair[0] == pair[1])
            || roots.iter().any(|r| {
                !ir.declarations
                    .binary_search_by(|d| d.identity.cmp(r))
                    .is_ok_and(|i| ir.declarations[i].public)
            })
        {
            return Err(E::Semantic);
        }
        let mut closure = Closure {
            project: self,
            cancel,
            work,
            edges: limits.closure_edges,
            retention,
            sources: BTreeSet::new(),
            pending: Vec::new(),
            types: BTreeSet::new(),
            pending_types: Vec::new(),
            bundles: BTreeSet::new(),
        };
        for root in &roots {
            closure.source(root)?;
        }
        while !closure.pending.is_empty() || !closure.pending_types.is_empty() {
            if let Some(owner) = closure.pending.pop() {
                closure.expand(&owner)?;
            }
            if let Some((identity, version, name)) = closure.pending_types.pop() {
                closure.expand_type(&identity, &version, &name)?;
            }
        }
        project_view(&mut closure, roots)
    }
}

/// Copies only interpretation closure and redacts every source/private implementation companion.
fn project_view(
    closure: &mut Closure<'_>,
    roots: Vec<ModuleSymbolIdentity>,
) -> Result<CompositionView, E> {
    let ir = closure.project.complete_ir();
    let mut declarations = Vec::new();
    for d in &ir.declarations {
        closure.scan_symbol(&d.identity)?;
        if !closure.sources.contains(&d.identity) {
            continue;
        }
        closure
            .retention
            .storage::<CompositionDeclaration>(1, &mut closure.work)?;
        closure.retention.declaration(d, &mut closure.work)?;
        push(&mut declarations, copy(d)?)?;
    }
    let mut vocabularies = Vec::new();
    for b in &ir.vocabularies {
        closure.step()?;
        if !closure.bundles.iter().any(|(identity, version)| {
            identity == b.identity.identity() && version == b.identity.version()
        }) {
            continue;
        }
        let mut definitions = Vec::new();
        for d in &b.definitions {
            closure.step()?;
            if d.public
                && has_type(
                    closure,
                    b.identity.identity(),
                    b.identity.version(),
                    &d.name,
                )
            {
                closure
                    .retention
                    .storage::<neutral_ir::composition::CompositionDefinition>(
                        1,
                        &mut closure.work,
                    )?;
                closure.retention.text(&d.name, &mut closure.work)?;
                closure.retention.body(&d.body, &mut closure.work)?;
                push(&mut definitions, copy(d)?)?;
            }
        }
        closure
            .retention
            .storage::<CompositionBundle>(1, &mut closure.work)?;
        closure.retention.vocabulary(b, &mut closure.work)?;
        push(
            &mut vocabularies,
            CompositionBundle {
                identity: copy(&b.identity)?,
                dependencies: copy(&b.dependencies)?,
                definitions,
            },
        )?;
    }
    let origins = project_origins(closure)?;
    let mut references = Vec::new();
    for b in closure.project.bindings().bindings() {
        closure.scan_symbol(&b.binding().owner)?;
        if !closure.sources.contains(&b.binding().owner) {
            continue;
        }
        for r in b.references() {
            closure.step()?;
            closure
                .retention
                .storage::<(ModuleSymbolIdentity, CompositionBindingReference)>(
                    1,
                    &mut closure.work,
                )?;
            closure
                .retention
                .symbol(&b.binding().owner, &mut closure.work)?;
            closure.retention.symbol(&r.target, &mut closure.work)?;
            closure.retention.path(&r.path, &mut closure.work)?;
            push(&mut references, (copy(&b.binding().owner)?, copy(r)?))?;
        }
    }
    closure.step()?;
    Ok(CompositionView {
        roots,
        declarations,
        vocabularies,
        origins,
        references,
    })
}

/// Copies only checked public origin classifications and canonical default owners, never source spans.
fn project_origins(closure: &mut Closure<'_>) -> Result<Vec<CompositionOrigin>, E> {
    let mut origins = Vec::new();
    for o in &closure.project.complete_ir().origins {
        closure.scan_symbol(&o.binding)?;
        if !closure.sources.contains(&o.binding) {
            continue;
        }
        closure
            .retention
            .storage::<CompositionOrigin>(1, &mut closure.work)?;
        closure.retention.symbol(&o.binding, &mut closure.work)?;
        closure.retention.path(&o.path, &mut closure.work)?;
        let attribution = match &o.attribution {
            Some(A::Vocabulary {
                identity,
                version,
                type_name,
                field_name,
                ..
            }) if has_type(closure, identity, version, type_name) => {
                for name in [identity, version, type_name, field_name] {
                    closure.retention.text(name, &mut closure.work)?;
                }
                Some(A::Vocabulary {
                    identity: owned(identity)?,
                    version: owned(version)?,
                    type_name: owned(type_name)?,
                    field_name: owned(field_name)?,
                    span: None,
                })
            }
            _ => None,
        };
        push(
            &mut origins,
            CompositionOrigin {
                binding: copy(&o.binding)?,
                path: copy(&o.path)?,
                kind: o.kind,
                attribution,
            },
        )?;
    }
    Ok(origins)
}
/// Checks nominal membership using borrowed spellings without allocating temporary tuple keys.
fn has_type(closure: &Closure<'_>, identity: &str, version: &str, name: &str) -> bool {
    closure
        .types
        .iter()
        .any(|(i, v, n)| i == identity && v == version && n == name)
}
/// Copies preflighted retained data without permitting allocator-abort ownership constructors.
fn copy<T: TryClone>(value: &T) -> Result<T, E> {
    value.try_clone().map_err(|_| E::Limit)
}
/// Retains one bounded logical spelling through a fallible string reservation.
fn owned(value: &str) -> Result<String, E> {
    text(value).map_err(|_| E::Limit)
}
/// Reserves unpublished view retention before appending, never exposing a partial view.
fn push<T>(values: &mut Vec<T>, value: T) -> Result<(), E> {
    // The output preflight charges requested slots, not geometric spare capacity.
    values.try_retain_exact(1).map_err(|_| E::Limit)?;
    values.push(value);
    Ok(())
}
