// SPDX-License-Identifier: Apache-2.0

//! Original immutable-copy occurrences survive normalization; computations and ports remain separate.

use super::{
    E, Edge, ExpressionBody, ModuleSymbolIdentity, ProjectProvenance, Root, SourceLocation, State,
    V, alloc, composition_error, copy_slice, error, lowering::edge,
};
use crate::frontend::expression_codes as ec;

impl State<'_, '_> {
    /// Retains original immutable-copy occurrences, not arithmetic or deferred value dependencies.
    pub(super) fn source_reuse_edges(
        &mut self,
        owner: &ModuleSymbolIdentity,
        value: &V,
        edges: &mut Vec<ProjectProvenance>,
    ) -> Result<(), E> {
        let Some(syntax) = self.bindings.get(owner) else {
            return Ok(());
        };
        // Generated expressions have definition/application lineage, not caller-scope source names.
        if !syntax.axes.is_empty() {
            return Ok(());
        }
        self.budget.retain::<super::Lex<'_>>(syntax.tokens.len())?;
        let tokens = alloc(copy_slice(&syntax.tokens))?;
        let root = self.budget.copy_root(&self.resolver.roots[owner])?;
        self.reuse_occurrences(&root, &tokens, value, edges, 1)
    }

    /// Follows only retained construction positions; erased computations never invent reuse edges.
    fn reuse_occurrences(
        &mut self,
        root: &Root,
        tokens: &[super::Lex<'_>],
        value: &V,
        edges: &mut Vec<ProjectProvenance>,
        depth: u64,
    ) -> Result<(), E> {
        self.budget.charge(tokens.len() as u64 + 1)?;
        if depth > self.budget.expression.syntax_depth {
            return Err(error(ec::EXP_LIMIT, Some(root.location)));
        }
        if tokens.first().is_some_and(|t| t.spelling == "(") {
            let mut cursor = 0;
            let inner = super::parser::group(tokens, &mut cursor, "(", ")")?;
            if cursor == tokens.len() {
                return self.reuse_occurrences(root, inner, value, edges, depth + 1);
            }
        }
        let mut cursor = 0;
        if let Ok((alias, name, span)) = super::parser::name(tokens, &mut cursor)
            && cursor == tokens.len()
            && !matches!(name, "true" | "false" | "null")
        {
            let target = self
                .resolver
                .name(root, alias, name, span, false)
                .map_err(composition_error)?;
            if matches!(self.bodies.get(&target), Some(ExpressionBody::Value(_))) {
                self.budget.retain::<ProjectProvenance>(1)?;
                for owner in [&root.owner, &target] {
                    self.budget.bytes(
                        (owner.module().language_behavior_version().len()
                            + owner.module().module_name().len()
                            + owner.declaration_name().len()) as u64,
                    )?;
                }
                edge(
                    &mut self.budget,
                    &root.owner,
                    &target,
                    Edge::Value,
                    SourceLocation::new(root.location.source(), span),
                    edges,
                )?;
            }
            return Ok(());
        }
        let (open, close) = match value {
            V::List(_) => ("[", "]"),
            V::Record(_) | V::Variant { .. } => ("{", "}"),
            _ => return Ok(()),
        };
        if tokens.first().is_none_or(|t| t.spelling != open) {
            return Ok(());
        }
        let mut cursor = 0;
        let inner = super::parser::group(tokens, &mut cursor, open, close)?;
        if cursor != tokens.len() {
            return Ok(());
        }
        self.budget.retain::<&[super::Lex<'_>]>(inner.len())?;
        let parts = super::parser::split(inner)?;
        match value {
            V::List(values) => {
                for (tokens, value) in parts.iter().zip(values) {
                    self.reuse_occurrences(root, tokens, value, edges, depth + 1)?;
                }
            }
            V::Record(fields) => {
                for (name, value) in fields {
                    self.budget
                        .charge(parts.len() as u64 + name.len() as u64 + 1)?;
                    if let Some(value) = value
                        && let Some(part) = parts
                            .iter()
                            .find(|p| p.first().is_some_and(|t| t.spelling == name))
                    {
                        self.reuse_occurrences(root, &part[2..], value, edges, depth + 1)?;
                    }
                }
            }
            V::Variant { payload, .. } => {
                if let Some(part) = parts
                    .iter()
                    .find(|p| p.first().is_some_and(|t| t.spelling == "payload"))
                {
                    self.reuse_occurrences(root, &part[2..], payload, edges, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
