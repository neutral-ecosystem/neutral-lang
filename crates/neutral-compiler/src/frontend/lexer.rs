// SPDX-License-Identifier: Apache-2.0

//! Raw lexer for source text, exact trivia, and physical line boundaries.

use super::{
    FrontendError, LexedSource, PhysicalLineEnd, Token, TokenKind, Trivia, TriviaKind, span,
};

/// UTF-8 byte-order mark accepted only once at byte offset zero.
const UTF8_BOM: &[u8; 3] = b"\xef\xbb\xbf";

/// Lexes exact captured bytes while retaining physical newline tokens and spans.
pub(super) fn lex(source: &[u8]) -> Result<LexedSource, FrontendError> {
    validate_source_text(source)?;
    let mut tokens = Vec::new();
    let mut trivia = Vec::new();
    let mut index = usize::from(source.starts_with(UTF8_BOM)) * UTF8_BOM.len();

    while index < source.len() {
        match source[index] {
            b' ' | b'\t' => {
                let end = consume_while(source, index, |value| matches!(value, b' ' | b'\t'));
                trivia.push(Trivia {
                    kind: TriviaKind::HorizontalWhitespace,
                    span: span(index, end),
                });
                index = end;
            }
            b'\n' => {
                tokens.push(token(
                    TokenKind::PhysicalLineEnd(PhysicalLineEnd::LineFeed),
                    index,
                    index + 1,
                ));
                index += 1;
            }
            b'\r' => {
                let end = if source.get(index + 1) == Some(&b'\n') {
                    index + 2
                } else {
                    index + 1
                };
                let kind = if end == index + 2 {
                    PhysicalLineEnd::CarriageReturnLineFeed
                } else {
                    PhysicalLineEnd::CarriageReturn
                };
                tokens.push(token(TokenKind::PhysicalLineEnd(kind), index, end));
                index = end;
            }
            b'=' => {
                tokens.push(token(TokenKind::Equals, index, index + 1));
                index += 1;
            }
            b'/' if source.get(index + 1) == Some(&b'/') => {
                let end = consume_while(source, index + 2, |value| !matches!(value, b'\n' | b'\r'));
                trivia.push(Trivia {
                    kind: TriviaKind::LineComment,
                    span: span(index, end),
                });
                index = end;
            }
            b'/' if source.get(index + 1) == Some(&b'*') => {
                let (end, line_ends) = block_comment(source, index)?;
                trivia.push(Trivia {
                    kind: TriviaKind::BlockComment,
                    span: span(index, end),
                });
                tokens.extend(line_ends);
                index = end;
            }
            b'"' => {
                let (next, value) = string_literal(source, index)?;
                tokens.push(token(TokenKind::StringLiteral(value), index, next));
                index = next;
            }
            byte if byte.is_ascii_alphabetic() || byte == b'_' => {
                let end = consume_while(source, index, |value| {
                    value.is_ascii_alphanumeric() || *value == b'_'
                });
                let text = ascii_text(&source[index..end], index)?;
                let kind = match text.as_str() {
                    "neu" => TokenKind::Neu,
                    "module" => TokenKind::Module,
                    "num" => TokenKind::Num,
                    "string" | "bool" | "List" | "Ref" | "use" | "record" | "true" | "false"
                    | "null" | "ref" => TokenKind::ProtectedName(text),
                    _ => TokenKind::Identifier(text),
                };
                tokens.push(token(kind, index, end));
                index = end;
            }
            byte if byte.is_ascii_digit() => {
                let end = consume_while(source, index, |value| {
                    value.is_ascii_alphanumeric() || *value == b'_'
                });
                let value = ascii_text(&source[index..end], index)?;
                let kind = if value.bytes().all(|byte| byte.is_ascii_digit()) {
                    TokenKind::Number(value)
                } else {
                    TokenKind::Identifier(value)
                };
                tokens.push(token(kind, index, end));
                index = end;
            }
            _ => return Err(FrontendError::unsupported_symbol(span(index, index + 1))),
        }
    }

    tokens.push(token(TokenKind::EndOfFile, source.len(), source.len()));
    Ok(LexedSource { tokens, trivia })
}

/// Reads one non-nesting block comment and retains any physical line endings.
fn block_comment(source: &[u8], start: usize) -> Result<(usize, Vec<Token>), FrontendError> {
    let mut index = start + 2;
    let mut line_ends = Vec::new();
    while index < source.len() {
        if source.get(index) == Some(&b'*') && source.get(index + 1) == Some(&b'/') {
            return Ok((index + 2, line_ends));
        }
        match source[index] {
            b'\n' => {
                line_ends.push(token(
                    TokenKind::PhysicalLineEnd(PhysicalLineEnd::LineFeed),
                    index,
                    index + 1,
                ));
                index += 1;
            }
            b'\r' => {
                let end = if source.get(index + 1) == Some(&b'\n') {
                    index + 2
                } else {
                    index + 1
                };
                let kind = if end == index + 2 {
                    PhysicalLineEnd::CarriageReturnLineFeed
                } else {
                    PhysicalLineEnd::CarriageReturn
                };
                line_ends.push(token(TokenKind::PhysicalLineEnd(kind), index, end));
                index = end;
            }
            _ => index += 1,
        }
    }
    Err(FrontendError::unterminated_block_comment(span(
        start,
        source.len(),
    )))
}

/// Rejects malformed UTF-8, unescaped NUL, and a BOM outside byte offset zero.
fn validate_source_text(source: &[u8]) -> Result<(), FrontendError> {
    if let Some(index) = source.iter().position(|byte| *byte == 0) {
        return Err(FrontendError::other(span(index, index + 1)));
    }
    if let Err(error) = std::str::from_utf8(source) {
        let start = error.valid_up_to();
        let end = start
            .saturating_add(error.error_len().unwrap_or(1))
            .min(source.len());
        return Err(FrontendError::other(span(start, end)));
    }
    let search_start = usize::from(source.starts_with(UTF8_BOM)) * UTF8_BOM.len();
    if let Some(relative) = source[search_start..]
        .windows(UTF8_BOM.len())
        .position(|window| window == UTF8_BOM)
    {
        let start = search_start + relative;
        return Err(FrontendError::other(span(start, start + UTF8_BOM.len())));
    }
    Ok(())
}

/// Reads one simple, unescaped string literal without crossing a physical line.
fn string_literal(source: &[u8], start: usize) -> Result<(usize, String), FrontendError> {
    let mut end = start + 1;
    while let Some(byte) = source.get(end) {
        match *byte {
            b'"' => {
                let value = ascii_text(&source[start + 1..end], start + 1)?;
                return Ok((end + 1, value));
            }
            b'\n' | b'\r' | b'\\' => {
                return Err(FrontendError::malformed_boundary(span(start, end + 1)));
            }
            _ => end += 1,
        }
    }
    Err(FrontendError::malformed_boundary(span(start, source.len())))
}

/// Advances over bytes while the supplied ASCII predicate remains true.
fn consume_while(source: &[u8], start: usize, predicate: fn(&u8) -> bool) -> usize {
    let mut end = start;
    while source.get(end).is_some_and(predicate) {
        end += 1;
    }
    end
}

/// Converts validated ASCII token bytes into owned text.
fn ascii_text(bytes: &[u8], start: usize) -> Result<String, FrontendError> {
    if !bytes.is_ascii() {
        return Err(FrontendError::other(span(start, start + bytes.len())));
    }
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

/// Creates one raw token over an exact original-byte span.
fn token(kind: TokenKind, start: usize, end: usize) -> Token {
    Token {
        kind,
        span: span(start, end),
    }
}
