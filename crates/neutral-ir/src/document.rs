// SPDX-License-Identifier: Apache-2.0

//! Document ownership and logical comparison independent of local labels.
//!
//! `LogicalDocument` groups module identity, record definitions, declarations
//! and the captured vocabulary contract. Equivalence matches symbols and builds
//! a bijection between graph-local element IDs before comparing reference edges.
//! Source spans, provenance and derivation policy belong to separate companions.

use crate::{
    Declaration, ElementId, LogicalModuleIdentity, LogicalValue, ModuleSymbolIdentity,
    RecordTypeDefinition, VocabularyContract,
};
use std::collections::BTreeMap;

/// Immutable validated logical Neutral document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalDocument {
    /// Logical module identity.
    module: LogicalModuleIdentity,
    /// Exported nominal record declarations in canonical name order.
    record_types: Vec<RecordTypeDefinition>,
    /// Optional exact captured vocabulary contract.
    vocabulary: Option<VocabularyContract>,
    /// Exported declarations in deterministic source order.
    pub(super) declarations: Vec<Declaration>,
}

impl LogicalDocument {
    /// Creates a validated logical document.
    #[must_use]
    pub fn new(module: LogicalModuleIdentity, declarations: Vec<Declaration>) -> Self {
        Self {
            module,
            record_types: Vec::new(),
            vocabulary: None,
            declarations,
        }
    }

    /// Creates a validated logical document containing nominal records and bindings.
    #[must_use]
    pub fn with_record_types(
        module: LogicalModuleIdentity,
        record_types: Vec<RecordTypeDefinition>,
        declarations: Vec<Declaration>,
    ) -> Self {
        Self {
            module,
            record_types,
            vocabulary: None,
            declarations,
        }
    }

    /// Attaches the exact validated captured vocabulary contract.
    #[must_use]
    pub fn with_vocabulary(mut self, vocabulary: VocabularyContract) -> Self {
        self.vocabulary = Some(vocabulary);
        self
    }

    /// Returns the exact captured vocabulary contract, when used.
    #[must_use]
    pub const fn vocabulary(&self) -> Option<&VocabularyContract> {
        self.vocabulary.as_ref()
    }

    /// Returns the logical module identity.
    #[must_use]
    pub const fn module(&self) -> &LogicalModuleIdentity {
        &self.module
    }

    /// Returns exported nominal records in canonical name order.
    #[must_use]
    pub fn record_types(&self) -> &[RecordTypeDefinition] {
        &self.record_types
    }

    /// Finds one exported nominal record by its validated name.
    #[must_use]
    pub fn record_type_by_name(&self, name: &str) -> Option<&RecordTypeDefinition> {
        self.record_types
            .iter()
            .find(|record| record.name() == name)
    }

    /// Returns exported declarations in deterministic order.
    #[must_use]
    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }

    /// Compares the complete logical payload under one bijective element-ID mapping.
    ///
    /// Companion source maps, provenance, derivation facts, and graph-local ID
    /// spellings are intentionally outside this comparison. Both documents must
    /// still contain unique node IDs and references to declared binding nodes.
    #[must_use]
    pub fn logically_equivalent(&self, other: &Self) -> bool {
        let Some(mapping) = ElementIdMapping::between(self, other) else {
            return false;
        };
        self.module == other.module
            && match (&self.vocabulary, &other.vocabulary) {
                (Some(left), Some(right)) => left.logically_equivalent(right),
                (None, None) => true,
                _ => false,
            }
            && logical_record_types_equal(self, other)
            && logical_declarations_equal(self, other, &mapping)
    }
}

/// One temporary bijection between graph-local IDs in two logical documents.
struct ElementIdMapping {
    /// Left-to-right graph-node mapping.
    forward: BTreeMap<ElementId, ElementId>,
    /// Right-to-left graph-node mapping proving injectivity.
    reverse: BTreeMap<ElementId, ElementId>,
}

impl ElementIdMapping {
    /// Builds one complete mapping by matching durable module-symbol identities.
    fn between(left: &LogicalDocument, right: &LogicalDocument) -> Option<Self> {
        if left.record_types.len() != right.record_types.len()
            || left.declarations.len() != right.declarations.len()
        {
            return None;
        }
        let mut mapping = Self {
            forward: BTreeMap::new(),
            reverse: BTreeMap::new(),
        };
        let right_records = unique_records_by_symbol(right)?;
        for record in &left.record_types {
            mapping.insert(
                record.element_id(),
                right_records.get(record.symbol_identity())?.element_id(),
            )?;
        }
        let right_declarations = unique_declarations_by_symbol(right)?;
        for declaration in &left.declarations {
            mapping.insert(
                declaration.element_id(),
                right_declarations
                    .get(declaration.symbol_identity())?
                    .element_id(),
            )?;
        }
        (mapping.forward.len() == left.record_types.len() + left.declarations.len())
            .then_some(mapping)
    }

    /// Adds one pair only when both graph labels remain one-to-one.
    fn insert(&mut self, left: ElementId, right: ElementId) -> Option<()> {
        if self.forward.insert(left, right).is_some() || self.reverse.insert(right, left).is_some()
        {
            return None;
        }
        Some(())
    }

