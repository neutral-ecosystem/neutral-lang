// SPDX-License-Identifier: Apache-2.0

//! Derived guarded potential value edges; traversal never evaluates guards or schedules work.

use super::{ExpressionReadError as E, ValidatedExpressionProject};
use neutral_core::{
    CancellationToken,
    allocation::{RetainCapacity, copy_slice},
};
use neutral_ir::{
    ModuleSymbolIdentity,
    expression::{BinaryOperator as O, Expression, ExpressionBody, ExpressionNode as N, profile},
};
use neutral_semantics::{BindingKey, ExpressionRoot};

/// Canonical branch condition, referencing typed guard data rather than executable text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GuardCondition<'a> {
    /// Required Boolean result of the guard at its canonical node site.
    Boolean(bool),
    /// Required exact variant tag at the canonical scrutinee site.
    Alternative(&'a str),
}
/// One canonical guard site in the consuming expression root.
#[derive(Debug, Eq, PartialEq)]
pub struct DependencyGuard<'a> {
    /// Canonical root-local guard/scrutinee node path.
    pub node_path: Vec<u64>,
    /// Selected branch condition, not an instruction to execute a consumer operation.
    pub condition: GuardCondition<'a>,
}
/// A potential exact port at a consuming site; repeated keys retain distinct guarded sites.
#[derive(Debug, Eq, PartialEq)]
pub struct ExpressionDependency<'a> {
    /// Public ordinary binding or non-callable template owner.
    pub owner: &'a ModuleSymbolIdentity,
    /// Separates ordinary body, template body and parameter defaults.
    pub root: ExpressionRoot,
    /// Canonical consuming node occurrence.
    pub node_path: Vec<u64>,
    /// Exact typed external input/output key.
    pub key: BindingKey<'a>,
    /// Ordered enclosing branch requirements; not explicit scheduling prerequisites.
    pub guards: Vec<DependencyGuard<'a>>,
}
/// Independently bounded potential dependency inspection.
#[derive(Clone, Copy, Debug)]
pub struct ExpressionInspectionLimits {
    /// Potential edge occurrences, never deduplicated by key alone.
    pub edges: u64,
    /// Aggregate traversal/comparison/retention work.
    pub work: u64,
    /// Aggregate owned path/guard/edge output, excluding borrowed authority.
    pub output_bytes: u64,
}
impl Default for ExpressionInspectionLimits {
    /// Uses frozen hard ceilings; callers may reduce every dimension independently.
    fn default() -> Self {
        Self {
            edges: profile::MAX_ITEMS,
            work: profile::MAX_WORK,
            output_bytes: profile::MAX_MODEL_BYTES,
        }
    }
}
/// Complete guarded occurrence information, separate from structural refs and prerequisites.
#[derive(Debug, Eq, PartialEq)]
pub struct GuardedDependencies<'a> {
    /// Complete canonical retained occurrence list.
    edges: Vec<ExpressionDependency<'a>>,
}
impl<'a> GuardedDependencies<'a> {
    /// Borrows every potential public port including unselected branches.
    #[must_use]
    pub fn edges(&self) -> &[ExpressionDependency<'a>] {
        &self.edges
    }
}

