// SPDX-License-Identifier: Apache-2.0

//! Allocation-free structural preflight; never grants semantic or public project authority.

use super::{
    Expression, ExpressionNode as N, MatchArm, SelectorSegment, profile::ExpressionLimits,
};
use crate::{ModuleSymbolIdentity, project_interface::ProjectPublicType as T};
use neutral_core::CancellationToken;
mod payload;

/// Safe structural failures with no supplied names, values or source paths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpressionShapeError {
    /// Noncanonical raw collection or impossible closed shape.
    Malformed,
    /// Independent positive count/depth/work/storage or allocation failure.
    Limit,
    /// Request cancelled before publication.
    Cancelled,
}
use ExpressionShapeError as E;

/// Isolated tree counts; not complete project resources or proof of semantic typing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExpressionShape {
    /// All retained occurrences, including unselected branches.
    pub nodes: u64,
    /// Maximum expression depth, with leaves at one.
    pub depth: u64,
    /// Potential output occurrence count, not scheduler edges.
    pub output_leaves: u64,
}

/// Allocation-free cost projection for callers sharing one budget across multiple trees.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExpressionInspection {
    /// Isolated residual-tree shape; empty for materialized-body inspection.
    pub shape: ExpressionShape,
    /// Checked owned model storage, excluding allocator metadata.
    pub retained_bytes: u64,
    /// Work charged by structural preflight, including conservative copy cost.
    pub work: u64,
}

/// Inspects every retained branch and nested payload without allocation or evaluation.
///
/// # Errors
/// Rejects noncanonical structures, independent limits or cancellation. Exact
/// nominal typing, binder scope, visibility and producer association require a reader.
pub fn inspect_expression_shape(
    expression: &Expression,
    limits: ExpressionLimits,
    cancellation: &CancellationToken,
) -> Result<ExpressionShape, E> {
    inspect_expression_retention(expression, limits, cancellation).map(|result| result.shape)
}

/// Returns isolated shape and retention costs for aggregate caller accounting.
///
/// Each call starts with the supplied remaining limits. When inspecting several
/// bodies, debit the returned work/storage before the next call; this function
/// does not mutate an external request budget or count complete project companions.
///
/// # Errors
/// Rejects the same structural, resource and cancellation failures as shape inspection.
pub fn inspect_expression_retention(
    expression: &Expression,
    limits: ExpressionLimits,
    cancellation: &CancellationToken,
) -> Result<ExpressionInspection, E> {
    inspect_retention(limits, cancellation, |state| {
        state.expression(expression, 1)
    })
}

/// Bounds a materialized body using the same deep-payload accounting as residual literals.
///
/// Counts owned value slots and text but no residual expression nodes. The returned
/// `shape` is empty; nominal typing and field restrictions remain validator-owned.
///
/// # Errors
/// Rejects noncanonical collections, independent limits and cancellation; does not establish typing.
pub fn inspect_expression_value_retention(
    value: &crate::composition::BindingValue,
    limits: ExpressionLimits,
    cancellation: &CancellationToken,
) -> Result<ExpressionInspection, E> {
    inspect_retention(limits, cancellation, |state| {
        state.storage::<crate::composition::BindingValue>(1)?;
        state.value(value, 1)
    })
}

