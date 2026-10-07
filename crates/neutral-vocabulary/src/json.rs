// SPDX-License-Identifier: Apache-2.0

//! Minimal strict bounded JSON decoder for untrusted vocabulary bundles.

use crate::{VocabularyError, VocabularyLimits};
use neutral_core::CancellationToken;

#[cfg(test)]
#[path = "../tests/json/mod.rs"]
mod tests;

/// Untrusted JSON tree retained only until closed-schema validation succeeds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum JsonValue {
    /// JSON null.
    Null,
    /// JSON Boolean.
    Bool(bool),
    /// Decoded Unicode string.
    String(String),
    /// Ordered JSON array.
    Array(Vec<JsonValue>),
    /// Ordered object members retained without map collapse.
    Object(Vec<(String, JsonValue)>),
}

/// Parses one complete strict JSON value under explicit allocation limits.
pub(crate) fn parse(text: &str, limits: VocabularyLimits) -> Result<JsonValue, VocabularyError> {
    parse_with_cancellation(text, limits, None)
}

/// Parses a composition input with cancellation checkpoints inside strings and node traversal.
pub(crate) fn parse_cancellable(
    text: &str,
    limits: VocabularyLimits,
    cancellation: &CancellationToken,
) -> Result<JsonValue, VocabularyError> {
    parse_with_cancellation(text, limits, Some(cancellation))
}

/// Preserves legacy parsing behavior while sharing the explicitly cancellable decoder.
fn parse_with_cancellation(
    text: &str,
    limits: VocabularyLimits,
    cancellation: Option<&CancellationToken>,
) -> Result<JsonValue, VocabularyError> {
    parse_observed(text, limits, cancellation, &mut |_| Ok(()))
}

/// Drives the real decoder with private reservation checkpoints for deterministic fault tests.
fn parse_observed<'a>(
    text: &'a str,
    limits: VocabularyLimits,
    cancellation: Option<&'a CancellationToken>,
    reservation: &'a mut dyn FnMut(usize) -> Result<(), VocabularyError>,
) -> Result<JsonValue, VocabularyError> {
    let mut parser = Parser {
        text,
        index: 0,
        limits,
        nodes: 0,
        cancellation,
        reservation,
    };
    parser.skip_whitespace();
    parser.check_cancelled()?;
    let value = parser.parse_value(1)?;
    parser.skip_whitespace();
    parser.check_cancelled()?;
    if parser.index == text.len() {
        Ok(value)
    } else {
        Err(VocabularyError::MalformedJson)
    }
}

/// Stateful byte-offset parser over one already validated UTF-8 string.
struct Parser<'a> {
    /// Complete untrusted JSON text.
    text: &'a str,
    /// Current UTF-8 byte offset.
    index: usize,
    /// Frozen resource limits.
    limits: VocabularyLimits,
    /// Total parsed JSON value nodes.
    nodes: u64,
    /// Optional composition-request cancellation; legacy callers supply none.
    cancellation: Option<&'a CancellationToken>,
    /// Request-local reservation observer, never global allocator state or public authority.
    reservation: &'a mut dyn FnMut(usize) -> Result<(), VocabularyError>,
}

