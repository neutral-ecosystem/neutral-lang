// SPDX-License-Identifier: Apache-2.0

//! Frozen vocabulary schema spellings and reusable validation predicates.

/// Exact captured-bundle format discriminator.
pub(crate) const BUNDLE_FORMAT: &str = "neutral-vocabulary-bundle";
/// Exact v0 JSON bundle encoding version.
pub(crate) const ENCODING_VERSION: &str = "0.1";
/// Exact v0 logical vocabulary schema version.
pub(crate) const SCHEMA_VERSION: &str = "0.1";

/// Core names unavailable to vocabulary-owned types and fields.
const PROTECTED_CORE_NAMES: [&str; 13] = [
    "num", "string", "bool", "List", "Ref", "neu", "module", "use", "record", "true", "false",
    "null", "ref",
];

/// Member spellings that attempt to introduce executable behavior.
const EXECUTABLE_MEMBERS: [&str; 10] = [
    "script",
    "callback",
    "bytecode",
    "native",
    "module",
    "wasm",
    "validator",
    "execute",
    "entrypoint",
    "command",
];

/// Returns whether a name belongs to the frozen protected core set.
pub(crate) fn is_protected_name(value: &str) -> bool {
    PROTECTED_CORE_NAMES.contains(&value)
}

/// Returns whether an unknown member explicitly requests executable behavior.
pub(crate) fn is_executable_member(value: &str) -> bool {
    EXECUTABLE_MEMBERS.contains(&value)
}

/// Returns whether a vocabulary or nominal type name is uppercase-leading ASCII.
pub(crate) fn is_upper_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    matches!(bytes.next(), Some(b'A'..=b'Z')) && bytes.all(|byte| byte.is_ascii_alphanumeric())
}

/// Returns whether a field name is canonical lower snake case ASCII.
pub(crate) fn is_snake_name(value: &str) -> bool {
    let bytes = value.as_bytes();
    !bytes.is_empty()
        && bytes[0].is_ascii_lowercase()
        && bytes[bytes.len() - 1].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
        && !bytes.windows(2).any(|pair| pair == b"__")
}

/// Returns whether text is one exact canonical three-part release version.
pub(crate) fn is_exact_release_version(value: &str) -> bool {
    let mut parts = value.split('.');
    let valid = parts.by_ref().take(3).all(is_canonical_unsigned_component);
    valid && parts.next().is_none() && value.matches('.').count() == 2
}

/// Returns whether one release-version component is canonical ASCII decimal.
fn is_canonical_unsigned_component(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && (value == "0" || !value.starts_with('0'))
}

/// Returns whether a structural feature ID is bounded to inert qualified ASCII.
pub(crate) fn is_feature_id(value: &str) -> bool {
    !value.is_empty()
        && value.starts_with(|character: char| character.is_ascii_lowercase())
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'/' | b':' | b'@')
        })
}
