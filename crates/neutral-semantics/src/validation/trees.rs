// SPDX-License-Identifier: Apache-2.0

//! Retypes all retained branches and derives unresolved selector/field obligations without evaluation.

use super::{B, E, S, State};
use crate::model::{
    AvailabilityObligation, AvailabilityRequirement as A, ExpressionSite, RestrictionObligation,
};
use neutral_core::allocation::{RetainCapacity, TryClone, text};
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::{FieldPresence, FieldRestrictions},
    expression::{Expression, ExpressionNode as N},
    project_interface::ProjectPublicType as T,
};

impl<'a> State<'a> {
    /// Visits a canonical child occurrence while sharing work/path retention limits.
    pub(super) fn child(
        &mut self,
        e: &'a Expression,
        index: u64,
        public: bool,
        owner: &ModuleSymbolIdentity,
        depth: u64,
    ) -> Result<(), E> {
        self.budget.step(1)?;
        self.path.try_retain_exact(1).map_err(|_| E::Limit)?;
        self.path.push(index);
        let result = self.tree(e, public, owner, depth);
        self.path.pop();
        result
    }
    /// Counts and independently retypes every descendant, including unselected branches.
    pub(super) fn tree(
        &mut self,
        e: &'a Expression,
        public: bool,
        owner: &ModuleSymbolIdentity,
        depth: u64,
    ) -> Result<(), E> {
        self.budget.step(1)?;
        if depth > self.limits.expression.evaluation_depth
            || self.facts.residual_nodes >= self.limits.expression.nodes
        {
            return Err(E::Limit);
        }
        self.facts.residual_nodes += 1;
        self.facts.residual_depth = self.facts.residual_depth.max(depth);
        self.ty(&e.ty, public, owner, 0)?;
        self.wire_text(e.node.kind())?;
        self.wire_type(&e.ty)?;
        match &e.node {
            N::Value(v) => self.value(v, &e.ty, public, owner, true, 0),
            N::Input(target) => {
                self.edge()?;
                self.wire_symbol(target)?;
                let d = self.binding(target, public, owner)?;
                if !matches!(&d.body, Some(neutral_ir::expression::ExpressionBody::Input))
                    || !matches!(&d.signature, S::Binding(ty) if ty == &e.ty)
                {
                    return Err(E::TypeMismatch);
                }
                Ok(())
            }
            N::Output {
                target,
                result_type,
                selector,
            } => self.output(e, target, result_type, selector, owner),
            N::Unary { operator, operand } => {
                self.unary(e, *operator, operand, public, owner, depth)
            }
            N::Binary {
                operator,
                left,
                right,
            } => self.binary(e, *operator, (left, right), public, owner, depth),
            N::If {
                guard,
                then_branch,
                else_branch,
            } => {
                self.child(guard, 0, public, owner, depth + 1)?;
                self.child(then_branch, 1, public, owner, depth + 1)?;
                self.child(else_branch, 2, public, owner, depth + 1)?;
                if guard.ty != T::Bool || then_branch.ty != e.ty || else_branch.ty != e.ty {
                    return Err(E::TypeMismatch);
                }
                Ok(())
            }
            N::Match { scrutinee, arms } => self.matching(e, scrutinee, arms, public, owner, depth),
            N::Text(operand) => {
                self.child(operand, 0, public, owner, depth + 1)?;
                if e.ty != T::String || !matches!(operand.ty, T::Num | T::Bool | T::String) {
                    return Err(E::TypeMismatch);
                }
                Ok(())
            }
            N::Interpolate(items) | N::List(items) => {
                let expected = match (&e.node, &e.ty) {
                    (N::Interpolate(_), T::String) => &T::String,
                    (N::List(_), T::List(t)) => t.as_ref(),
                    (N::List(_), T::Nullable(t)) => match t.as_ref() {
                        T::List(inner) => inner.as_ref(),
                        _ => return Err(E::TypeMismatch),
                    },
                    _ => return Err(E::TypeMismatch),
                };
                for (index, item) in items.iter().enumerate() {
                    self.child(item, index as u64, public, owner, depth + 1)?;
                    if &item.ty != expected {
                        return Err(E::TypeMismatch);
                    }
                }
                Ok(())
            }
            N::Record(values) => self.record(e, values, public, owner, depth),
            N::Variant { tag, payload } => {
                self.wire_text(tag)?;
                let nominal = match &e.ty {
                    T::Nullable(inner) => inner.as_ref(),
                    ty => ty,
                };
                let B::Variant(alternatives) = self.nominal(nominal, public, owner)? else {
                    return Err(E::TypeMismatch);
                };
                self.budget.step(alternatives.len() as u64 + 1)?;
                let alternative = alternatives
                    .iter()
                    .find(|a| &a.tag == tag)
                    .ok_or(E::MatchAlternatives)?;
                self.child(payload, 0, public, owner, depth + 1)?;
                if payload.ty != alternative.ty {
                    return Err(E::TypeMismatch);
                }
                Ok(())
            }
            N::Bound { depth, slot } => self.bound(*depth, *slot, &e.ty),
        }
    }
    /// Rejects orphan, oversized or differently typed payload/parameter lexical references.
    fn bound(&self, depth: u64, slot: u64, ty: &T) -> Result<(), E> {
        let depth = usize::try_from(depth).map_err(|_| E::TypeMismatch)?;
        let index = self
            .frame_count
            .checked_sub(depth.checked_add(1).ok_or(E::TypeMismatch)?)
            .ok_or(E::TypeMismatch)?;
        let slot = usize::try_from(slot).map_err(|_| E::TypeMismatch)?;
        let resolved = match self.frames[index] {
            Some(super::Frame::Payload(ty)) if slot == 0 => Some(ty),
            Some(super::Frame::Parameters(parameters)) => parameters.get(slot).map(|p| &p.ty),
            _ => None,
        };
        if resolved != Some(ty) {
            return Err(E::TypeMismatch);
        }
        Ok(())
    }
    /// Tracks potential leaf occurrences separately from structural references or scheduling data.
    pub(super) fn edge(&mut self) -> Result<(), E> {
        self.budget.step(1)?;
        if self.facts.potential_value_edges >= self.limits.expression.dependency_edges {
            return Err(E::Limit);
        }
        self.facts.potential_value_edges += 1;
        Ok(())
    }
    /// Retains a checked site only after count/work/storage controls have passed.
    fn site(&mut self, owner: &ModuleSymbolIdentity) -> Result<ExpressionSite, E> {
        self.budget.symbol(owner)?;
        let bytes = (owner.module().language_behavior_version().len()
            + owner.module().module_name().len()
            + owner.declaration_name().len()) as u64
            + self.path.len() as u64 * 8;
        self.budget.storage(bytes)?;
        Ok(ExpressionSite {
            owner: owner.try_clone().map_err(|_| E::Limit)?,
            root: self.root,
            node_path: self.path.try_clone().map_err(|_| E::Limit)?,
        })
    }
    /// Retains one future selector requirement; never attempts to acquire the output.
    pub(super) fn available(
        &mut self,
        owner: &ModuleSymbolIdentity,
        segment: usize,
        requirement: A,
    ) -> Result<(), E> {
        self.budget
            .storage(std::mem::size_of::<AvailabilityObligation>() as u64)?;
        let site = self.site(owner)?;
        self.availability
            .try_retain_exact(1)
            .map_err(|_| E::Limit)?;
        self.availability.push(AvailabilityObligation {
            site,
            segment,
            requirement,
        });
        Ok(())
    }
    /// Checks record expression fields and retains real restriction obligations on computed children.
    fn record(
        &mut self,
        e: &'a Expression,
        values: &'a [(String, Option<Expression>)],
        public: bool,
        owner: &ModuleSymbolIdentity,
        depth: u64,
    ) -> Result<(), E> {
        let nominal = match &e.ty {
            T::Nullable(inner) => inner.as_ref(),
            ty => ty,
        };
        let B::Record(fields) = self.nominal(nominal, public, owner)? else {
            return Err(E::TypeMismatch);
        };
        if fields.len() != values.len() {
            return Err(E::TypeMismatch);
        }
        for (index, (field, (name, value))) in fields.iter().zip(values).enumerate() {
            self.wire_text(name)?;
            if name != &field.name {
                return Err(E::TypeMismatch);
            }
            let Some(value) = value else {
                if field.presence != FieldPresence::Optional {
                    return Err(E::TypeMismatch);
                }
                self.wire_text("omitted")?;
                continue;
            };
            self.child(value, index as u64, public, owner, depth + 1)?;
            if value.ty != field.ty {
                return Err(E::TypeMismatch);
            }
            if field.restrictions != FieldRestrictions::default() {
                if let N::Value(v) = &value.node {
                    self.scope
                        .check_field_restrictions(
                            nominal,
                            &field.name,
                            v,
                            self.limits.composition,
                            &mut self.budget.work,
                            self.budget.cancel,
                        )
                        .map_err(E::from)?;
                } else {
                    self.budget.storage(
                        std::mem::size_of::<RestrictionObligation>() as u64
                            + field.name.len() as u64,
                    )?;
                    self.path.try_retain_exact(1).map_err(|_| E::Limit)?;
                    self.path.push(index as u64);
                    let site = self.site(owner)?;
                    self.path.pop();
                    self.restrictions
                        .try_retain_exact(1)
                        .map_err(|_| E::Limit)?;
                    self.restrictions.push(RestrictionObligation {
                        site,
                        contract: nominal.try_clone().map_err(|_| E::Limit)?,
                        field: text(&field.name).map_err(|_| E::Limit)?,
                    });
                }
            }
        }
        Ok(())
    }
}
