// SPDX-License-Identifier: Apache-2.0

//! Hostile frame and restricted-CBOR decoding primitives.

use crate::{DecodeError, DecodeErrorClass, DecodeLimits};
use neutral_core::CancellationToken;
use neutral_core::allocation::RetainCapacity;

#[cfg(test)]
#[path = "../tests/decoder/mod.rs"]
mod tests;

/// One duplicate-preserving untrusted CBOR value and its byte offset.
pub(crate) struct LocatedValue {
    /// Absolute encoded byte offset of the initial byte.
    pub(crate) offset: u64,
    /// Untrusted restricted-CBOR value.
    pub(crate) value: CborValue,
}

/// Untrusted value forms admitted by the restricted CBOR lexical profile.
pub(crate) enum CborValue {
    /// Unsigned integer.
    Unsigned(u64),
    /// Signed negative integer.
    Negative(i64),
    /// Definite byte string.
    Bytes(Vec<u8>),
    /// Definite valid UTF-8 text string.
    Text(String),
    /// Definite array.
    Array(Vec<LocatedValue>),
    /// Definite map retaining member order and duplicates.
    Map(Vec<(String, LocatedValue)>),
    /// Boolean simple value.
    Boolean(bool),
    /// Null simple value.
    Null,
}

/// Parses exactly one bounded restricted-CBOR section.
pub(crate) fn parse_section(
    bytes: &[u8],
    base_offset: usize,
    limits: DecodeLimits,
    cancellation: &CancellationToken,
) -> Result<LocatedValue, DecodeError> {
    parse_section_profile(bytes, base_offset, limits, cancellation, false, true)
        .map(|(value, _)| value)
}

/// Parses successor wire data with minimal-width integer and length arguments.
pub(crate) fn parse_canonical_section(
    bytes: &[u8],
    base_offset: usize,
    limits: DecodeLimits,
    cancellation: &CancellationToken,
) -> Result<LocatedValue, DecodeError> {
    parse_section_profile(bytes, base_offset, limits, cancellation, true, true)
        .map(|(value, _)| value)
}

/// Checks writer output with the same lexical budgets, without retaining a decode tree.
pub(crate) fn validate_section(
    bytes: &[u8],
    limits: DecodeLimits,
    cancellation: &CancellationToken,
) -> Result<(), DecodeError> {
    parse_section_profile(bytes, 0, limits, cancellation, true, false).map(|_| ())
}

/// Keeps legacy lexical acceptance separate from explicit canonical successor selection.
fn parse_section_profile(
    bytes: &[u8],
    base_offset: usize,
    limits: DecodeLimits,
    cancellation: &CancellationToken,
    canonical: bool,
    materialize: bool,
) -> Result<(LocatedValue, usize), DecodeError> {
    let mut parser = Parser {
        bytes,
        position: 0,
        base_offset,
        limits,
        nodes: 0,
        cancellation,
        canonical,
        materialize,
        allocations: 0,
        allocation_fault: None,
    };
    let value = parser.value(1)?;
    if parser.position != bytes.len() {
        return Err(parser.error(DecodeErrorClass::MalformedCbor));
    }
    Ok((value, parser.allocations))
}

/// Stateful parser that checks bounds before every allocation.
struct Parser<'a> {
    /// Validation-only walks share grammar and budgets but retain no untrusted tree.
    materialize: bool,
    /// Request-local reservation count for tests, never a global allocator override.
    allocations: usize,
    /// One private synthetic allocation failure or cancellation checkpoint.
    allocation_fault: Option<(usize, bool)>,
    /// Exact section bytes.
    bytes: &'a [u8],
    /// Next unread section-relative position.
    position: usize,
    /// Absolute offset of the section.
    base_offset: usize,
    /// Effective caller and hard limits.
    limits: DecodeLimits,
    /// Total decoded nodes.
    nodes: usize,
    /// Cooperative cancellation signal.
    cancellation: &'a CancellationToken,
    /// Explicit successor minimal-width enforcement; old selectors remain unchanged.
    canonical: bool,
}

