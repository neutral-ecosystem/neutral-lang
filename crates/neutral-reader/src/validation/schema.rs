// SPDX-License-Identifier: Apache-2.0

//! Validate captured vocabulary schemas and their nominal record values.
//!
//! Contract checks enforce the supported field/default model. Value checks
//! resolve vocabulary-qualified types against the retained contract and verify
//! their fields recursively. Validation uses captured facts only: vocabulary
//! names do not authorize acquisition or execution.

use super::{
    BTreeMap, LogicalValue, ReaderError, ResolvedType, VocabularyContract, is_closed_default,
    validate_value,
};

/// Validates canonical shape and closed defaults in an embedded vocabulary contract.
pub(super) fn validate_vocabulary_contract(
    contract: Option<&VocabularyContract>,
) -> Result<(), ReaderError> {
    let Some(contract) = contract else {
        return Ok(());
    };
    let mut previous_type = None;
    for definition in contract.types() {
        if definition.identity().vocabulary() != contract.identity().identity()
            || previous_type.is_some_and(|name| name >= definition.identity().name())
        {
            return Err(ReaderError::InvalidRecordSchema);
        }
        previous_type = Some(definition.identity().name());
        let mut previous_field = None;
        for field in definition.fields() {
            if previous_field.is_some_and(|name| name >= field.name())
                || field.default_value().is_some_and(|value| {
                    !is_closed_default(value)
                        || !validate_vocabulary_value(field.resolved_type(), value, Some(contract))
                })
            {
                return Err(ReaderError::InvalidRecordSchema);
            }
            previous_field = Some(field.name());
        }
    }
    Ok(())
}

/// Validates qualified values solely against the embedded exact contract.
pub(super) fn validate_vocabulary_value(
    expected: &ResolvedType,
    value: &LogicalValue,
    contract: Option<&VocabularyContract>,
) -> bool {
    match (expected, value) {
        (ResolvedType::Nullable(_), LogicalValue::Null) => true,
        (ResolvedType::Nullable(inner), value) => validate_vocabulary_value(inner, value, contract),
        (ResolvedType::List(inner), LogicalValue::List(items)) => items
            .iter()
            .all(|item| validate_vocabulary_value(inner, item, contract)),
        (ResolvedType::VocabularyRecord(identity), LogicalValue::VocabularyRecord(value)) => {
            let Some(contract) = contract else {
                return false;
            };
            let Some(schema) = contract.type_by_name(identity.name()) else {
                return false;
            };
            identity == value.nominal_type()
                && identity.vocabulary() == contract.identity().identity()
                && schema.fields().len() == value.fields().len()
                && schema
                    .fields()
                    .iter()
                    .zip(value.fields())
                    .all(|(field, actual)| {
                        field.name() == actual.name()
                            && validate_value(
                                field.resolved_type(),
                                actual.value(),
                                &BTreeMap::new(),
                            )
                            && validate_vocabulary_value(
                                field.resolved_type(),
                                actual.value(),
                                Some(contract),
                            )
                    })
        }
        (ResolvedType::VocabularyRecord(_), _) => false,
        _ => true,
    }
}
