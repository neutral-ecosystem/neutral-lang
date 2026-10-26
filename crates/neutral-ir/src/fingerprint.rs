// SPDX-License-Identifier: Apache-2.0

//! Domain-separated fingerprints for binding definitions and record schemas.
//!
//! Bindings hash their resolved type and logical value; records hash their field
//! contracts and defaults. These fingerprints describe definitions, not symbol
//! continuity or complete project identity. Transcript tags and framing are
//! frozen compatibility contracts, not implementation details to rename.
//!
//! The constructors consume already-resolved, canonically ordered materialized
//! data. They do not sort fields, resolve references or validate schema/value
//! agreement. Reference payloads use durable module-symbol identities rather than
//! graph-local element numbers, so relabelling local IDs does not rename a target.
//!
//! These shared definition helpers retain their frozen grammar; current complete
//! project identities use the separate bounded transcript writers. Temporary
//! vectors and type formatting here use ordinary allocations, and recursive value
//! traversal has no request budget: callers must establish safe input bounds first.

use crate::{LogicalValue, RecordFieldSchema, ResolvedType};
use neutral_core::{CoreError, SemanticDigest, nht_frame};

/// Frozen declaration fingerprint separated from module-symbol continuity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DeclarationFingerprint(SemanticDigest);

impl DeclarationFingerprint {
    /// Reconstructs a declared fingerprint from validated external digest bytes.
    /// Wrapping the supplied digest does not recompute or verify the definition.
    #[must_use]
    pub const fn from_digest(digest: SemanticDigest) -> Self {
        Self(digest)
    }

    /// Computes the v1 binding fingerprint over resolved type and logical value.
    ///
    /// # Errors
    ///
    /// Returns an error only when NHT framing exceeds its fixed widths.
    pub fn for_binding(
        resolved_type: &ResolvedType,
        value: &LogicalValue,
    ) -> Result<Self, CoreError> {
        let mut payload = Vec::new();
        payload.extend(nht_frame("declaration-kind", b"binding")?);
        payload.extend(nht_frame(
            "resolved-type",
            resolved_type.to_string().as_bytes(),
        )?);
        let definition = logical_value_payload(value)?;
        payload.extend(nht_frame("logical-definition", &definition)?);
        SemanticDigest::from_nht("neutral/declaration-fingerprint/v1", &payload).map(Self)
    }

    /// Computes the v1 record-schema fingerprint over canonical field contracts.
    ///
    /// # Errors
    ///
    /// Returns an error only when NHT framing exceeds its fixed widths.
    pub fn for_record(fields: &[RecordFieldSchema]) -> Result<Self, CoreError> {
        // Field order is already canonical input; repairing it here would hide malformed data.
        let mut payload = Vec::new();
        payload.extend(nht_frame("declaration-kind", b"record")?);
        for field in fields {
            let mut definition = Vec::new();
            definition.extend(nht_frame("field-name", field.name().as_bytes())?);
            definition.extend(nht_frame(
                "field-type",
                field.resolved_type().to_string().as_bytes(),
            )?);
            let default = match field.default_value() {
                Some(value) => logical_value_payload(value)?,
                None => nht_frame("required", &[])?,
            };
            definition.extend(nht_frame("field-default", &default)?);
            payload.extend(nht_frame("record-field", &definition)?);
        }
        SemanticDigest::from_nht("neutral/declaration-fingerprint/v1", &payload).map(Self)
    }

    /// Returns the underlying domain-separated semantic digest.
    #[must_use]
    pub const fn digest(self) -> SemanticDigest {
        self.0
    }
}

/// Builds the recursive NHT payload for one final logical value.
fn logical_value_payload(value: &LogicalValue) -> Result<Vec<u8>, CoreError> {
    match value {
        LogicalValue::Number(number) => number.nht_payload(),
        LogicalValue::String(value) => nht_frame("string", value.as_bytes()),
        LogicalValue::Boolean(value) => nht_frame("boolean", &[u8::from(*value)]),
        LogicalValue::Null => nht_frame("null", &[]),
        LogicalValue::Record(value) => {
            let mut payload = Vec::new();
            payload.extend(nht_frame(
                "record-type",
                value.nominal_type().name().as_bytes(),
            )?);
            for field in value.fields() {
                let mut definition = Vec::new();
                definition.extend(nht_frame("field-name", field.name().as_bytes())?);
                definition.extend(nht_frame(
                    "field-value",
                    &logical_value_payload(field.value())?,
                )?);
                payload.extend(nht_frame("record-field", &definition)?);
            }
            nht_frame("record", &payload)
        }
        LogicalValue::VocabularyRecord(value) => {
            let mut payload = nht_frame(
                "qualified-type",
                value.nominal_type().to_string().as_bytes(),
            )?;
            for field in value.fields() {
                let mut framed = nht_frame("field-name", field.name().as_bytes())?;
                framed.extend(nht_frame(
                    "field-value",
                    &logical_value_payload(field.value())?,
                )?);
                payload.extend(nht_frame("field", &framed)?);
            }
            nht_frame("vocabulary-record", &payload)
        }
        LogicalValue::List(items) => {
            let mut payload = Vec::new();
            for item in items {
                payload.extend(nht_frame("list-item", &logical_value_payload(item)?)?);
            }
            nht_frame("list", &payload)
        }
        LogicalValue::Reference(reference) => {
            // Local element labels are indexing details; continuity comes from the stable owner tuple.
            let mut payload = Vec::new();
            payload.extend(nht_frame(
                "target-language-version",
                reference
                    .target_symbol_identity()
                    .module()
                    .language_behavior_version()
                    .as_bytes(),
            )?);
            payload.extend(nht_frame(
                "target-module",
                reference
                    .target_symbol_identity()
                    .module()
                    .module_name()
                    .as_bytes(),
            )?);
            payload.extend(nht_frame(
                "target-declaration",
                reference
                    .target_symbol_identity()
                    .declaration_name()
                    .as_bytes(),
            )?);
            payload.extend(nht_frame(
                "target-type",
                reference.target_type().to_string().as_bytes(),
            )?);
            nht_frame("identity-reference", &payload)
        }
    }
}
