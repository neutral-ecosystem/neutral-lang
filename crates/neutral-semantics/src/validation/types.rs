// SPDX-License-Identifier: Apache-2.0

//! Exact nominal/public lookup and canonical wire text accounting under request budgets.

use super::{B, E, S, State};
use neutral_ir::{ModuleSymbolIdentity, language, project_interface::ProjectPublicType as T};

impl<'a> State<'a> {
    /// Resolves a binding symbol without fabricating a value for external input.
    pub(super) fn binding(
        &mut self,
        owner: &ModuleSymbolIdentity,
        public: bool,
        consumer: &ModuleSymbolIdentity,
    ) -> Result<&'a neutral_ir::expression::project::ExpressionDeclaration, E> {
        self.budget.symbol(owner)?;
        self.budget.step(
            (owner.module().module_name().len() + owner.declaration_name().len() + 1) as u64
                * (u64::from(self.ir.declarations.len().bit_width()) + 1),
        )?;
        let index = self
            .ir
            .declarations
            .binary_search_by(|d| d.identity.cmp(owner))
            .map_err(|_| E::UnknownOrPrivate)?;
        let d = &self.ir.declarations[index];
        if !matches!(d.signature, S::Binding(_))
            || (!d.public && (public || owner.module() != consumer.module()))
        {
            return Err(E::UnknownOrPrivate);
        }
        Ok(d)
    }
    /// Checks every wrapper/nominal and its public/cross-module visibility before comparison.
    pub(super) fn ty(
        &mut self,
        ty: &T,
        public: bool,
        consumer: &ModuleSymbolIdentity,
        depth: u64,
    ) -> Result<(), E> {
        self.budget.step(1)?;
        if depth
            > self
                .limits
                .composition
                .type_depth
                .min(neutral_ir::project::PROJECT_MAX_DEPTH as u64)
        {
            return Err(E::Limit);
        }
        match ty {
            T::List(t) | T::Nullable(t) | T::Ref(t) => self.ty(t, public, consumer, depth + 1),
            T::Nominal(_) | T::VocabularyNominal { .. } => {
                self.nominal(ty, public, consumer)?;
                Ok(())
            }
            _ => Ok(()),
        }
    }
    /// Resolves a complete contract body from independently validated scope authority.
    pub(super) fn nominal(
        &mut self,
        ty: &T,
        public: bool,
        consumer: &ModuleSymbolIdentity,
    ) -> Result<&'a B, E> {
        match ty {
            T::Nominal(owner) => {
                self.budget.symbol(owner)?;
                self.budget.step(self.scope.sources().len() as u64 + 1)?;
                let definition = self
                    .scope
                    .sources()
                    .iter()
                    .find(|d| &d.owner == owner)
                    .ok_or(E::UnknownOrPrivate)?;
                if !definition.definition.public && (public || owner.module() != consumer.module())
                {
                    return Err(E::UnknownOrPrivate);
                }
                Ok(&definition.definition.body)
            }
            T::VocabularyNominal {
                identity,
                version,
                name,
            } => {
                self.budget.text(identity)?;
                self.budget.text(version)?;
                self.budget.text(name)?;
                self.budget
                    .step(self.scope.catalogue().bundles().len() as u64 + 1)?;
                let bundle = self
                    .scope
                    .catalogue()
                    .bundles()
                    .iter()
                    .find(|b| b.identity.identity() == identity && b.identity.version() == version)
                    .ok_or(E::UnknownOrPrivate)?;
                self.budget.step(bundle.definitions.len() as u64 + 1)?;
                let definition = bundle
                    .definitions
                    .iter()
                    .find(|d| &d.name == name)
                    .ok_or(E::UnknownOrPrivate)?;
                if !definition.public {
                    return Err(E::UnknownOrPrivate);
                }
                Ok(&definition.body)
            }
            _ => Err(E::TypeMismatch),
        }
    }
    /// Accumulates exact UTF-8 tree text; schema labels outside the tree do not contribute.
    pub(super) fn wire_text(&mut self, text: &str) -> Result<(), E> {
        self.budget.text(text)?;
        self.facts.retained_expression_text_bytes = self
            .facts
            .retained_expression_text_bytes
            .checked_add(text.len() as u64)
            .ok_or(E::Limit)?;
        Ok(())
    }
    /// Counts exact symbol text in wire tree occurrences.
    pub(super) fn wire_symbol(&mut self, symbol: &ModuleSymbolIdentity) -> Result<(), E> {
        self.wire_text(symbol.module().language_behavior_version())?;
        self.wire_text(symbol.module().module_name())?;
        self.wire_text(symbol.declaration_name())
    }
    /// Counts all frozen type tags/owners, not a source-facing pretty representation.
    pub(super) fn wire_type(&mut self, ty: &T) -> Result<(), E> {
        match ty {
            T::Num => self.wire_text(language::NUM),
            T::String => self.wire_text(language::STRING),
            T::Bool => self.wire_text(language::BOOL),
            T::Url => self.wire_text(language::URL),
            T::Path => self.wire_text(language::PATH),
            T::List(t) | T::Nullable(t) | T::Ref(t) => {
                self.wire_text(match ty {
                    T::List(_) => language::LIST,
                    T::Ref(_) => language::REF_TYPE,
                    _ => "nullable",
                })?;
                self.wire_type(t)
            }
            T::Nominal(s) => {
                self.wire_text("nominal")?;
                self.wire_symbol(s)
            }
            T::VocabularyNominal {
                identity,
                version,
                name,
            } => {
                self.wire_text("vocabulary")?;
                self.wire_text(identity)?;
                self.wire_text(version)?;
                self.wire_text(name)
            }
        }
    }
}
