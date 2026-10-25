// SPDX-License-Identifier: Apache-2.0

//! Complete explicit expression transport; no source activation or evaluation.

use crate::{
    DecodeError as D, DecodeErrorClass as C, DecodeLimits, EncodingError as E, cbor::CborWriter,
    primitives::failure,
};
use neutral_core::{
    CancellationToken,
    allocation::{RetainCapacity, Shared},
};
use neutral_ir::expression::profile;
use neutral_reader::{ExpressionReadError, SemanticLimits, ValidatedExpressionProject};

mod companions;
mod project;
mod trees;

#[cfg(test)]
#[path = "../../tests/expression/mod.rs"]
mod tests;

/// Encodes the complete independently validated `/3` authority.
///
/// # Errors
/// Returns bounded size, allocation or cancellation failure with no partial artifact.
pub fn encode_expression_project(
    project: &ValidatedExpressionProject,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, E> {
    let ir = project.ir();
    let mut writer = CborWriter::with_cancellation(cancel);
    project::write_ir(&mut writer, ir)?;
    let payload = writer.finish()?;
    crate::decoder::validate_section(&payload, DecodeLimits::hard(), cancel)
        .map_err(validation_error)?;
    let length = profile::MAGIC
        .len()
        .checked_add(payload.len())
        .ok_or(E::EncodedSizeLimit)?;
    if length as u64
        > ir.limits
            .artifact_bytes
            .min(project.model().limits().project.artifact_bytes)
    {
        return Err(E::EncodedSizeLimit);
    }
    let mut bytes = Vec::new();
    bytes.try_retain_exact(length).map_err(|_| {
        if cancel.is_cancelled() {
            E::Cancelled
        } else {
            E::EncodedSizeLimit
        }
    })?;
    bytes.extend_from_slice(&profile::MAGIC);
    bytes.extend(payload);
    if cancel.is_cancelled() {
        return Err(E::Cancelled);
    }
    Ok(bytes)
}

/// Reconstructs all 22 sections, then independently validates before publication.
///
/// # Errors
/// Rejects malformed/unsupported tuples, stale annotations/facts/companions,
/// retention limits and cancellation atomically, with no historical-profile fallback.
pub fn decode_expression_project(
    bytes: &[u8],
    wire: DecodeLimits,
    semantic: SemanticLimits,
    cancel: &CancellationToken,
) -> Result<ValidatedExpressionProject, D> {
    let result = decode_complete(bytes, wire, &semantic, cancel);
    if cancel.is_cancelled() {
        Err(failure(C::Cancelled))
    } else {
        result
    }
}

/// Keeps reconstruction private until the independent reader grants complete authority.
fn decode_complete(
    bytes: &[u8],
    wire: DecodeLimits,
    semantic: &SemanticLimits,
    cancel: &CancellationToken,
) -> Result<ValidatedExpressionProject, D> {
    check_cancelled(cancel)?;
    if bytes.len() > wire.maximum_artifact_bytes()
        || bytes.len() as u64 > semantic.project.artifact_bytes
    {
        return Err(failure(C::EncodedSizeLimit));
    }
    let payload = bytes
        .strip_prefix(&profile::MAGIC)
        .ok_or_else(|| failure(C::MalformedFrame))?;
    if payload.len() > wire.maximum_section_bytes() {
        return Err(failure(C::EncodedSizeLimit));
    }
    if semantic.expression.values().contains(&0)
        || semantic.expansion.values().contains(&0)
        || [
            semantic.project.modules,
            semantic.project.declarations,
            semantic.project.import_edges,
            semantic.project.nodes,
            semantic.project.text_bytes,
            semantic.project.artifact_bytes,
        ]
        .contains(&0)
        || semantic.composition.validate().is_err()
    {
        return Err(failure(C::EncodedSizeLimit));
    }
    let parsed =
        crate::decoder::parse_canonical_section(payload, profile::MAGIC.len(), wire, cancel)?;
    let ir = project::read_ir(&parsed, cancel)?;
    drop(parsed);
    if bytes.len() as u64 > ir.limits.artifact_bytes {
        return Err(failure(C::EncodedSizeLimit));
    }
    check_cancelled(cancel)?;
    let ir = Shared::try_new(ir).map_err(|_| failure(C::EncodedSizeLimit))?;
    ValidatedExpressionProject::from_ir(ir, *semantic, cancel).map_err(|error| {
        failure(match error {
            ExpressionReadError::Cancelled => C::Cancelled,
            ExpressionReadError::Limit => C::EncodedSizeLimit,
            ExpressionReadError::Companion => C::InvalidProvenance,
            ExpressionReadError::Schema => C::UnsupportedVersion,
            _ => C::InvalidLogicalIr,
        })
    })
}

/// Observes cancellation between reconstructing complete sections and retained children.
fn check_cancelled(cancel: &CancellationToken) -> Result<(), D> {
    if cancel.is_cancelled() {
        Err(failure(C::Cancelled))
    } else {
        Ok(())
    }
}

/// Maps a post-write lexical validation error to the stable encoder class.
fn validation_error(error: D) -> E {
    if error.class() == C::Cancelled {
        E::Cancelled
    } else {
        E::EncodedSizeLimit
    }
}

/// Checks cancellation at each reconstructed collection item without widening shared `/2` grammar.
fn read_list<T>(
    value: &crate::decoder::LocatedValue,
    cancel: &CancellationToken,
    mut item: impl FnMut(&crate::decoder::LocatedValue) -> Result<T, D>,
) -> Result<Vec<T>, D> {
    check_cancelled(cancel)?;
    crate::primitives::read_list(value, |value| {
        check_cancelled(cancel)?;
        item(value)
    })
}