impl Parser<'_> {
    /// Reserves collection growth fallibly after cancellation and before retaining another child.
    fn reserve<T>(&mut self, output: &mut Vec<T>) -> Result<(), VocabularyError> {
        self.check_cancelled()?;
        (self.reservation)(std::mem::size_of::<T>())?;
        self.check_cancelled()?;
        output
            .try_reserve(1)
            .map_err(|_| VocabularyError::JsonLimitExceeded)
    }

    /// Checks decoded byte limits before allocating or appending the next UTF-8 scalar.
    fn push_character(&mut self, output: &mut String, ch: char) -> Result<(), VocabularyError> {
        self.check_cancelled()?;
        let bytes = ch.len_utf8();
        let length = output
            .len()
            .checked_add(bytes)
            .ok_or(VocabularyError::JsonLimitExceeded)?;
        if length as u64 > self.limits.string_bytes() {
            return Err(VocabularyError::JsonLimitExceeded);
        }
        (self.reservation)(bytes)?;
        self.check_cancelled()?;
        output
            .try_reserve(bytes)
            .map_err(|_| VocabularyError::JsonLimitExceeded)?;
        output.push(ch);
        Ok(())
    }
    /// Checks a cancellation signal without changing any frozen non-cancellable caller.
    fn check_cancelled(&self) -> Result<(), VocabularyError> {
        if self
            .cancellation
            .is_some_and(CancellationToken::is_cancelled)
        {
            Err(VocabularyError::Cancelled)
        } else {
            Ok(())
        }
    }
    /// Parses one JSON value and rejects raw numeric tokens.
    fn parse_value(&mut self, depth: u64) -> Result<JsonValue, VocabularyError> {
        self.check_cancelled()?;
        if depth > self.limits.nesting_depth() {
            return Err(VocabularyError::JsonLimitExceeded);
        }
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or(VocabularyError::JsonLimitExceeded)?;
        if self.nodes > self.limits.total_nodes() {
            return Err(VocabularyError::JsonLimitExceeded);
        }
        match self.peek_byte() {
            Some(b'n') => self.parse_literal(b"null", JsonValue::Null),
            Some(b't') => self.parse_literal(b"true", JsonValue::Bool(true)),
            Some(b'f') => self.parse_literal(b"false", JsonValue::Bool(false)),
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b'[') => self.parse_array(depth),
            Some(b'{') => self.parse_object(depth),
            Some(b'-' | b'0'..=b'9') => Err(VocabularyError::RawJsonNumberForbidden),
            _ => Err(VocabularyError::MalformedJson),
        }
    }

    /// Parses one fixed ASCII literal.
    fn parse_literal(
        &mut self,
        expected: &[u8],
        value: JsonValue,
    ) -> Result<JsonValue, VocabularyError> {
        let remaining = &self.text.as_bytes()[self.index..];
        if !remaining.starts_with(expected) {
            return Err(VocabularyError::MalformedJson);
        }
        self.index += expected.len();
        Ok(value)
    }

    /// Parses one bounded array while preserving logical order.
    fn parse_array(&mut self, depth: u64) -> Result<JsonValue, VocabularyError> {
        self.index += 1;
        self.skip_whitespace();
        let mut items = Vec::new();
        if self.consume_byte(b']') {
            return Ok(JsonValue::Array(items));
        }
        loop {
            if u64::try_from(items.len()).unwrap_or(u64::MAX) >= self.limits.array_items() {
                return Err(VocabularyError::JsonLimitExceeded);
            }
            self.reserve(&mut items)?;
            items.push(self.parse_value(depth.saturating_add(1))?);
            self.skip_whitespace();
            if self.consume_byte(b']') {
                return Ok(JsonValue::Array(items));
            }
            if !self.consume_byte(b',') {
                return Err(VocabularyError::MalformedJson);
            }
            self.skip_whitespace();
        }
    }

    /// Parses one bounded object and rejects duplicate keys before map collapse.
    fn parse_object(&mut self, depth: u64) -> Result<JsonValue, VocabularyError> {
        self.index += 1;
        self.skip_whitespace();
        let mut members: Vec<(String, JsonValue)> = Vec::new();
        if self.consume_byte(b'}') {
            return Ok(JsonValue::Object(members));
        }
        loop {
            if u64::try_from(members.len()).unwrap_or(u64::MAX) >= self.limits.object_members() {
                return Err(VocabularyError::JsonLimitExceeded);
            }
            if self.peek_byte() != Some(b'"') {
                return Err(VocabularyError::MalformedJson);
            }
            let name = self.parse_string()?;
            if members.iter().any(|(existing, _)| existing == &name) {
                return Err(VocabularyError::DuplicateJsonMember);
            }
            self.skip_whitespace();
            if !self.consume_byte(b':') {
                return Err(VocabularyError::MalformedJson);
            }
            self.skip_whitespace();
            self.reserve(&mut members)?;
            let value = self.parse_value(depth.saturating_add(1))?;
            members.push((name, value));
            self.skip_whitespace();
            if self.consume_byte(b'}') {
                return Ok(JsonValue::Object(members));
            }
            if !self.consume_byte(b',') {
                return Err(VocabularyError::MalformedJson);
            }
            self.skip_whitespace();
        }
    }

    /// Parses and decodes one bounded JSON string including surrogate pairs.
    fn parse_string(&mut self) -> Result<String, VocabularyError> {
        if !self.consume_byte(b'"') {
            return Err(VocabularyError::MalformedJson);
        }
        let mut output = String::new();
        loop {
            self.check_cancelled()?;
            let byte = self.peek_byte().ok_or(VocabularyError::MalformedJson)?;
            match byte {
                b'"' => {
                    self.index += 1;
                    return Ok(output);
                }
                b'\\' => {
                    self.index += 1;
                    self.parse_escape(&mut output)?;
                }
                0x00..=0x1f => return Err(VocabularyError::MalformedJson),
                0x20..=0x7f => {
                    self.index += 1;
                    self.push_character(&mut output, char::from(byte))?;
                }
                _ => {
                    let character = self.text[self.index..]
                        .chars()
                        .next()
                        .ok_or(VocabularyError::MalformedJson)?;
                    self.index += character.len_utf8();
                    self.push_character(&mut output, character)?;
                }
            }
            if u64::try_from(output.len()).unwrap_or(u64::MAX) > self.limits.string_bytes() {
                return Err(VocabularyError::JsonLimitExceeded);
            }
        }
    }

    /// Decodes one JSON escape into a bounded output string.
    fn parse_escape(&mut self, output: &mut String) -> Result<(), VocabularyError> {
        let escaped = self.take_byte().ok_or(VocabularyError::MalformedJson)?;
        let character = match escaped {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{0008}',
            b'f' => '\u{000c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => {
                let first = self.parse_hex_quad()?;
                let scalar = if (0xd800..=0xdbff).contains(&first) {
                    if self.take_byte() != Some(b'\\') || self.take_byte() != Some(b'u') {
                        return Err(VocabularyError::MalformedJson);
                    }
                    let second = self.parse_hex_quad()?;
                    if !(0xdc00..=0xdfff).contains(&second) {
                        return Err(VocabularyError::MalformedJson);
                    }
                    0x1_0000 + ((u32::from(first) - 0xd800) << 10) + (u32::from(second) - 0xdc00)
                } else if (0xdc00..=0xdfff).contains(&first) {
                    return Err(VocabularyError::MalformedJson);
                } else {
                    u32::from(first)
                };
                char::from_u32(scalar).ok_or(VocabularyError::MalformedJson)?
            }
            _ => return Err(VocabularyError::MalformedJson),
        };
        self.push_character(output, character)
    }

    /// Parses exactly four lowercase or uppercase JSON hexadecimal digits.
    fn parse_hex_quad(&mut self) -> Result<u16, VocabularyError> {
        let mut value = 0_u16;
        for _ in 0..4 {
            let digit = self.take_byte().ok_or(VocabularyError::MalformedJson)?;
            let digit = match digit {
                b'0'..=b'9' => u16::from(digit - b'0'),
                b'a'..=b'f' => u16::from(digit - b'a' + 10),
                b'A'..=b'F' => u16::from(digit - b'A' + 10),
                _ => return Err(VocabularyError::MalformedJson),
            };
            value = (value << 4) | digit;
        }
        Ok(value)
    }

    /// Skips only RFC JSON insignificant whitespace bytes.
    fn skip_whitespace(&mut self) {
        while matches!(self.peek_byte(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.index += 1;
        }
    }

    /// Consumes one exact byte when present.
    fn consume_byte(&mut self, expected: u8) -> bool {
        if self.peek_byte() == Some(expected) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    /// Returns and advances past one byte.
    fn take_byte(&mut self) -> Option<u8> {
        let byte = self.peek_byte()?;
        self.index += 1;
        Some(byte)
    }

    /// Returns the next byte without advancing.
    fn peek_byte(&self) -> Option<u8> {
        self.text.as_bytes().get(self.index).copied()
    }
}
