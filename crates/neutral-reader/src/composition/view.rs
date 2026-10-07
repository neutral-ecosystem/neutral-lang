// SPDX-License-Identifier: Apache-2.0

//! Post-validation public interpretive closure; source/provenance evidence never escapes.

use super::{CompositionReadError as E, ValidatedCompositionProject};
use neutral_core::CancellationToken;
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
use std::collections::{BTreeMap, BTreeSet};

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
                let key = (identity.clone(), version.clone(), name.clone());
                if self.types.insert(key.clone()) {
                    self.pending_types.try_reserve(1).map_err(|_| E::Limit)?;
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
        if !self.sources.insert(owner.clone()) {
            return Ok(());
        }
        self.pending.try_reserve(1).map_err(|_| E::Limit)?;
        self.pending.push(owner.clone());
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
        if !self
            .bundles
            .insert((identity.to_owned(), version.to_owned()))
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
        let public = ir
            .declarations
            .iter()
            .filter(|d| d.public)
            .map(|d| (&d.identity, d))
            .collect::<BTreeMap<_, _>>();
        let roots = request.roots.iter().cloned().collect::<BTreeSet<_>>();
        if roots.len() != request.roots.len() || roots.iter().any(|r| !public.contains_key(r)) {
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
    roots: BTreeSet<ModuleSymbolIdentity>,
) -> Result<CompositionView, E> {
    let ir = closure.project.complete_ir();
    let declarations = ir
        .declarations
        .iter()
        .filter(|d| closure.sources.contains(&d.identity))
        .cloned()
        .collect();
    let vocabularies = ir
        .vocabularies
        .iter()
        .filter(|b| {
            closure.bundles.contains(&(
                b.identity.identity().to_owned(),
                b.identity.version().to_owned(),
            ))
        })
        .map(|b| CompositionBundle {
            identity: b.identity.clone(),
            dependencies: b.dependencies.clone(),
            definitions: b
                .definitions
                .iter()
                .filter(|d| {
                    d.public
                        && closure.types.contains(&(
                            b.identity.identity().to_owned(),
                            b.identity.version().to_owned(),
                            d.name.clone(),
                        ))
                })
                .cloned()
                .collect(),
        })
        .collect();
    let origins = ir
        .origins
        .iter()
        .filter(|o| closure.sources.contains(&o.binding))
        .map(|o| {
            let mut origin = o.clone();
            origin.attribution = match &o.attribution {
                Some(A::Vocabulary {
                    identity,
                    version,
                    type_name,
                    field_name,
                    ..
                }) if closure.types.contains(&(
                    identity.clone(),
                    version.clone(),
                    type_name.clone(),
                )) =>
                {
                    Some(A::Vocabulary {
                        identity: identity.clone(),
                        version: version.clone(),
                        type_name: type_name.clone(),
                        field_name: field_name.clone(),
                        span: None,
                    })
                }
                _ => None,
            };
            origin
        })
        .collect();
    let references = closure
        .project
        .bindings()
        .bindings()
        .iter()
        .filter(|b| closure.sources.contains(&b.binding().owner))
        .flat_map(|b| {
            b.references()
                .iter()
                .map(|r| (b.binding().owner.clone(), r.clone()))
        })
        .collect();
    closure.step()?;
    Ok(CompositionView {
        roots: roots.into_iter().collect(),
        declarations,
        vocabularies,
        origins,
        references,
    })
}
