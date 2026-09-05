// SPDX-License-Identifier: Apache-2.0

//! Minimal deterministic writer for the restricted Neutral CBOR subset.

use crate::{EncodingError, constants};

/// One bounded restricted-CBOR section writer.
pub(crate) struct CborWriter {
    /// Accumulated section bytes.
    bytes: Vec<u8>,
}

impl CborWriter {
    /// Creates an empty section writer.
    pub(crate) const fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// Finishes the nonempty bounded section.
    pub(crate) fn finish(self) -> Result<Vec<u8>, EncodingError> {
        if self.bytes.is_empty() || self.bytes.len() > constants::MAXIMUM_SECTION_BYTES {
            return Err(EncodingError::EncodedSizeLimit);
        }
        Ok(self.bytes)
    }

    /// Writes a definite map header.
    pub(crate) fn map(&mut self, length: usize) -> Result<(), EncodingError> {
        self.container(5, length)
    }

    /// Writes a definite array header.
    pub(crate) fn array(&mut self, length: usize) -> Result<(), EncodingError> {
        self.container(4, length)
    }

    /// Writes one UTF-8 text string.
    pub(crate) fn text(&mut self, value: &str) -> Result<(), EncodingError> {
        if value.len() > constants::MAXIMUM_TEXT_BYTES {
            return Err(EncodingError::EncodedSizeLimit);
        }
        self.argument(3, value.len() as u64)?;
        self.extend(value.as_bytes())
    }

    /// Writes one byte string.
    pub(crate) fn bytes(&mut self, value: &[u8]) -> Result<(), EncodingError> {
        if value.len() > constants::MAXIMUM_BYTE_STRING_BYTES {
            return Err(EncodingError::EncodedSizeLimit);
        }
        self.argument(2, value.len() as u64)?;
        self.extend(value)
    }

    /// Writes one unsigned integer.
    pub(crate) fn unsigned(&mut self, value: u64) -> Result<(), EncodingError> {
        self.argument(0, value)
    }

    /// Writes one signed integer.
    pub(crate) fn signed(&mut self, value: i64) -> Result<(), EncodingError> {
        if value >= 0 {
            self.argument(
                0,
                u64::try_from(value).map_err(|_| EncodingError::InternalDefect)?,
            )
        } else {
            self.argument(1, value.unsigned_abs() - 1)
        }
    }

    /// Writes one Boolean.
    pub(crate) fn boolean(&mut self, value: bool) -> Result<(), EncodingError> {
        self.push(if value { 0xf5 } else { 0xf4 })
    }

    /// Writes the null simple value.
    pub(crate) fn null(&mut self) -> Result<(), EncodingError> {
        self.push(0xf6)
    }

    /// Writes a bounded definite container header.
    fn container(&mut self, major: u8, length: usize) -> Result<(), EncodingError> {
        if length > constants::MAXIMUM_CONTAINER_ITEMS {
            return Err(EncodingError::EncodedSizeLimit);
        }
        self.argument(major, length as u64)
    }

    /// Writes a shortest-width major-type argument.
    fn argument(&mut self, major: u8, value: u64) -> Result<(), EncodingError> {
        let prefix = major << 5;
        match value {
            0..=23 => {
                self.push(prefix | u8::try_from(value).map_err(|_| EncodingError::InternalDefect)?)
            }
            24..=0xff => self.extend(&[
                prefix | 0x18,
                u8::try_from(value).map_err(|_| EncodingError::InternalDefect)?,
            ]),
            0x100..=0xffff => {
                self.push(prefix | 0x19)?;
                self.extend(
                    &u16::try_from(value)
                        .map_err(|_| EncodingError::InternalDefect)?
                        .to_be_bytes(),
                )
            }
            0x1_0000..=0xffff_ffff => {
                self.push(prefix | 0x1a)?;
                self.extend(
                    &u32::try_from(value)
                        .map_err(|_| EncodingError::InternalDefect)?
                        .to_be_bytes(),
                )
            }
            _ => {
                self.push(prefix | 0x1b)?;
                self.extend(&value.to_be_bytes())
            }
        }
    }

    /// Appends one byte under the section ceiling.
    fn push(&mut self, value: u8) -> Result<(), EncodingError> {
        self.bytes.push(value);
        self.check_size()
    }

    /// Appends bytes under the section ceiling.
    fn extend(&mut self, value: &[u8]) -> Result<(), EncodingError> {
        let new_length = self
            .bytes
            .len()
            .checked_add(value.len())
            .ok_or(EncodingError::EncodedSizeLimit)?;
        if new_length > constants::MAXIMUM_SECTION_BYTES {
            return Err(EncodingError::EncodedSizeLimit);
        }
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    /// Checks the current section size.
    fn check_size(&self) -> Result<(), EncodingError> {
        if self.bytes.len() > constants::MAXIMUM_SECTION_BYTES {
            Err(EncodingError::EncodedSizeLimit)
        } else {
            Ok(())
        }
    }
}