impl<'a> Parser<'a> {
    /// Checks cancellation before fallible storage growth; tests interrupt the same boundary.
    fn reserve<T>(&mut self, values: &mut Vec<T>, count: usize) -> Result<(), DecodeError> {
        self.check_cancelled()?;
        if !self.materialize {
            return Ok(());
        }
        {
            let current = self.allocations;
            self.allocations += 1;
            if let Some((index, cancel)) = self.allocation_fault
                && index == current
            {
                if cancel {
                    self.cancellation.cancel();
                    return Err(self.error(DecodeErrorClass::Cancelled));
                }
                return Err(self.error(DecodeErrorClass::EncodedSizeLimit));
            }
        }
        values
            .try_retain(count)
            .map_err(|_| self.error(DecodeErrorClass::EncodedSizeLimit))?;
        self.check_cancelled()
    }
    /// Parses one value under depth, traversal, and cancellation limits.
    fn value(&mut self, depth: usize) -> Result<LocatedValue, DecodeError> {
        self.check_cancelled()?;
        if depth > self.limits.maximum_nesting_depth() {
            return Err(self.error(DecodeErrorClass::EncodedSizeLimit));
        }
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or_else(|| self.error(DecodeErrorClass::EncodedSizeLimit))?;
        if self.nodes > self.limits.maximum_traversal_nodes() {
            return Err(self.error(DecodeErrorClass::EncodedSizeLimit));
        }
        let offset = self.absolute_offset()?;
        let initial = self.read_byte()?;
        let major = initial >> 5;
        let additional = initial & 0x1f;
        let value = match major {
            0 => CborValue::Unsigned(self.argument(additional)?),
            1 => {
                let argument = self.argument(additional)?;
                if argument > i64::MAX.cast_unsigned() {
                    return Err(Self::error_at(
                        DecodeErrorClass::InvalidEncodedSchema,
                        offset,
                    ));
                }
                let Ok(magnitude) = i64::try_from(argument) else {
                    return Err(self.error(DecodeErrorClass::InvalidEncodedSchema));
                };
                CborValue::Negative(-1 - magnitude)
            }
            2 => CborValue::Bytes(
                self.byte_string(additional, self.limits.maximum_byte_string_bytes())?,
            ),
            3 => CborValue::Text(self.text_string(additional)?),
            4 => CborValue::Array(self.array(additional, depth)?),
            5 => CborValue::Map(self.map(additional, depth)?),
            7 if additional == 20 => CborValue::Boolean(false),
            7 if additional == 21 => CborValue::Boolean(true),
            7 if additional == 22 => CborValue::Null,
            _ => return Err(Self::error_at(DecodeErrorClass::MalformedCbor, offset)),
        };
        Ok(LocatedValue { offset, value })
    }

    /// Parses one bounded definite array without trusting its length for reserve.
    fn array(&mut self, additional: u8, depth: usize) -> Result<Vec<LocatedValue>, DecodeError> {
        let length = self.container_length(additional)?;
        let mut values = Vec::new();
        for _ in 0..length {
            if self.nodes >= self.limits.maximum_traversal_nodes() {
                return Err(self.error(DecodeErrorClass::EncodedSizeLimit));
            }
            self.reserve(&mut values, 1)?;
            let value = self.value(depth + 1)?;
            if self.materialize {
                values.push(value);
            }
        }
        Ok(values)
    }

    /// Parses one text-keyed map while retaining duplicate members.
    fn map(
        &mut self,
        additional: u8,
        depth: usize,
    ) -> Result<Vec<(String, LocatedValue)>, DecodeError> {
        let length = self.container_length(additional)?;
        let mut members = Vec::new();
        for _ in 0..length {
            if self
                .limits
                .maximum_traversal_nodes()
                .saturating_sub(self.nodes)
                < 2
            {
                return Err(self.error(DecodeErrorClass::EncodedSizeLimit));
            }
            self.reserve(&mut members, 1)?;
            let key = self.value(depth + 1)?;
            let CborValue::Text(key) = key.value else {
                return Err(Self::error_at(DecodeErrorClass::MalformedCbor, key.offset));
            };
            let value = self.value(depth + 1)?;
            if self.materialize {
                members.push((key, value));
            }
        }
        Ok(members)
    }

    /// Reads and bounds one definite container length.
    fn container_length(&mut self, additional: u8) -> Result<usize, DecodeError> {
        let length = self.argument(additional)?;
        let Ok(length) = usize::try_from(length) else {
            return Err(self.error(DecodeErrorClass::EncodedSizeLimit));
        };
        if length > self.limits.maximum_container_items() {
            return Err(self.error(DecodeErrorClass::EncodedSizeLimit));
        }
        if length > self.remaining() {
            return Err(self.error(DecodeErrorClass::MalformedCbor));
        }
        Ok(length)
    }