/// Shares request-local preflight setup without constructing synthetic expression nodes.
fn inspect_retention(
    limits: ExpressionLimits,
    cancellation: &CancellationToken,
    inspect: impl FnOnce(&mut Inspection<'_>) -> Result<(), E>,
) -> Result<ExpressionInspection, E> {
    if cancellation.is_cancelled() {
        return Err(E::Cancelled);
    }
    let limits = limits.intersect(ExpressionLimits::hard());
    if limits.values().contains(&0) {
        return Err(E::Limit);
    }
    let mut state = Inspection {
        limits,
        cancellation,
        work: limits.work,
        bytes: limits.retained_bytes.min(limits.output_bytes),
        edges: limits.dependency_edges,
        shape: ExpressionShape {
            nodes: 0,
            depth: 0,
            output_leaves: 0,
        },
    };
    inspect(&mut state)?;
    Ok(ExpressionInspection {
        shape: state.shape,
        retained_bytes: limits.retained_bytes.min(limits.output_bytes) - state.bytes,
        work: limits.work - state.work,
    })
}

/// Request-local counters, shared across every descendant and deep payload.
struct Inspection<'a> {
    /// Independently intersected controls.
    limits: ExpressionLimits,
    /// Explicit cooperative signal.
    cancellation: &'a CancellationToken,
    /// Remaining work, including conservative copy cost.
    work: u64,
    /// Remaining owned model storage, not allocator overhead or wire bytes.
    bytes: u64,
    /// Remaining potential input/output occurrences.
    edges: u64,
    /// Accumulated raw tree shape.
    shape: ExpressionShape,
}
impl Inspection<'_> {
    /// Charges traversal and both inspection/copy byte work before retention.
    fn charge(&mut self, bytes: u64) -> Result<(), E> {
        if self.cancellation.is_cancelled() {
            return Err(E::Cancelled);
        }
        self.work = self
            .work
            .checked_sub(
                bytes
                    .checked_mul(2)
                    .and_then(|n| n.checked_add(1))
                    .ok_or(E::Limit)?,
            )
            .ok_or(E::Limit)?;
        self.bytes = self.bytes.checked_sub(bytes).ok_or(E::Limit)?;
        Ok(())
    }
    /// Bounds every recursive structural descent before calling it.
    fn structural_depth(&mut self, depth: u64) -> Result<(), E> {
        self.charge(0)?;
        if depth > self.limits.syntax_depth {
            return Err(E::Limit);
        }
        Ok(())
    }
    /// Counts exact owned slots with checked multiplication.
    fn storage<T>(&mut self, count: usize) -> Result<(), E> {
        self.charge(
            (count as u64)
                .checked_mul(std::mem::size_of::<T>() as u64)
                .ok_or(E::Limit)?,
        )
    }
    /// Counts exact UTF-8 storage; no normalization or string allocation.
    fn text(&mut self, text: &str) -> Result<(), E> {
        self.charge(text.len() as u64)
    }
    /// Counts the complete stable owner without a source alias or host path.
    fn symbol(&mut self, symbol: &ModuleSymbolIdentity) -> Result<(), E> {
        self.text(symbol.module().language_behavior_version())?;
        self.text(symbol.module().module_name())?;
        self.text(symbol.declaration_name())
    }
    /// Bounds wrapper recursion separately from expression evaluation depth.
    fn ty(&mut self, ty: &T, depth: u64) -> Result<(), E> {
        self.structural_depth(depth)?;
        match ty {
            T::List(t) | T::Nullable(t) | T::Ref(t) => {
                self.storage::<T>(1)?;
                self.ty(t, depth + 1)
            }
            T::Nominal(s) => self.symbol(s),
            T::VocabularyNominal {
                identity,
                version,
                name,
            } => {
                self.text(identity)?;
                self.text(version)?;
                self.text(name)
            }
            _ => Ok(()),
        }
    }
    /// Charges a potential value edge without scheduling or dereferencing it.
    fn edge(&mut self) -> Result<(), E> {
        self.charge(0)?;
        self.edges = self.edges.checked_sub(1).ok_or(E::Limit)?;
        Ok(())
    }
    /// Visits the complete expression tree, never selecting a branch.
    fn expression(&mut self, expression: &Expression, depth: u64) -> Result<(), E> {
        self.structural_depth(depth)?;
        if depth > self.limits.evaluation_depth || self.shape.nodes >= self.limits.nodes {
            return Err(E::Limit);
        }
        self.shape.nodes += 1;
        self.shape.depth = self.shape.depth.max(depth);
        self.storage::<Expression>(1)?;
        self.ty(&expression.ty, 1)?;
        self.node(&expression.node, depth + 1)
    }
    /// Visits payloads and children in retained semantic order.
    fn node(&mut self, node: &N, depth: u64) -> Result<(), E> {
        match node {
            N::Value(v) => self.value(v, 1),
            N::Input(s) => {
                self.edge()?;
                self.symbol(s)
            }
            N::Output {
                target,
                result_type,
                selector,
            } => {
                self.edge()?;
                self.shape.output_leaves += 1;
                self.symbol(target)?;
                self.ty(result_type, 1)?;
                if selector.len() > super::profile::MAX_SELECTOR_SEGMENTS {
                    return Err(E::Limit);
                }
                self.storage::<SelectorSegment>(selector.len())?;
                for segment in selector {
                    match segment {
                        SelectorSegment::Field(s) | SelectorSegment::Alternative(s) => {
                            self.text(s)?;
                        }
                        SelectorSegment::Index(_) => self.charge(0)?,
                    }
                }
                Ok(())
            }
            N::Unary { operand, .. } | N::Text(operand) => self.expression(operand, depth),
            N::Binary { left, right, .. } => {
                self.expression(left, depth)?;
                self.expression(right, depth)
            }
            N::If {
                guard,
                then_branch,
                else_branch,
            } => {
                self.expression(guard, depth)?;
                self.expression(then_branch, depth)?;
                self.expression(else_branch, depth)
            }
            N::Match { scrutinee, arms } => {
                self.expression(scrutinee, depth)?;
                self.arms(arms, depth)
            }
            N::Interpolate(items) | N::List(items) => {
                for item in items {
                    self.expression(item, depth)?;
                }
                Ok(())
            }
            N::Record(fields) => {
                let mut previous: Option<&str> = None;
                for (name, value) in fields {
                    self.charge(
                        (std::mem::size_of::<(String, Option<Expression>)>()
                            - std::mem::size_of::<Expression>()) as u64,
                    )?;
                    self.name(name, &mut previous)?;
                    if let Some(value) = value {
                        self.expression(value, depth)?;
                    } else {
                        self.storage::<Expression>(1)?;
                    }
                }
                Ok(())
            }
            N::Variant { tag, payload } => {
                self.text(tag)?;
                self.expression(payload, depth)
            }
            N::Bound { .. } => self.charge(0),
        }
    }
    /// Retains canonical names without sorting hostile inputs or granting nominal authority.
    fn name<'a>(&mut self, name: &'a str, previous: &mut Option<&'a str>) -> Result<(), E> {
        self.text(name)?;
        if previous.is_some_and(|p| p >= name) {
            return Err(E::Malformed);
        }
        *previous = Some(name);
        Ok(())
    }
    /// Counts arm metadata and every retained body, including unselected arms.
    fn arms(&mut self, arms: &[MatchArm], depth: u64) -> Result<(), E> {
        let mut previous = None;
        for arm in arms {
            self.charge(
                (std::mem::size_of::<MatchArm>() - std::mem::size_of::<Expression>()) as u64,
            )?;
            self.name(&arm.tag, &mut previous)?;
            self.ty(&arm.payload_type, 1)?;
            self.expression(&arm.body, depth)?;
        }
        Ok(())
    }
}
