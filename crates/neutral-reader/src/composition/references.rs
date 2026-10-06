// SPDX-License-Identifier: Apache-2.0

//! Bounded, canonical reference-type inspection over already validated public contracts.

use super::CompositionLookupError;
use neutral_core::CancellationToken;
use neutral_ir::{
    composition::{CompositionBody, CompositionDefinition},
    project_interface::ProjectPublicType as T,
};
use neutral_vocabulary::composition::{MAX_DEPTH, MAX_WORK};

/// Independent per-definition bounds, not semantic field or list-length restrictions.
#[derive(Clone, Copy, Debug)]
pub struct CompositionInspectionLimits {
    /// Maximum inspected type nodes and path-copy/key bytes.
    pub visits: u64,
    /// Maximum emitted reference-type occurrences.
    pub references: u64,
    /// Maximum field/alternative/composed-type path depth.
    pub depth: u64,
}

/// Safe public inspection failures with no captured text or private definition details.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionReferenceError {
    /// Existing public owner/type lookup classification.
    Lookup(CompositionLookupError),
    /// Zero independent caller budget.
    InvalidLimits,
    /// Caller or hard traversal/reference/depth ceiling exceeded.
    Limit,
    /// Cooperative cancellation before publishing any partial enumeration.
    Cancelled,
    /// Bounded fallible path/result reservation failed.
    Allocation,
}

/// A canonical type-contract path, distinct from source/value/host paths.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReferenceTypeSegment<'a> {
    /// Record field, including optional/defaulted fields.
    Field(&'a str),
    /// Closed variant alternative, including unselected branches.
    Alternative(&'a str),
    /// Nested ordered-list element type.
    ListElement,
    /// Nested nullable inner type.
    NullableInner,
}

/// One reference type occurrence with an exact public nominal target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReferenceTypeDependency<'a> {
    /// Canonical field/tag/wrapper path within the selected public definition.
    pub path: Vec<ReferenceTypeSegment<'a>>,
    /// Complete alias-independent target type, not a binding/value reference edge.
    pub target: &'a T,
}

/// Request-local counters shared by every branch of one definition traversal.
struct Inspector<'a> {
    /// Complete independent caller policy.
    limits: CompositionInspectionLimits,
    /// Explicit cooperative cancellation signal.
    cancellation: &'a CancellationToken,
    /// Charged traversal/path-copy work.
    visits: u64,
}

impl Inspector<'_> {
    /// Observes cancellation and charges work before traversal or proportional allocation.
    fn step(&mut self, amount: u64) -> Result<(), CompositionReferenceError> {
        if self.cancellation.is_cancelled() {
            return Err(CompositionReferenceError::Cancelled);
        }
        self.visits = self
            .visits
            .checked_add(amount)
            .ok_or(CompositionReferenceError::Limit)?;
        if self.visits > self.limits.visits.min(MAX_WORK) {
            return Err(CompositionReferenceError::Limit);
        }
        Ok(())
    }

    /// Walks list/nullable wrappers but emits rather than expands a nominal reference target.
    fn walk<'a>(
        &mut self,
        ty: &'a T,
        path: &mut Vec<ReferenceTypeSegment<'a>>,
        result: &mut Vec<ReferenceTypeDependency<'a>>,
    ) -> Result<(), CompositionReferenceError> {
        self.step(1)?;
        if path.len() as u64 > self.limits.depth.min(MAX_DEPTH) {
            return Err(CompositionReferenceError::Limit);
        }
        match ty {
            T::Ref(target) => {
                if result.len() as u64 >= self.limits.references.min(MAX_WORK) {
                    return Err(CompositionReferenceError::Limit);
                }
                let bytes = path.iter().try_fold(1_u64, |total, segment| {
                    total
                        .checked_add(match segment {
                            ReferenceTypeSegment::Field(name)
                            | ReferenceTypeSegment::Alternative(name) => name.len() as u64,
                            _ => 1,
                        })
                        .ok_or(CompositionReferenceError::Limit)
                })?;
                self.step(bytes)?;
                let mut copied = Vec::new();
                copied
                    .try_reserve_exact(path.len())
                    .map_err(|_| CompositionReferenceError::Allocation)?;
                copied.extend_from_slice(path);
                result
                    .try_reserve(1)
                    .map_err(|_| CompositionReferenceError::Allocation)?;
                result.push(ReferenceTypeDependency {
                    path: copied,
                    target,
                });
            }
            T::List(inner) | T::Nullable(inner) => {
                path.try_reserve(1)
                    .map_err(|_| CompositionReferenceError::Allocation)?;
                path.push(if matches!(ty, T::List(_)) {
                    ReferenceTypeSegment::ListElement
                } else {
                    ReferenceTypeSegment::NullableInner
                });
                let outcome = self.walk(inner, path, result);
                path.pop();
                outcome?;
            }
            _ => {}
        }
        Ok(())
    }
}

/// Enumerates direct declared reference contracts atomically in validated canonical field/tag order.
pub(super) fn inspect<'a>(
    definition: &'a CompositionDefinition,
    limits: CompositionInspectionLimits,
    cancellation: &CancellationToken,
) -> Result<Vec<ReferenceTypeDependency<'a>>, CompositionReferenceError> {
    if [limits.visits, limits.references, limits.depth].contains(&0) {
        return Err(CompositionReferenceError::InvalidLimits);
    }
    let mut inspector = Inspector {
        limits,
        cancellation,
        visits: 0,
    };
    let mut result = Vec::new();
    let mut path = Vec::new();
    path.try_reserve(1)
        .map_err(|_| CompositionReferenceError::Allocation)?;
    inspector.step(1)?;
    match &definition.body {
        CompositionBody::Record(fields) => {
            for field in fields {
                path.push(ReferenceTypeSegment::Field(&field.name));
                inspector.walk(&field.ty, &mut path, &mut result)?;
                path.pop();
            }
        }
        CompositionBody::Variant(alternatives) => {
            for alternative in alternatives {
                path.push(ReferenceTypeSegment::Alternative(&alternative.tag));
                inspector.walk(&alternative.ty, &mut path, &mut result)?;
                path.pop();
            }
        }
    }
    inspector.step(1)?;
    Ok(result)
}
