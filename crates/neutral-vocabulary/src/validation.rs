// SPDX-License-Identifier: Apache-2.0

//! Reusable closed-schema validation primitives for decoded JSON values.

use crate::{JsonValue, VocabularyError, schema};

/// Requires a JSON object while retaining the decoder's duplicate-key check.
pub(crate) fn object(value: &JsonValue) -> Result<&[(String, JsonValue)], VocabularyError> {
    match value {
        JsonValue::Object(members) => Ok(members),
        _ => Err(VocabularyError::InvalidMemberType),
    }
}

/// Requires one JSON array.
pub(crate) fn array(value: &JsonValue) -> Result<&[JsonValue], VocabularyError> {
    match value {
        JsonValue::Array(items) => Ok(items),
        _ => Err(VocabularyError::InvalidMemberType),
    }
}

/// Requires one known member without silently accepting omissions.
pub(crate) fn member<'a>(
    object: &'a [(String, JsonValue)],
    name: &str,
) -> Result<&'a JsonValue, VocabularyError> {
    object
        .iter()
        .find(|(candidate, _)| candidate == name)
        .map(|(_, value)| value)
        .ok_or(VocabularyError::MissingMember)
}

/// Requires one string member.
pub(crate) fn string<'a>(
    object: &'a [(String, JsonValue)],
    name: &str,
) -> Result<&'a str, VocabularyError> {
    match member(object, name)? {
        JsonValue::String(value) => Ok(value),
        _ => Err(VocabularyError::InvalidMemberType),
    }
}

/// Requires one Boolean member.
pub(crate) fn boolean(object: &[(String, JsonValue)], name: &str) -> Result<bool, VocabularyError> {
    match member(object, name)? {
        JsonValue::Bool(value) => Ok(*value),
        _ => Err(VocabularyError::InvalidMemberType),
    }
}

/// Rejects unknown or executable members in one closed-schema object.
pub(crate) fn members(
    object: &[(String, JsonValue)],
    allowed: &[&str],
) -> Result<(), VocabularyError> {
    for (name, _) in object {
        if !allowed.contains(&name.as_str()) {
            return if schema::is_executable_member(name) {
                Err(VocabularyError::ExecutableShapeForbidden)
            } else {
                Err(VocabularyError::UnknownMember)
            };
        }
    }
    Ok(())
}

/// Requires an object to contain exactly the complete allowed member set.
pub(crate) fn exact_members(
    object: &[(String, JsonValue)],
    allowed: &[&str],
) -> Result<(), VocabularyError> {
    members(object, allowed)?;
    if object.len() == allowed.len() {
        Ok(())
    } else {
        Err(VocabularyError::MissingMember)
    }
}
