// SPDX-License-Identifier: Apache-2.0

//! Post-validation public interpretive closure; source/provenance evidence never escapes.

use super::{CompositionReadError as E, ValidatedCompositionProject};
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

/// Explicit post-compilation selection, never a capture or compiler input.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionViewRequest {
    /// Exact successor view selector.
    pub schema: String,
    /// Public roots; empty means an empty view, not all exports.
    pub roots: Vec<ModuleSymbolIdentity>,
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
        self.step()?;
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
        self.step()?;
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
        self.step()?;
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
        if cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        if request.schema != profile::PROJECT_VIEW_SCHEMA {
            return Err(E::Schema);
        }
        let ir = self.complete_ir();
        if request.roots.len() > ir.declarations.len() {
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
        if key_bytes > self.effective_composition_limits().work {
            return Err(E::Limit);
        }
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
            work: self
                .effective_composition_limits()
                .work
                .saturating_sub(key_bytes)
                .min(profile::MAX_ITEMS),
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
        if !closure.sources.contains(&d.identity) {
            continue;
        }
        closure.step()?;
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
                push(&mut definitions, copy(d)?)?;
            }
        }
        push(
            &mut vocabularies,
            CompositionBundle {
                identity: copy(&b.identity)?,
                dependencies: copy(&b.dependencies)?,
                definitions,
            },
        )?;
    }
    let mut origins = Vec::new();
    for o in &ir.origins {
        if !closure.sources.contains(&o.binding) {
            continue;
        }
        closure.step()?;
        let attribution = match &o.attribution {
            Some(A::Vocabulary {
                identity,
                version,
                type_name,
                field_name,
                ..
            }) if has_type(closure, identity, version, type_name) => Some(A::Vocabulary {
                identity: owned(identity)?,
                version: owned(version)?,
                type_name: owned(type_name)?,
                field_name: owned(field_name)?,
                span: None,
            }),
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
    let mut references = Vec::new();
    for b in closure.project.bindings().bindings() {
        if !closure.sources.contains(&b.binding().owner) {
            continue;
        }
        for r in b.references() {
            closure.step()?;
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
    values.try_retain(1).map_err(|_| E::Limit)?;
    values.push(value);
    Ok(())
}
