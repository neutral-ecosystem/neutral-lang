// SPDX-License-Identifier: Apache-2.0

//! Derive the capability bits required by the document's actual contents.
//!
//! Type and value walks share a node/depth budget so capability inspection is
//! bounded. Both encoding and independent decoding use this calculation to
//! compare declared capabilities with observed features; header bits alone are
//! not evidence that an artifact's contents are supported.

use super::{CompilationArtifacts, EncodingError, LogicalValue, ResolvedType, constants};

/// Tracks the hard recursive traversal ceiling while inspecting capabilities.
pub(crate) struct TraversalBudget {
    /// Logical nodes visited so far.
    nodes: usize,
}

impl TraversalBudget {
    /// Creates an empty traversal budget.
    pub(crate) const fn new() -> Self {
        Self { nodes: 0 }
    }

    /// Accounts for one node and validates its nesting depth.
    pub(crate) fn visit(&mut self, depth: usize) -> Result<(), EncodingError> {
        if depth > constants::MAXIMUM_NESTING_DEPTH {
            return Err(EncodingError::EncodedSizeLimit);
        }
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or(EncodingError::EncodedSizeLimit)?;
        if self.nodes > constants::MAXIMUM_TRAVERSAL_NODES {
            return Err(EncodingError::EncodedSizeLimit);
        }
        Ok(())
    }
}

/// Derives the exact capability mask from validated content.
pub(crate) fn derive_capabilities(
    artifacts: &CompilationArtifacts,
    budget: &mut TraversalBudget,
) -> Result<u64, EncodingError> {
    let document = artifacts.logical_document();
    let mut mask = 0_u64;
    if document.vocabulary().is_some() {
        mask |= capability(5);
    }
    for record in document.record_types() {
        mask |= capability(2);
        for field in record.fields() {
            collect_type_capabilities(field.resolved_type(), 1, budget, &mut mask)?;
            if let Some(value) = field.default_value() {
                collect_value_capabilities(value, 1, budget, &mut mask)?;
            }
        }
    }
    if let Some(vocabulary) = document.vocabulary() {
        for contract in vocabulary.types() {
            for field in contract.fields() {
                collect_type_capabilities(field.resolved_type(), 1, budget, &mut mask)?;
                if let Some(value) = field.default_value() {
                    collect_value_capabilities(value, 1, budget, &mut mask)?;
                }
            }
        }
    }
    for declaration in document.declarations() {
        collect_type_capabilities(declaration.resolved_type(), 1, budget, &mut mask)?;
        collect_value_capabilities(declaration.value(), 1, budget, &mut mask)?;
    }
    if !artifacts.reference_provenance().is_empty() {
        mask |= capability(4);
    }
    if !artifacts.reuse_provenance().is_empty() {
        mask |= capability(6);
    }
    if artifacts.field_provenance().iter().any(|record| {
        matches!(
            record.origin(),
            neutral_ir::ValueOrigin::UserRecordDefault | neutral_ir::ValueOrigin::VocabularyDefault
        )
    }) {
        mask |= capability(7);
    }
    Ok(mask)
}

/// Returns one capability bit.
pub(crate) const fn capability(bit: u32) -> u64 {
    1_u64 << bit
}

/// Collects capability bits from one resolved type.
fn collect_type_capabilities(
    resolved_type: &ResolvedType,
    depth: usize,
    budget: &mut TraversalBudget,
    mask: &mut u64,
) -> Result<(), EncodingError> {
    budget.visit(depth)?;
    match resolved_type {
        ResolvedType::Num => *mask |= capability(0),
        ResolvedType::Record(_) => *mask |= capability(2),
        ResolvedType::VocabularyRecord(_) => *mask |= capability(5),
        ResolvedType::List(inner) => {
            *mask |= capability(3);
            collect_type_capabilities(inner, depth + 1, budget, mask)?;
        }
        ResolvedType::Ref(inner) => {
            *mask |= capability(4);
            collect_type_capabilities(inner, depth + 1, budget, mask)?;
        }
        ResolvedType::Nullable(inner) => {
            *mask |= capability(1);
            collect_type_capabilities(inner, depth + 1, budget, mask)?;
        }
        ResolvedType::String | ResolvedType::Bool => {}
    }
    Ok(())
}

/// Collects capability bits from one logical value.
fn collect_value_capabilities(
    value: &LogicalValue,
    depth: usize,
    budget: &mut TraversalBudget,
    mask: &mut u64,
) -> Result<(), EncodingError> {
    budget.visit(depth)?;
    match value {
        LogicalValue::Number(_) => *mask |= capability(0),
        LogicalValue::Null => *mask |= capability(1),
        LogicalValue::Record(record) => {
            *mask |= capability(2);
            for field in record.fields() {
                collect_value_capabilities(field.value(), depth + 1, budget, mask)?;
            }
        }
        LogicalValue::VocabularyRecord(record) => {
            *mask |= capability(5);
            for field in record.fields() {
                collect_value_capabilities(field.value(), depth + 1, budget, mask)?;
            }
        }
        LogicalValue::List(items) => {
            *mask |= capability(3);
            for item in items {
                collect_value_capabilities(item, depth + 1, budget, mask)?;
            }
        }
        LogicalValue::Reference(reference) => {
            *mask |= capability(4);
            collect_type_capabilities(reference.target_type(), depth + 1, budget, mask)?;
        }
        LogicalValue::String(_) | LogicalValue::Boolean(_) => {}
    }
    Ok(())
}