    /// Maps one left document label into the right document.
    fn target(&self, source: ElementId) -> Option<ElementId> {
        self.forward.get(&source).copied()
    }
}

/// Indexes record declarations by durable identity while rejecting duplicates.
fn unique_records_by_symbol(
    document: &LogicalDocument,
) -> Option<BTreeMap<&ModuleSymbolIdentity, &RecordTypeDefinition>> {
    let mut records = BTreeMap::new();
    for record in &document.record_types {
        if records.insert(record.symbol_identity(), record).is_some() {
            return None;
        }
    }
    Some(records)
}

/// Indexes binding declarations by durable identity while rejecting duplicates.
fn unique_declarations_by_symbol(
    document: &LogicalDocument,
) -> Option<BTreeMap<&ModuleSymbolIdentity, &Declaration>> {
    let mut declarations = BTreeMap::new();
    for declaration in &document.declarations {
        if declarations
            .insert(declaration.symbol_identity(), declaration)
            .is_some()
        {
            return None;
        }
    }
    Some(declarations)
}

/// Compares nominal record payloads without graph-local IDs or vector order.
fn logical_record_types_equal(left: &LogicalDocument, right: &LogicalDocument) -> bool {
    let (Some(left_records), Some(right_records)) = (
        unique_records_by_symbol(left),
        unique_records_by_symbol(right),
    ) else {
        return false;
    };
    left_records.iter().all(|(identity, left_record)| {
        right_records.get(identity).is_some_and(|right_record| {
            left_record.fingerprint() == right_record.fingerprint()
                && left_record.nominal_identity() == right_record.nominal_identity()
                && left_record.fields() == right_record.fields()
        })
    })
}

/// Compares binding payloads and every nested identity edge under one mapping.
fn logical_declarations_equal(
    left: &LogicalDocument,
    right: &LogicalDocument,
    mapping: &ElementIdMapping,
) -> bool {
    let (Some(left_declarations), Some(right_declarations)) = (
        unique_declarations_by_symbol(left),
        unique_declarations_by_symbol(right),
    ) else {
        return false;
    };
    left_declarations
        .iter()
        .all(|(identity, left_declaration)| {
            right_declarations
                .get(identity)
                .is_some_and(|right_declaration| {
                    left_declaration.fingerprint() == right_declaration.fingerprint()
                        && left_declaration.name() == right_declaration.name()
                        && left_declaration.resolved_type() == right_declaration.resolved_type()
                        && logical_values_equal(
                            left_declaration.value(),
                            right_declaration.value(),
                            mapping,
                            &left_declarations,
                            &right_declarations,
                        )
                })
        })
}

/// Compares recursively nested values while translating identity-edge targets.
fn logical_values_equal(
    left: &LogicalValue,
    right: &LogicalValue,
    mapping: &ElementIdMapping,
    left_declarations: &BTreeMap<&ModuleSymbolIdentity, &Declaration>,
    right_declarations: &BTreeMap<&ModuleSymbolIdentity, &Declaration>,
) -> bool {
    match (left, right) {
        (LogicalValue::Number(left), LogicalValue::Number(right)) => left == right,
        (LogicalValue::String(left), LogicalValue::String(right)) => left == right,
        (LogicalValue::Boolean(left), LogicalValue::Boolean(right)) => left == right,
        (LogicalValue::Null, LogicalValue::Null) => true,
        (LogicalValue::Record(left), LogicalValue::Record(right)) => {
            left.nominal_type() == right.nominal_type()
                && left.fields().len() == right.fields().len()
                && left
                    .fields()
                    .iter()
                    .zip(right.fields())
                    .all(|(left_field, right_field)| {
                        left_field.name() == right_field.name()
                            && logical_values_equal(
                                left_field.value(),
                                right_field.value(),
                                mapping,
                                left_declarations,
                                right_declarations,
                            )
                    })
        }
        (LogicalValue::VocabularyRecord(left), LogicalValue::VocabularyRecord(right)) => {
            left.nominal_type() == right.nominal_type()
                && left.fields().len() == right.fields().len()
                && left
                    .fields()
                    .iter()
                    .zip(right.fields())
                    .all(|(left_field, right_field)| {
                        left_field.name() == right_field.name()
                            && logical_values_equal(
                                left_field.value(),
                                right_field.value(),
                                mapping,
                                left_declarations,
                                right_declarations,
                            )
                    })
        }
        (LogicalValue::List(left), LogicalValue::List(right)) => {
            left.len() == right.len()
                && left.iter().zip(right).all(|(left_item, right_item)| {
                    logical_values_equal(
                        left_item,
                        right_item,
                        mapping,
                        left_declarations,
                        right_declarations,
                    )
                })
        }
        (LogicalValue::Reference(left), LogicalValue::Reference(right)) => {
            left_declarations
                .get(left.target_symbol_identity())
                .is_some_and(|target| target.element_id() == left.target_element_id())
                && right_declarations
                    .get(right.target_symbol_identity())
                    .is_some_and(|target| target.element_id() == right.target_element_id())
                && mapping.target(left.target_element_id()) == Some(right.target_element_id())
                && left.target_symbol_identity() == right.target_symbol_identity()
                && left.target_type() == right.target_type()
        }
        _ => false,
    }
}
