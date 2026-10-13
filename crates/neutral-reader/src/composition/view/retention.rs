// SPDX-License-Identifier: Apache-2.0

//! Borrowed deep-size preflight before public projection copies; validated depth bounds recursion.

use super::{E, S, T};
use neutral_core::CancellationToken;
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::{
        CompositionBody as Body, CompositionBundle, CompositionDependency, CompositionValue as V,
        ValuePathSegment, project::CompositionDeclaration,
    },
};

#[cfg(test)]
#[path = "../../../tests/unit/composition_retention.rs"]
mod tests;

/// Request-local owned output bound; no allocation or producer assertions are needed to count it.
pub(super) struct Retention<'a> {
    /// Unconsumed requested owned storage, excluding allocator metadata.
    remaining: u64,
    /// Signal checked on every node/byte charge.
    cancel: &'a CancellationToken,
}
impl<'a> Retention<'a> {
    /// Starts allocation-free output accounting for one projection.
    pub(super) const fn new(remaining: u64, cancel: &'a CancellationToken) -> Self {
        Self { remaining, cancel }
    }
    /// Charges output and the shared work counter before any retained clone.
    fn charge(&mut self, bytes: u64, work: &mut u64) -> Result<(), E> {
        if self.cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        *work = work
            .checked_sub(bytes.checked_add(1).ok_or(E::Limit)?)
            .ok_or(E::Limit)?;
        self.remaining = self.remaining.checked_sub(bytes).ok_or(E::Limit)?;
        Ok(())
    }
    /// Accounts vector elements or boxed storage without unchecked multiplication.
    pub(super) fn storage<T>(&mut self, count: usize, work: &mut u64) -> Result<(), E> {
        self.charge(
            (count as u64)
                .checked_mul(std::mem::size_of::<T>() as u64)
                .ok_or(E::Limit)?,
            work,
        )
    }
    /// Accounts exact retained UTF-8, never original source byte claims.
    pub(super) fn text(&mut self, value: &str, work: &mut u64) -> Result<(), E> {
        self.charge(value.len() as u64, work)
    }
    /// Accounts every dynamically owned stable symbol component.
    pub(super) fn symbol(&mut self, value: &ModuleSymbolIdentity, work: &mut u64) -> Result<(), E> {
        self.text(value.module().language_behavior_version(), work)?;
        self.text(value.module().module_name(), work)?;
        self.text(value.declaration_name(), work)
    }
    /// Accounts wrappers and nominal keys under already validated type-depth limits.
    fn ty(&mut self, ty: &T, work: &mut u64) -> Result<(), E> {
        self.charge(0, work)?;
        match ty {
            T::List(inner) | T::Nullable(inner) | T::Ref(inner) => {
                self.storage::<T>(1, work)?;
                self.ty(inner, work)
            }
            T::Nominal(owner) => self.symbol(owner, work),
            T::VocabularyNominal {
                identity,
                version,
                name,
            } => {
                self.text(identity, work)?;
                self.text(version, work)?;
                self.text(name, work)
            }
            _ => Ok(()),
        }
    }
    /// Accounts nested dynamic storage; references retain keys rather than expanding targets.
    fn value<R>(
        &mut self,
        value: &V<R>,
        work: &mut u64,
        reference: fn(&R) -> Option<&ModuleSymbolIdentity>,
    ) -> Result<(), E> {
        self.charge(0, work)?;
        match value {
            V::Number(n) => self.text(n.coefficient(), work),
            V::String(s) | V::Url(s) | V::Path(s) => self.text(s, work),
            V::List(values) => {
                self.storage::<V<R>>(values.len(), work)?;
                for value in values {
                    self.value(value, work, reference)?;
                }
                Ok(())
            }
            V::Record(fields) => {
                self.storage::<(String, Option<V<R>>)>(fields.len(), work)?;
                for (name, value) in fields {
                    self.text(name, work)?;
                    if let Some(value) = value {
                        self.value(value, work, reference)?;
                    }
                }
                Ok(())
            }
            V::Variant { tag, payload } => {
                self.text(tag, work)?;
                self.storage::<V<R>>(1, work)?;
                self.value(payload, work, reference)
            }
            V::Reference(r) => {
                if let Some(owner) = reference(r) {
                    self.symbol(owner, work)?;
                }
                Ok(())
            }
            V::Null | V::Bool(_) => Ok(()),
        }
    }
    /// Includes all defaults/restrictions and alternatives, not only currently selected values.
    pub(super) fn body(&mut self, body: &Body, work: &mut u64) -> Result<(), E> {
        match body {
            Body::Record(fields) => {
                self.storage::<neutral_ir::composition::CompositionField>(fields.len(), work)?;
                for field in fields {
                    self.text(&field.name, work)?;
                    self.ty(&field.ty, work)?;
                    if let Some(values) = &field.restrictions.choices {
                        self.storage::<neutral_ir::composition::ClosedValue>(values.len(), work)?;
                        for value in values {
                            self.value(value, work, |_| None)?;
                        }
                    }
                    for number in [&field.restrictions.minimum, &field.restrictions.maximum]
                        .into_iter()
                        .flatten()
                    {
                        self.text(number.coefficient(), work)?;
                    }
                    if let Some(value) = &field.default {
                        self.value(value, work, |_| None)?;
                    }
                }
                Ok(())
            }
            Body::Variant(alternatives) => {
                self.storage::<neutral_ir::composition::CompositionAlternative>(
                    alternatives.len(),
                    work,
                )?;
                for alt in alternatives {
                    self.text(&alt.tag, work)?;
                    self.ty(&alt.ty, work)?;
                }
                Ok(())
            }
        }
    }
    /// Accounts a public signature and any fully materialized binding value.
    pub(super) fn declaration(
        &mut self,
        declaration: &CompositionDeclaration,
        work: &mut u64,
    ) -> Result<(), E> {
        self.symbol(&declaration.identity, work)?;
        match &declaration.signature {
            S::Binding(ty) => self.ty(ty, work)?,
            S::Definition(body) => self.body(body, work)?,
        }
        if let Some(value) = &declaration.value {
            self.value(value, work, |r| Some(r))?;
        }
        Ok(())
    }
    /// Accounts bundle identity/features/dependencies; selected definitions are charged separately.
    pub(super) fn vocabulary(
        &mut self,
        bundle: &CompositionBundle,
        work: &mut u64,
    ) -> Result<(), E> {
        let v = &bundle.identity;
        for text in [
            v.identity(),
            v.version(),
            v.schema_version(),
            v.encoding_version(),
        ] {
            self.text(text, work)?;
        }
        self.storage::<String>(v.required_features().len(), work)?;
        for feature in v.required_features() {
            self.text(feature, work)?;
        }
        self.storage::<CompositionDependency>(bundle.dependencies.len(), work)?;
        for dependency in &bundle.dependencies {
            self.text(&dependency.identity, work)?;
            self.text(&dependency.version, work)?;
        }
        Ok(())
    }
    /// Accounts owned occurrence segments and field-name text.
    pub(super) fn path(&mut self, path: &[ValuePathSegment], work: &mut u64) -> Result<(), E> {
        self.storage::<ValuePathSegment>(path.len(), work)?;
        for segment in path {
            if let ValuePathSegment::Field(name) = segment {
                self.text(name, work)?;
            }
        }
        Ok(())
    }
}
