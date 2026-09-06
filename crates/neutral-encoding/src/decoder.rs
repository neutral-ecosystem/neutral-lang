// SPDX-License-Identifier: Apache-2.0

//! Hostile frame and restricted-CBOR decoding primitives.

use crate::{DecodeError, DecodeErrorClass, DecodeLimits};
use neutral_core::CancellationToken;

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
    let mut parser = Parser {
        bytes,
        position: 0,
        base_offset,
        limits,
        nodes: 0,
        cancellation,
    };
    let value = parser.value(1)?;
    if parser.position != bytes.len() {
        return Err(parser.error(DecodeErrorClass::MalformedCbor));
    }
    Ok(value)
}

/// Stateful parser that checks bounds before every allocation.
struct Parser<'a> {
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
}

impl Parser<'_> {
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
                let magnitude = i64::try_from(argument)
                    .map_err(|_| self.error(DecodeErrorClass::InvalidEncodedSchema))?;
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
            values.push(self.value(depth + 1)?);
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
            let key = self.value(depth + 1)?;
            let CborValue::Text(key) = key.value else {
                return Err(Self::error_at(DecodeErrorClass::MalformedCbor, key.offset));
            };
            let value = self.value(depth + 1)?;
            members.push((key, value));
        }
        Ok(members)
    }

    /// Reads and bounds one definite container length.
    fn container_length(&mut self, additional: u8) -> Result<usize, DecodeError> {
        let length = self.argument(additional)?;
        let length =
            usize::try_from(length).map_err(|_| self.error(DecodeErrorClass::EncodedSizeLimit))?;
        if length > self.limits.maximum_container_items() {
            return Err(self.error(DecodeErrorClass::EncodedSizeLimit));
        }
        if length > self.remaining() {
            return Err(self.error(DecodeErrorClass::MalformedCbor));
        }
        Ok(length)
    }

    /// Reads one bounded byte string after validating remaining input.
    fn byte_string(&mut self, additional: u8, maximum: usize) -> Result<Vec<u8>, DecodeError> {
        let length = self.argument(additional)?;
        let length =
            usize::try_from(length).map_err(|_| self.error(DecodeErrorClass::EncodedSizeLimit))?;
        if length > maximum {
            return Err(self.error(DecodeErrorClass::EncodedSizeLimit));
        }
        if length > self.remaining() {
            return Err(self.error(DecodeErrorClass::MalformedCbor));
        }
        let end = self
            .position
            .checked_add(length)
            .ok_or_else(|| self.error(DecodeErrorClass::EncodedSizeLimit))?;
        let value = self.bytes[self.position..end].to_vec();
        self.position = end;
        Ok(value)
    }

    /// Reads one bounded valid UTF-8 text string.
    fn text_string(&mut self, additional: u8) -> Result<String, DecodeError> {
        let bytes = self.byte_string(additional, self.limits.maximum_text_bytes())?;
        String::from_utf8(bytes).map_err(|_| self.error(DecodeErrorClass::MalformedCbor))
    }

    /// Reads a supported CBOR argument and rejects indefinite/reserved forms.
    fn argument(&mut self, additional: u8) -> Result<u64, DecodeError> {
        match additional {
            0..=23 => Ok(u64::from(additional)),
            24 => Ok(u64::from(self.read_byte()?)),
            25 => Ok(u64::from(u16::from_be_bytes(self.read_array()?))),
            26 => Ok(u64::from(u32::from_be_bytes(self.read_array()?))),
            27 => Ok(u64::from_be_bytes(self.read_array()?)),
            _ => Err(self.error(DecodeErrorClass::MalformedCbor)),
        }
    }

    /// Reads one fixed-size byte array without unchecked slicing.
    fn read_array<const N: usize>(&mut self) -> Result<[u8; N], DecodeError> {
        let end = self
            .position
            .checked_add(N)
            .ok_or_else(|| self.error(DecodeErrorClass::MalformedCbor))?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| self.error(DecodeErrorClass::MalformedCbor))?;
        self.position = end;
        bytes
            .try_into()
            .map_err(|_| self.error(DecodeErrorClass::InternalDefect))
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
        let absolute = self
            .base_offset
            .checked_add(self.position)
            .ok_or_else(|| self.error(DecodeErrorClass::EncodedSizeLimit))?;
        u64::try_from(absolute).map_err(|_| self.error(DecodeErrorClass::EncodedSizeLimit))
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
