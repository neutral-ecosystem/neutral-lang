// SPDX-License-Identifier: Apache-2.0

//! Shared primitives for emitting the restricted CBOR section schemas.
//!
//! Section encoders reuse field-key, scalar and text-array writers rather than
//! duplicating wire primitives. Depth checks enforce the encoding contract's
//! fixed ceiling. Logical schema decisions stay with the section modules, while
//! byte-level CBOR representation stays with `CborWriter`.

use super::{CborWriter, EncodingError, constants};

/// Encodes a map containing only a `kind` discriminator.
pub(super) fn kind_only(writer: &mut CborWriter, kind: &str) -> Result<(), EncodingError> {
    writer.map(1)?;
    field_text(writer, constants::key::KIND, kind)
}

/// Encodes one map key.
pub(super) fn field_key(writer: &mut CborWriter, key: &str) -> Result<(), EncodingError> {
    writer.text(key)
}

/// Encodes one text-valued map field.
pub(super) fn field_text(
    writer: &mut CborWriter,
    key: &str,
    value: &str,
) -> Result<(), EncodingError> {
    field_key(writer, key)?;
    writer.text(value)
}

/// Encodes one unsigned-integer-valued map field.
pub(super) fn field_unsigned(
    writer: &mut CborWriter,
    key: &str,
    value: u64,
) -> Result<(), EncodingError> {
    field_key(writer, key)?;
    writer.unsigned(value)
}

/// Encodes one array of UTF-8 strings.
pub(super) fn encode_text_array<T: AsRef<str>>(
    writer: &mut CborWriter,
    values: &[T],
) -> Result<(), EncodingError> {
    writer.array(values.len())?;
    for value in values {
        writer.text(value.as_ref())?;
    }
    Ok(())
}

/// Enforces the fixed logical nesting ceiling.
pub(super) fn check_depth(depth: usize) -> Result<(), EncodingError> {
    if depth > constants::MAXIMUM_NESTING_DEPTH {
        Err(EncodingError::EncodedSizeLimit)
    } else {
        Ok(())
    }
}
