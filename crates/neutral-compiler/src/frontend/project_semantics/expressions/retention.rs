// SPDX-License-Identifier: Apache-2.0

//! Every typed retained copy pays aggregate work/storage before allocating.
//!
//! Centralizes source-root, token, type, symbol, definition and value copying so
//! call sites cannot accidentally pay only for an outer handle while retaining a
//! large owned subtree. Borrowed lexemes still require vector slots; root copies
//! also retain their type syntax, member names and any decoded token contents.
//!
//! Shared IR inspectors preflight recursive values/contracts without granting
//! semantic authority. Their modeled storage and conservative traversal/copy work
//! debit the persistent compiler budget before fallible retention. Expression
//! copies preserve all branches and use cancellation-aware bounded entry points;
//! numeric normalization, default materialization and evaluation happen elsewhere.
//! Resource mapping preserves cancellation and the tighter active expansion policy.

use super::{Budget, E, Expression, ModuleSymbolIdentity, T, TryClone, V, alloc, error};
use crate::frontend::expression_codes as ec;
use neutral_ir::expression::{ExpressionInspection, ExpressionShapeError};

impl Budget<'_> {
    /// Parsed syntax copies pay their owned slots and spellings before cloning.
    pub(super) fn copy_syntax_type(
        &mut self,
        ty: &crate::frontend::project_semantics::TypeExpr,
    ) -> Result<crate::frontend::project_semantics::TypeExpr, E> {
        self.charge_syntax_type(ty)?;
        alloc(ty.try_clone())
    }

    /// Pays every owned type layer and nominal spelling without allocating.
    fn charge_syntax_type(
        &mut self,
        ty: &crate::frontend::project_semantics::TypeExpr,
    ) -> Result<(), E> {
        use crate::frontend::project_semantics::TypeExpr;
        self.retain::<TypeExpr>(1)?;
        match ty {
            TypeExpr::Nominal(alias, name, _) => {
                self.bytes(name.len() as u64)?;
                if let Some(alias) = alias {
                    self.bytes(alias.len() as u64)?;
                }
            }
            TypeExpr::List(inner) | TypeExpr::Ref(inner) | TypeExpr::Nullable(inner) => {
                self.charge_syntax_type(inner)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Root retention includes tokens and types, not only its identity header.
    pub(super) fn copy_root(&mut self, root: &super::Root) -> Result<super::Root, E> {
        use crate::frontend::TokenKind;
        self.retain::<super::Root>(1)?;
        self.bytes(
            (root.owner.module().language_behavior_version().len()
                + root.owner.module().module_name().len()
                + root.owner.declaration_name().len()) as u64,
        )?;
        if let Some(ty) = &root.ty {
            self.charge_syntax_type(ty)?;
        }
        self.retain::<(String, crate::frontend::project_semantics::TypeExpr)>(root.fields.len())?;
        for (name, ty) in &root.fields {
            self.bytes(name.len() as u64)?;
            self.charge_syntax_type(ty)?;
        }
        self.retain::<crate::frontend::Token>(root.tokens.len())?;
        for token in &root.tokens {
            match &token.kind {
                TokenKind::Identifier(s) | TokenKind::ProtectedName(s) | TokenKind::Number(s) => {
                    self.bytes(s.len() as u64)?;
                }
                TokenKind::StringLiteral(s) => self.bytes(s.value.len() as u64)?,
                _ => {}
            }
        }
        alloc(root.try_clone())
    }

    /// Lexemes borrow spellings but their copied vector still consumes storage.
    pub(super) fn copy_tokens<'a>(
        &mut self,
        tokens: &[super::Lex<'a>],
    ) -> Result<Vec<super::Lex<'a>>, E> {
        self.retain::<super::Lex<'a>>(tokens.len())?;
        alloc(super::copy_slice(tokens))
    }
    /// Cancellation keeps its own diagnostic when a nested retention boundary fails.
    pub(super) fn resource_error(&self) -> E {
        error(
            if self.cancel.is_cancelled() {
                ec::EXP_CANCELLED
            } else if self.expanding
                && (self.expansion.work.saturating_sub(self.expansion_work)
                    < self.expression.work.saturating_sub(self.work)
                    || self
                        .expansion
                        .retained_bytes
                        .saturating_sub(self.expansion_retained)
                        < self.expression.retained_bytes.saturating_sub(self.bytes))
            {
                ec::TPL_LIMIT
            } else {
                ec::EXP_LIMIT
            },
            None,
        )
    }
    /// Canonical source definitions reuse the raw contract inspector before fallible retention.
    pub(super) fn copy_definition(
        &mut self,
        body: &neutral_ir::composition::CompositionBody,
        project: neutral_ir::project::ProjectLimits,
        mut policy: neutral_ir::composition::project::CompositionPolicy,
    ) -> Result<neutral_ir::composition::CompositionBody, E> {
        policy.work = policy.work.min(self.remaining().work);
        let inspected = neutral_ir::composition::project::inspect_composition_bodies(
            [body],
            project,
            policy,
            self.cancel,
        )
        .map_err(|_| self.resource_error())?;
        self.charge(
            inspected
                .work
                .checked_mul(2)
                .ok_or_else(|| error(ec::EXP_LIMIT, None))?,
        )?;
        self.storage(inspected.retained_bytes)?;
        alloc(body.try_clone())
    }
    /// Text retention and copying pay proportional UTF-8 work before allocation.
    pub(super) fn copy_text(&mut self, value: &str) -> Result<String, E> {
        self.bytes(value.len() as u64)?;
        alloc(neutral_core::allocation::text(value))
    }
    /// Stable identity copies retain only their actual text, never host paths.
    pub(super) fn copy_symbol(
        &mut self,
        owner: &ModuleSymbolIdentity,
    ) -> Result<ModuleSymbolIdentity, E> {
        self.bytes(
            (owner.module().language_behavior_version().len()
                + owner.module().module_name().len()
                + owner.declaration_name().len()) as u64,
        )?;
        alloc(owner.try_clone())
    }

    /// Shared borrowed inspection charges both traversal and the ensuing bounded clone.
    fn pay(&mut self, cost: Result<ExpressionInspection, ExpressionShapeError>) -> Result<(), E> {
        let cost = cost.map_err(|_| self.resource_error())?;
        // Cover inspection and ensuing retention work; a copy is not free just because shape was checked.
        self.charge(
            cost.work
                .checked_mul(2)
                .ok_or_else(|| error(ec::EXP_LIMIT, None))?,
        )?;
        self.storage(cost.retained_bytes)
    }

    /// Copies all branches without resetting remaining request counters.
    pub(super) fn copy_expression(&mut self, expression: &Expression) -> Result<Expression, E> {
        // Keep the same preflight policy for the bounded clone while debiting aggregate counters here.
        let limits = self.remaining();
        self.pay(neutral_ir::expression::inspect_expression_retention(
            expression,
            limits,
            self.cancel,
        ))?;
        expression
            .try_clone_bounded(limits, self.cancel)
            .map_err(|_| self.resource_error())
    }

    /// Copies complete materialized payloads under the same independent bounds.
    pub(super) fn copy_value(&mut self, value: &V) -> Result<V, E> {
        let limits = self.remaining();
        self.pay(neutral_ir::expression::inspect_expression_value_retention(
            value,
            limits,
            self.cancel,
        ))?;
        neutral_ir::expression::try_clone_value_bounded(value, limits, self.cancel)
            .map_err(|_| self.resource_error())
    }

    /// Exact public type annotations share key, nesting and storage accounting.
    pub(super) fn copy_type(&mut self, ty: &T) -> Result<T, E> {
        self.pay(neutral_ir::expression::inspect_expression_type_retention(
            ty,
            self.remaining(),
            self.cancel,
        ))?;
        alloc(ty.try_clone())
    }
}
