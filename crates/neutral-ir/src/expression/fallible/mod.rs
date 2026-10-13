// SPDX-License-Identifier: Apache-2.0

//! Preflighted owned retention; no unbounded allocating Clone implementation on raw trees.

use super::{
    Expression, ExpressionNode as N, ExpressionShapeError as E, MatchArm, SelectorSegment as S,
    inspect_expression_shape, profile::ExpressionLimits,
};
use neutral_core::{
    CancellationToken,
    allocation::{AllocationError, RetainCapacity, TryClone, boxed, text},
};
mod value;

impl Expression {
    /// Copies every retained branch only after allocation-free independent bound checks.
    ///
    /// # Errors
    /// Returns structural, limit/allocation or cancellation failure with no partial copy.
    /// Successful copying is not semantic type, visibility or binder validation.
    pub fn try_clone_bounded(
        &self,
        limits: ExpressionLimits,
        cancel: &CancellationToken,
    ) -> Result<Self, E> {
        inspect_expression_shape(self, limits, cancel)?;
        copy(self, cancel)
    }
}

/// Observes cancellation even when it arrives at the final allocation checkpoint.
fn check<T>(result: Result<T, AllocationError>, cancel: &CancellationToken) -> Result<T, E> {
    if cancel.is_cancelled() {
        return Err(E::Cancelled);
    }
    result.map_err(|_| E::Limit)
}

/// Retains exact vector capacity and checks cancellation before each child and publication.
fn sequence<T, U>(
    items: &[T],
    cancel: &CancellationToken,
    mut retain: impl FnMut(&T) -> Result<U, E>,
) -> Result<Vec<U>, E> {
    let mut result = Vec::new();
    check(result.try_retain_exact(items.len()), cancel)?;
    for item in items {
        if cancel.is_cancelled() {
            return Err(E::Cancelled);
        }
        result.push(retain(item)?);
    }
    if cancel.is_cancelled() {
        return Err(E::Cancelled);
    }
    Ok(result)
}

/// Uses the already checked depth and owned-byte bound; never callable without preflight.
fn copy(expression: &Expression, cancel: &CancellationToken) -> Result<Expression, E> {
    if cancel.is_cancelled() {
        return Err(E::Cancelled);
    }
    let ty = check(expression.ty.try_clone(), cancel)?;
    let node = node(&expression.node, cancel)?;
    if cancel.is_cancelled() {
        return Err(E::Cancelled);
    }
    Ok(Expression { ty, node })
}

/// Retains a checked child in fallible boxed storage.
fn child(expression: &Expression, cancel: &CancellationToken) -> Result<Box<Expression>, E> {
    check(boxed(copy(expression, cancel)?), cancel)
}

/// Copies closed raw shapes without selecting, reordering, folding or dereferencing.
fn node(node: &N, cancel: &CancellationToken) -> Result<N, E> {
    Ok(match node {
        N::Value(literal) => N::Value(value::copy(literal, cancel)?),
        N::Input(symbol) => N::Input(check(symbol.try_clone(), cancel)?),
        N::Output {
            target,
            result_type,
            selector,
        } => N::Output {
            target: check(target.try_clone(), cancel)?,
            result_type: check(result_type.try_clone(), cancel)?,
            selector: sequence(selector, cancel, |segment| {
                Ok(match segment {
                    S::Field(name) => S::Field(check(text(name), cancel)?),
                    S::Index(index) => S::Index(*index),
                    S::Alternative(tag) => S::Alternative(check(text(tag), cancel)?),
                })
            })?,
        },
        N::Unary { operator, operand } => N::Unary {
            operator: *operator,
            operand: child(operand, cancel)?,
        },
        N::Binary {
            operator,
            left,
            right,
        } => N::Binary {
            operator: *operator,
            left: child(left, cancel)?,
            right: child(right, cancel)?,
        },
        N::If {
            guard,
            then_branch,
            else_branch,
        } => N::If {
            guard: child(guard, cancel)?,
            then_branch: child(then_branch, cancel)?,
            else_branch: child(else_branch, cancel)?,
        },
        N::Match { scrutinee, arms } => N::Match {
            scrutinee: child(scrutinee, cancel)?,
            arms: sequence(arms, cancel, |arm| {
                Ok(MatchArm {
                    tag: check(text(&arm.tag), cancel)?,
                    payload_type: check(arm.payload_type.try_clone(), cancel)?,
                    body: copy(&arm.body, cancel)?,
                })
            })?,
        },
        N::Text(operand) => N::Text(child(operand, cancel)?),
        N::Interpolate(items) => {
            N::Interpolate(sequence(items, cancel, |item| copy(item, cancel))?)
        }
        N::List(items) => N::List(sequence(items, cancel, |item| copy(item, cancel))?),
        N::Record(fields) => N::Record(sequence(fields, cancel, |(name, value)| {
            Ok((
                check(text(name), cancel)?,
                value.as_ref().map(|v| copy(v, cancel)).transpose()?,
            ))
        })?),
        N::Variant { tag, payload } => N::Variant {
            tag: check(text(tag), cancel)?,
            payload: child(payload, cancel)?,
        },
        N::Bound { depth, slot } => N::Bound {
            depth: *depth,
            slot: *slot,
        },
    })
}
