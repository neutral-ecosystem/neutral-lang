// SPDX-License-Identifier: Apache-2.0

//! Known ports and complete explicit envelope validation, independent of selected-path demands.

use super::{Budget, EvaluationError as E, Flow, State, T, V};
use neutral_ir::{
    ModuleSymbolIdentity,
    composition::project::CompositionSignature as S,
    expression::{
        Expression, ExpressionBody, ExpressionNode as N, SelectorSegment,
        profile::MAX_SELECTOR_SEGMENTS,
    },
};

/// An exact public input or typed future-output selector; never a host locator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingKey<'a> {
    /// Stable declared external input symbol.
    Input(&'a ModuleSymbolIdentity),
    /// Exact target/result contract and canonical type-mode selector.
    Output {
        /// Public nominal operation/data binding; core does not assert it executes anything.
        target: &'a ModuleSymbolIdentity,
        /// Exact nominal public result contract selected by the source intrinsic.
        result_type: &'a T,
        /// Ordered field/index/alternative selection into that contract.
        selector: &'a [SelectorSegment],
    },
}
/// Explicit states; missing is represented by absence from the envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingState<'a> {
    /// Borrowed supplied data, independently checked before evaluation.
    Value(&'a V),
    /// Complete typed output root; checked restrictions/selection are not caller assertions.
    OutputResult(&'a V),
    /// A known typed binding is not available; no implicit waiting.
    Unavailable,
    /// A known typed binding failed; no backend exception text is retained.
    Failed,
}
/// One complete supplied key/type/state envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Binding<'a> {
    /// Exact known model port.
    pub key: BindingKey<'a>,
    /// Claimed exact result type, checked against the independently typed port.
    pub ty: &'a T,
    /// Explicit available/unavailable/failed state.
    pub state: BindingState<'a>,
}
/// Consumer-owned producer/result association, not core authorization or execution evidence.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutputAssociation<'a> {
    /// Known public target of a retained output intrinsic.
    pub target: &'a ModuleSymbolIdentity,
    /// Known exact public nominal result contract for that target.
    pub result_type: &'a T,
}
/// Independently typed canonical public model port.
#[derive(Clone, Copy)]
pub(super) struct Port<'m> {
    /// Exact public model key.
    pub(super) key: BindingKey<'m>,
    /// Independently resolved selected value type.
    pub(super) ty: &'m T,
    /// Conservative bounded comparison work.
    pub(super) cost: u64,
}

/// Charges bounded borrowed UTF-8 before comparisons or envelope scanning.
fn text(value: &str, budget: &mut Budget<'_>) -> Result<(), E> {
    budget.storage(value.len() as u64)?;
    Ok(())
}
/// Bounds every stable symbol component before a potentially expensive comparison.
pub(super) fn symbol(value: &ModuleSymbolIdentity, budget: &mut Budget<'_>) -> Result<(), E> {
    text(value.module().language_behavior_version(), budget)?;
    text(value.module().module_name(), budget)?;
    text(value.declaration_name(), budget)
}
/// Preflights type depth and owned key bytes without resolving untrusted nominal owners.
fn type_envelope(value: &T, budget: &mut Budget<'_>) -> Result<(), E> {
    let cost = neutral_ir::expression::inspect_expression_type_retention(
        value,
        budget.input_limits(),
        budget.cancellation(),
    )
    .map_err(crate::budget::shape_error)?;
    budget.step(cost.work)?;
    budget.storage(cost.retained_bytes)?;
    Ok(())
}
/// Returns charged key-comparison work after bounded input inspection.
fn key_envelope(key: BindingKey<'_>, budget: &mut Budget<'_>) -> Result<u64, E> {
    let before = budget.remaining().work;
    match key {
        BindingKey::Input(owner) => symbol(owner, budget)?,
        BindingKey::Output {
            target,
            result_type,
            selector,
        } => {
            symbol(target, budget)?;
            type_envelope(result_type, budget)?;
            if selector.len() > MAX_SELECTOR_SEGMENTS {
                return Err(E::Limit);
            }
            budget.storage(
                (selector.len() as u64)
                    .checked_mul(std::mem::size_of::<SelectorSegment>() as u64)
                    .ok_or(E::Limit)?,
            )?;
            for segment in selector {
                budget.step(1)?;
                if let SelectorSegment::Field(name) | SelectorSegment::Alternative(name) = segment {
                    text(name, budget)?;
                }
            }
        }
    }
    Ok((before - budget.remaining().work).max(1))
}

