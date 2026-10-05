// SPDX-License-Identifier: Apache-2.0

//! Shared serialization of generated JSON documents; schemas stay with their owners.

use serde::Serialize;

/// Serializes one deterministic indented document with a terminating newline.
pub(crate) fn pretty(value: &impl Serialize) -> Result<Vec<u8>, String> {
    let mut bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("could not serialize generated JSON: {error}"))?;
    bytes.push(b'\n');
    Ok(bytes)
}
