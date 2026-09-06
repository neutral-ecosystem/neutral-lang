// SPDX-License-Identifier: Apache-2.0

//! Frozen vocabulary schema spellings and reusable validation predicates.

/// Exact captured-bundle format discriminator.
pub(crate) const BUNDLE_FORMAT: &str = "neutral-vocabulary-bundle";
/// Exact v0 JSON bundle encoding version.
pub(crate) const ENCODING_VERSION: &str = "0.1";
/// Exact v0 logical vocabulary schema version.
pub(crate) const SCHEMA_VERSION: &str = "0.1";

pub(crate) use neutral_ir::language::{
    is_exact_release_version, is_feature_id, is_protected_name, is_snake_name, is_upper_name,
};

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

/// Returns whether an unknown member explicitly requests executable behavior.
pub(crate) fn is_executable_member(value: &str) -> bool {
    EXECUTABLE_MEMBERS.contains(&value)
}