    /// Borrows bounded string bytes after checking length, before any retention.
    fn string_slice(&mut self, additional: u8, maximum: usize) -> Result<&'a [u8], DecodeError> {
        let length = self.argument(additional)?;
        let Ok(length) = usize::try_from(length) else {
            return Err(self.error(DecodeErrorClass::EncodedSizeLimit));
        };
        if length > maximum {
            return Err(self.error(DecodeErrorClass::EncodedSizeLimit));
        }
        if length > self.remaining() {
            return Err(self.error(DecodeErrorClass::MalformedCbor));
        }
        let Some(end) = self.position.checked_add(length) else {
            return Err(self.error(DecodeErrorClass::EncodedSizeLimit));
        };
        let start = self.position;
        self.position = end;
        Ok(&self.bytes[start..end])
    }

    /// Retains byte strings only when constructing an independent decode tree.
    fn byte_string(&mut self, additional: u8, maximum: usize) -> Result<Vec<u8>, DecodeError> {
        let bytes = self.string_slice(additional, maximum)?;
        let length = bytes.len();
        let mut value = Vec::new();
        self.reserve(&mut value, length)?;
        if self.materialize {
            value.extend_from_slice(bytes);
        }
        Ok(value)
    }

    /// Reads one bounded valid UTF-8 text string.
    fn text_string(&mut self, additional: u8) -> Result<String, DecodeError> {
        let bytes = self.string_slice(additional, self.limits.maximum_text_bytes())?;
        let length = bytes.len();
        if std::str::from_utf8(bytes).is_err() {
            return Err(self.error(DecodeErrorClass::MalformedCbor));
        }
        let mut retained = Vec::new();
        self.reserve(&mut retained, length)?;
        if self.materialize {
            retained.extend_from_slice(bytes);
        }
        match String::from_utf8(retained) {
            Ok(value) => Ok(value),
            Err(_) => Err(self.error(DecodeErrorClass::MalformedCbor)),
        }
    }

    /// Reads a supported CBOR argument and rejects indefinite/reserved forms.
    fn argument(&mut self, additional: u8) -> Result<u64, DecodeError> {
        let value = match additional {
            0..=23 => u64::from(additional),
            24 => u64::from(self.read_byte()?),
            25 => u64::from(u16::from_be_bytes(self.read_array()?)),
            26 => u64::from(u32::from_be_bytes(self.read_array()?)),
            27 => u64::from_be_bytes(self.read_array()?),
            _ => return Err(self.error(DecodeErrorClass::MalformedCbor)),
        };
        let minimum = match additional {
            24 => 24,
            25 => 256,
            26 => 65_536,
            27 => 4_294_967_296,
            _ => 0,
        };
        if self.canonical && value < minimum {
            return Err(self.error(DecodeErrorClass::MalformedCbor));
        }
        Ok(value)
    }

    /// Reads one fixed-size byte array without unchecked slicing.
    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let Some(end) = self.position.checked_add(N) else {
            return Err(self.error(DecodeErrorClass::MalformedCbor));
        };
        let Some(bytes) = self.bytes.get(self.position..end) else {
            return Err(self.error(DecodeErrorClass::MalformedCbor));
        };
        self.position = end;
        match bytes.try_into() {
            Ok(array) => Ok(array),
            Err(_) => Err(self.error(DecodeErrorClass::InternalDefect)),
        }
    }

    /// Reads one byte or returns a bounded truncation error.
    fn read_byte(&mut self) -> Result<u8, DecodeError> {
        let value = self
            .bytes
            .get(self.position)
            .copied()
            .ok_or_else(|| self.error(DecodeErrorClass::MalformedCbor))?;
        self.position += 1;
        Ok(value)
    }

    /// Returns the unread section byte count.
    fn remaining(&self) -> usize {
        self.bytes.len().saturating_sub(self.position)
    }

    /// Returns the current absolute offset with checked conversion.
    fn absolute_offset(&self) -> Result<u64, DecodeError> {
        let Some(absolute) = self.base_offset.checked_add(self.position) else {
            return Err(self.error(DecodeErrorClass::EncodedSizeLimit));
        };
        match u64::try_from(absolute) {
            Ok(offset) => Ok(offset),
            Err(_) => Err(self.error(DecodeErrorClass::EncodedSizeLimit)),
        }
    }

    /// Returns an error at the current position.
    fn error(&self, class: DecodeErrorClass) -> DecodeError {
        let offset = self
            .base_offset
            .checked_add(self.position)
            .and_then(|value| u64::try_from(value).ok());
        DecodeError::new(class, offset)
    }

    /// Returns an error at a previously captured absolute position.
    const fn error_at(class: DecodeErrorClass, offset: u64) -> DecodeError {
        DecodeError::new(class, Some(offset))
    }

    /// Stops promptly when caller cancellation is observed.
    fn check_cancelled(&self) -> Result<(), DecodeError> {
        if self.cancellation.is_cancelled() {
            Err(self.error(DecodeErrorClass::Cancelled))
        } else {
            Ok(())
        }
    }
}