impl<'m> State<'m, '_> {
    /// Registers complete public input/output contracts, not values or scheduler edges.
    pub(super) fn registry(&mut self) -> Result<(), E> {
        for declaration in &self.model.ir.declarations {
            self.budget.step(1)?;
            if self.mode == super::Mode::Consumer && !declaration.public {
                continue;
            }
            let S::Binding(ty) = &declaration.signature else {
                continue;
            };
            match &declaration.body {
                Some(ExpressionBody::Input) => {
                    self.port(BindingKey::Input(&declaration.identity), ty)?;
                }
                Some(ExpressionBody::Expression(expression)) => self.ports_in(expression)?,
                _ => {}
            }
        }
        Ok(())
    }
    /// Collects every potential leaf, including unselected guards, without evaluation.
    fn ports_in(&mut self, expression: &'m Expression) -> Result<(), E> {
        self.budget.step(1)?;
        match &expression.node {
            N::Output {
                target,
                result_type,
                selector,
            } => self.port(
                BindingKey::Output {
                    target,
                    result_type,
                    selector,
                },
                &expression.ty,
            )?,
            N::Unary { operand, .. } | N::Text(operand) => self.ports_in(operand)?,
            N::Binary { left, right, .. } => {
                self.ports_in(left)?;
                self.ports_in(right)?;
            }
            N::If {
                guard,
                then_branch,
                else_branch,
            } => {
                self.ports_in(guard)?;
                self.ports_in(then_branch)?;
                self.ports_in(else_branch)?;
            }
            N::Match { scrutinee, arms } => {
                self.ports_in(scrutinee)?;
                for arm in arms {
                    self.ports_in(&arm.body)?;
                }
            }
            N::List(items) | N::Interpolate(items) => {
                for item in items {
                    self.ports_in(item)?;
                }
            }
            N::Record(fields) => {
                for (_, value) in fields {
                    if let Some(value) = value {
                        self.ports_in(value)?;
                    }
                }
            }
            N::Variant { payload, .. } => self.ports_in(payload)?,
            _ => {}
        }
        Ok(())
    }
    /// Deduplicates a canonical model port after paying worst-case comparison work.
    fn port(&mut self, key: BindingKey<'m>, ty: &'m T) -> Result<(), E> {
        let cost = key_envelope(key, &mut self.budget)?;
        self.budget.step(
            cost.checked_mul(self.ports.len() as u64 + 1)
                .ok_or(E::Limit)?,
        )?;
        if self.ports.iter().any(|p| p.key == key) {
            return Ok(());
        }
        if self.ports.len() as u64 >= self.budget.limits.bindings {
            return Err(E::Limit);
        }
        self.budget.reserve(&mut self.ports, 1)?;
        self.ports.push(Port { key, ty, cost });
        Ok(())
    }
    /// Validates even unused supplied envelopes before evaluating any guard or arithmetic.
    pub(super) fn envelopes(&mut self) -> Result<(), E> {
        if self.request.bindings.len() as u64 > self.budget.limits.bindings
            || self.request.associations.len() as u64 > self.budget.limits.bindings
        {
            return Err(E::Limit);
        }
        for (index, association) in self.request.associations.iter().enumerate() {
            symbol(association.target, &mut self.budget)?;
            type_envelope(association.result_type, &mut self.budget)?;
            self.budget.step(
                (self.ports.iter().map(|p| p.cost).max().unwrap_or(1))
                    .checked_mul(self.ports.len() as u64 + index as u64 + 1)
                    .ok_or(E::Limit)?,
            )?;
            if !self.ports.iter().any(|p| matches!(p.key, BindingKey::Output { target, result_type, .. } if target == association.target && result_type == association.result_type))
                || self.request.associations[..index].iter().any(|a| a == association)
            { return Err(E::InvalidBinding); }
        }
        for (index, binding) in self.request.bindings.iter().enumerate() {
            let cost = key_envelope(binding.key, &mut self.budget)?;
            type_envelope(binding.ty, &mut self.budget)?;
            self.budget.step(
                cost.checked_mul(self.ports.len() as u64 + index as u64 + 1)
                    .ok_or(E::Limit)?,
            )?;
            let port = *self
                .ports
                .iter()
                .find(|p| p.key == binding.key)
                .ok_or(E::InvalidBinding)?;
            if self.request.bindings[..index]
                .iter()
                .any(|b| b.key == binding.key)
            {
                return Err(E::InvalidBinding);
            }
            if binding.ty != port.ty || !self.associated(binding.key)? {
                return Err(E::BindingType);
            }
            match (port.key, binding.state) {
                (BindingKey::Input(owner), BindingState::Value(value)) => {
                    self.checked(value, port.ty, owner)?;
                }
                (
                    BindingKey::Output {
                        target,
                        result_type,
                        selector,
                    },
                    BindingState::OutputResult(root),
                ) => {
                    self.checked(root, result_type, target)?;
                    let selected = super::selectors::select(root, selector, &mut self.budget)?;
                    self.checked(selected, port.ty, target)?;
                }
                (_, BindingState::Unavailable | BindingState::Failed) => {}
                _ => return Err(E::BindingType),
            }
        }
        Ok(())
    }
    /// Checks consumer-owned association only for supplied or actually demanded outputs.
    fn associated(&mut self, key: BindingKey<'_>) -> Result<bool, E> {
        let BindingKey::Output {
            target,
            result_type,
            ..
        } = key
        else {
            return Ok(true);
        };
        self.budget.step(
            (self.ports.iter().map(|p| p.cost).max().unwrap_or(1))
                .checked_mul(self.request.associations.len() as u64 + 1)
                .ok_or(E::Limit)?,
        )?;
        Ok(self
            .request
            .associations
            .iter()
            .any(|a| a.target == target && a.result_type == result_type))
    }
    /// Demands one known-public port without waiting, scheduling or substituting null.
    pub(super) fn demand(&mut self, key: BindingKey<'m>) -> Result<Option<V>, Flow<'m>> {
        let cost = key_envelope(key, &mut self.budget)?;
        if self.mode == super::Mode::Consumer && !self.associated(key)? {
            return Err(E::BindingType.into());
        }
        self.budget.step(
            cost.checked_mul(self.request.bindings.len() as u64 + 1)
                .ok_or(E::Limit)?,
        )?;
        if let Some(binding) = self.request.bindings.iter().find(|b| b.key == key) {
            return match binding.state {
                BindingState::Value(value) => Ok(Some(self.budget.copy_value(value)?)),
                BindingState::OutputResult(root) => {
                    let BindingKey::Output { selector, .. } = key else {
                        return Err(E::BindingType.into());
                    };
                    let selected = super::selectors::select(root, selector, &mut self.budget)?;
                    Ok(Some(self.budget.copy_value(selected)?))
                }
                BindingState::Unavailable => Err(Flow::Unavailable(key)),
                BindingState::Failed => Err(E::BindingFailed.into()),
            };
        }
        // Folding retains input leaves, not a consumer demand report for every repeated visit.
        if self.mode == super::Mode::Producer {
            return Ok(None);
        }
        if self.missing.len() as u64 >= self.budget.limits.dependency_edges {
            return Err(E::Limit.into());
        }
        self.budget.reserve(&mut self.missing, 1)?;
        self.missing.push(key);
        Ok(None)
    }
}
