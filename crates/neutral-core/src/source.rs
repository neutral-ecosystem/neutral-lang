// SPDX-License-Identifier: Apache-2.0

//! Checked original-byte spans and source coordinates shared across the pipeline.

use crate::{CoreError, SourceContentDigest};

/// A checked half-open byte range within one original source byte sequence.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ByteSpan {
    /// The first included original-byte offset.
    start: u64,
    /// The first excluded original-byte offset.
    end: u64,
}

impl ByteSpan {
    /// Creates a half-open span when `start` does not follow `end`.
    ///
    /// # Errors
    ///
    /// Returns [`CoreError::InvertedSpan`] when `start` follows `end`.
    pub const fn new(start: u64, end: u64) -> Result<Self, CoreError> {
        if start > end {
            return Err(CoreError::InvertedSpan { start, end });
        }
        Ok(Self { start, end })
    }

    /// Returns the first included original-byte offset.
    #[must_use]
    pub const fn start(self) -> u64 {
        self.start
    }

    /// Returns the first excluded original-byte offset.
    #[must_use]
    pub const fn end(self) -> u64 {
        self.end
    }

    /// Returns the number of bytes in this span.
    #[must_use]
    pub const fn len(self) -> u64 {
        self.end - self.start
    }

    /// Returns whether this span contains no bytes.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

/// A source location tied to one exact captured source identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SourceLocation {
    /// The exact captured source identity.
    source: SourceContentDigest,
    /// The half-open original-byte range.
    span: ByteSpan,
}

impl SourceLocation {
    /// Creates a location for `span` in the source identified by `source`.
    ///
    /// The digest binds the range to exact source bytes, but this constructor
    /// cannot check whether the range fits those bytes. The owning source-map
    /// validator must check that before a consumer indexes into captured input.
    #[must_use]
    pub const fn new(source: SourceContentDigest, span: ByteSpan) -> Self {
        Self { source, span }
    }

    /// Returns the exact source content identity.
    #[must_use]
    pub const fn source(self) -> SourceContentDigest {
        self.source
    }

    /// Returns the original-byte span.
    #[must_use]
    pub const fn span(self) -> ByteSpan {
        self.span
    }
}

/// A one-based line and original-byte column within captured source bytes.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LineColumn {
    /// The one-based logical line number.
    line: u64,
    /// The one-based original-byte column number.
    column: u64,
}

impl LineColumn {
    /// Returns the one-based logical line number.
    #[must_use]
    pub const fn line(self) -> u64 {
        self.line
    }

    /// Returns the one-based original-byte column number.
    #[must_use]
    pub const fn column(self) -> u64 {
        self.column
    }
}

/// Derives a one-based line and original-byte column at `offset`.
///
/// CRLF is one line ending, while a lone CR and a lone LF are each one line
/// ending. The LF byte in a CRLF pair maps to the following line's first column.
/// Columns count original bytes, so a multibyte UTF-8 character occupies multiple
/// columns. The end-of-input offset is valid; no decoding or tab expansion occurs.
/// The scan is linear in `offset`, so repeated bulk lookups belong in an indexed
/// source-map layer rather than repeated calls to this primitive.
///
/// # Errors
///
/// Returns [`CoreError::OffsetOutOfBounds`] when `offset` exceeds `bytes`.
pub fn line_column_at(bytes: &[u8], offset: u64) -> Result<LineColumn, CoreError> {
    let offset = usize::try_from(offset).unwrap_or(usize::MAX);
    if offset > bytes.len() {
        return Err(CoreError::OffsetOutOfBounds {
            offset: u64::try_from(offset).unwrap_or(u64::MAX),
            source_bytes: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        });
    }

    let mut line = 1_u64;
    let mut column = 1_u64;
    let mut index = 0_usize;
    while index < offset {
        match bytes[index] {
            b'\r' => {
                line = line.saturating_add(1);
                column = 1;
                index += 1;
                if index < offset && bytes.get(index) == Some(&b'\n') {
                    index += 1;
                }
            }
            b'\n' => {
                line = line.saturating_add(1);
                column = 1;
                index += 1;
            }
            _ => {
                column = column.saturating_add(1);
                index += 1;
            }
        }
    }
    Ok(LineColumn { line, column })
}