/// Request-local closure traversal with shared work/storage accounting.
struct Walk<'a, 'c> {
    /// Public consuming owner.
    owner: &'a ModuleSymbolIdentity,
    /// Current retained root discriminator.
    root: ExpressionRoot,
    /// Current canonical node occurrence path.
    path: Vec<u64>,
    /// Ordered enclosing guard facts.
    guards: Vec<DependencyGuard<'a>>,
    /// Complete resulting occurrences.
    edges: Vec<ExpressionDependency<'a>>,
    /// Remaining independent work budget.
    work: u64,
    /// Remaining independent owned storage budget.
    bytes: u64,
    /// Maximum retained edge occurrences.
    edge_limit: u64,
    /// Cooperative caller signal.
    cancel: &'c CancellationToken,
}
impl<'a> ValidatedExpressionProject {
    /// Derives all potential guarded ports without evaluating or interpreting ordinary prerequisite fields.
    /// # Errors
    /// Rejects missing/private owners, independent controls, allocation or cancellation without partial output.
    pub fn dependencies(
        &'a self,
        owner: &'a ModuleSymbolIdentity,
        limits: ExpressionInspectionLimits,
        cancel: &CancellationToken,
    ) -> Result<GuardedDependencies<'a>, E> {
        let accepted = self.model().limits().expression;
        let mut walk = Walk {
            owner,
            root: ExpressionRoot::Binding,
            path: Vec::new(),
            guards: Vec::new(),
            edges: Vec::new(),
            work: limits.work.min(accepted.work),
            bytes: limits.output_bytes.min(accepted.retained_bytes),
            edge_limit: limits.edges.min(accepted.dependency_edges),
            cancel,
        };
        if walk.work == 0 || walk.bytes == 0 || walk.edge_limit == 0 {
            return Err(E::Limit);
        }
        walk.step(
            owner.declaration_name().len() as u64
                + self.ir().declarations.len() as u64
                + self.ir().public_templates.len() as u64
                + 1,
        )?;
        if let Ok(index) = self
            .ir()
            .declarations
            .binary_search_by(|d| d.identity.cmp(owner))
        {
            let d = &self.ir().declarations[index];
            if !d.public {
                return Err(E::Semantic);
            }
            walk.owner = &d.identity;
            match &d.body {
                Some(ExpressionBody::Expression(e)) => walk.tree(e)?,
                Some(ExpressionBody::Input) => walk.emit(BindingKey::Input(&d.identity))?,
                Some(ExpressionBody::Value(_)) | None => {}
            }
        } else if let Ok(index) = self
            .ir()
            .public_templates
            .binary_search_by(|t| t.identity.cmp(owner))
        {
            let template = &self.ir().public_templates[index];
            walk.owner = &template.identity;
            walk.root = ExpressionRoot::TemplateBody;
            walk.tree(&template.body)?;
            for (index, parameter) in template.parameters.iter().enumerate() {
                if let Some(default) = &parameter.default {
                    walk.root = ExpressionRoot::TemplateDefault(index);
                    walk.tree(default)?;
                }
            }
        } else {
            return Err(E::Semantic);
        }
        walk.step(0)?;
        Ok(GuardedDependencies { edges: walk.edges })
    }
}
impl<'a> Walk<'a, '_> {
    /// Charges work before traversal/allocation and observes cancellation.
    fn step(&mut self, n: u64) -> Result<(), E> {
        if self.cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        self.work = self.work.checked_sub(n).ok_or(E::Limit)?;
        Ok(())
    }
    /// Charges model storage before a fallible retention operation.
    fn storage(&mut self, n: u64) -> Result<(), E> {
        self.step(n.checked_add(1).ok_or(E::Limit)?)?;
        self.bytes = self.bytes.checked_sub(n).ok_or(E::Limit)?;
        Ok(())
    }
    /// Emits a complete independently accounted occurrence with its distinct guard chain.
    fn emit(&mut self, key: BindingKey<'a>) -> Result<(), E> {
        if self.edges.len() as u64 >= self.edge_limit {
            return Err(E::Limit);
        }
        self.storage(std::mem::size_of::<ExpressionDependency<'_>>() as u64)?;
        self.storage(self.path.len() as u64 * 8)?;
        let path = copy_slice(&self.path).map_err(|_| E::Limit)?;
        let mut guards = Vec::new();
        self.storage(
            (self.guards.len() as u64)
                .checked_mul(std::mem::size_of::<DependencyGuard<'_>>() as u64)
                .ok_or(E::Limit)?,
        )?;
        guards
            .try_retain_exact(self.guards.len())
            .map_err(|_| E::Limit)?;
        for index in 0..self.guards.len() {
            self.storage(self.guards[index].node_path.len() as u64 * 8)?;
            guards.push(DependencyGuard {
                node_path: copy_slice(&self.guards[index].node_path).map_err(|_| E::Limit)?,
                condition: self.guards[index].condition,
            });
        }
        self.edges.try_retain_exact(1).map_err(|_| E::Limit)?;
        self.edges.push(ExpressionDependency {
            owner: self.owner,
            root: self.root,
            node_path: path,
            key,
            guards,
        });
        self.step(0)
    }
    /// Visits a canonical child while retaining bounded request-local path state.
    fn child(&mut self, expression: &'a Expression, index: u64) -> Result<(), E> {
        self.storage(8)?;
        self.path.try_retain_exact(1).map_err(|_| E::Limit)?;
        self.path.push(index);
        let result = self.tree(expression);
        self.path.pop();
        result
    }
    /// Adds a guard tied to its typed guard/scrutinee site before visiting a branch.
    fn guarded(
        &mut self,
        expression: &'a Expression,
        child: u64,
        guard_child: u64,
        condition: GuardCondition<'a>,
    ) -> Result<(), E> {
        self.storage(
            (self.path.len() as u64 + 1) * 8 + std::mem::size_of::<DependencyGuard<'_>>() as u64,
        )?;
        let mut path = copy_slice(&self.path).map_err(|_| E::Limit)?;
        path.try_retain_exact(1).map_err(|_| E::Limit)?;
        path.push(guard_child);
        self.guards.try_retain_exact(1).map_err(|_| E::Limit)?;
        self.guards.push(DependencyGuard {
            node_path: path,
            condition,
        });
        let result = self.child(expression, child);
        self.guards.pop();
        result
    }
    /// Enumerates every typed potential leaf, never selecting or evaluating a branch.
    fn tree(&mut self, e: &'a Expression) -> Result<(), E> {
        self.step(1)?;
        match &e.node {
            N::Input(owner) => self.emit(BindingKey::Input(owner))?,
            N::Output {
                target,
                result_type,
                selector,
            } => self.emit(BindingKey::Output {
                target,
                result_type,
                selector,
            })?,
            N::Unary { operand, .. } | N::Text(operand) => self.child(operand, 0)?,
            N::Binary {
                operator,
                left,
                right,
            } => {
                self.child(left, 0)?;
                match operator {
                    O::And => self.guarded(right, 1, 0, GuardCondition::Boolean(true))?,
                    O::Or => self.guarded(right, 1, 0, GuardCondition::Boolean(false))?,
                    _ => self.child(right, 1)?,
                }
            }
            N::If {
                guard,
                then_branch,
                else_branch,
            } => {
                self.child(guard, 0)?;
                self.guarded(then_branch, 1, 0, GuardCondition::Boolean(true))?;
                self.guarded(else_branch, 2, 0, GuardCondition::Boolean(false))?;
            }
            N::Match { scrutinee, arms } => {
                self.child(scrutinee, 0)?;
                for (index, arm) in arms.iter().enumerate() {
                    self.guarded(
                        &arm.body,
                        index as u64 + 1,
                        0,
                        GuardCondition::Alternative(&arm.tag),
                    )?;
                }
            }
            N::List(items) | N::Interpolate(items) => {
                for (index, item) in items.iter().enumerate() {
                    self.child(item, index as u64)?;
                }
            }
            N::Record(fields) => {
                for (index, (_, e)) in fields.iter().enumerate() {
                    if let Some(e) = e {
                        self.child(e, index as u64)?;
                    }
                }
            }
            N::Variant { payload, .. } => self.child(payload, 0)?,
            N::Value(_) | N::Bound { .. } => {}
        }
        Ok(())
    }
}
