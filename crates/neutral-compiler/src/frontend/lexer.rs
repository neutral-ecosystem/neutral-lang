// SPDX-License-Identifier: Apache-2.0

//! Raw lexer for source text, exact trivia, and physical line boundaries.

use super::{
    DecodedString, FrontendError, LexedSource, PhysicalLineEnd, Token, TokenKind, Trivia,
    TriviaKind, span,
};
use crate::language::names;

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
            byte if punctuation_token(byte).is_some() => {
                let kind = punctuation_token(byte).expect("matched punctuation must classify");
                tokens.push(token(kind, index, index + 1));
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
                tokens.push(token(word_token(text), index, end));
                index = end;
            }
            byte if starts_number(source, index, byte) => {
                let end = consume_number_candidate(source, index);
                let value = ascii_text(&source[index..end], index)?;
                tokens.push(token(TokenKind::Number(value), index, end));
                index = end;
            }
            _ => return Err(FrontendError::unsupported_symbol(span(index, index + 1))),
        }
    }

    tokens.push(token(TokenKind::EndOfFile, source.len(), source.len()));
    Ok(LexedSource { tokens, trivia })
}

/// Maps one active punctuation byte to its compiler-private token category.
fn punctuation_token(byte: u8) -> Option<TokenKind> {
    match byte {
        b'=' => Some(TokenKind::Equals),
        b'?' => Some(TokenKind::Question),
        b'{' => Some(TokenKind::OpenBrace),
        b'}' => Some(TokenKind::CloseBrace),
        b':' => Some(TokenKind::Colon),
        b',' => Some(TokenKind::Comma),
        b'<' => Some(TokenKind::Less),
        b'>' => Some(TokenKind::Greater),
        b'[' => Some(TokenKind::OpenBracket),
        b']' => Some(TokenKind::CloseBracket),
        b'(' => Some(TokenKind::OpenParen),
        b')' => Some(TokenKind::CloseParen),
        _ => None,
    }
}

/// Returns whether `byte` begins a candidate frozen numeric literal.
fn starts_number(source: &[u8], index: usize, byte: u8) -> bool {
    byte.is_ascii_digit()
        || matches!(byte, b'+' | b'-') && source.get(index + 1).is_some_and(u8::is_ascii_digit)
}

/// Consumes a numeric candidate for later exact semantic validation.
fn consume_number_candidate(source: &[u8], start: usize) -> usize {
    consume_while(source, start, |byte| {
        byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'.' | b'+' | b'-')
    })
}

/// Classifies one ASCII word through the centralized language-name namespace.
fn word_token(text: String) -> TokenKind {
    if text == names::NEU {
        TokenKind::Neu
    } else if text == names::MODULE {
        TokenKind::Module
    } else if text == names::RECORD {
        TokenKind::Record
    } else if text == names::LIST {
        TokenKind::List
    } else if text == names::REF_TYPE {
        TokenKind::RefType
    } else if text == names::REF {
        TokenKind::RefValue
    } else if text == names::NUM {
        TokenKind::Num
    } else if text == names::STRING {
        TokenKind::StringType
    } else if text == names::BOOL {
        TokenKind::BoolType
    } else if text == names::TRUE {
        TokenKind::True
    } else if text == names::FALSE {
        TokenKind::False
    } else if text == names::NULL {
        TokenKind::Null
    } else if names::is_protected_name(&text) {
        TokenKind::ProtectedName(text)
    } else {
        TokenKind::Identifier(text)
    }
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
fn string_literal(source: &[u8], start: usize) -> Result<(usize, DecodedString), FrontendError> {
    let mut index = start + 1;
    let mut value = String::new();
    let mut had_escape = false;
    while let Some(byte) = source.get(index).copied() {
        match byte {
            b'"' => {
                return Ok((index + 1, DecodedString { value, had_escape }));
            }
            b'\\' => {
                had_escape = true;
                index = decode_escape(source, index, &mut value)?;
            }
            b'\n' | b'\r' => {
                return Err(FrontendError::unterminated_string_literal(span(
                    start, index,
                )));
            }
            byte if byte.is_ascii_control() => {
                return Err(FrontendError::invalid_string_literal(span(
                    index,
                    index + 1,
                )));
            }
            byte if byte.is_ascii() => {
                value.push(char::from(byte));
                index += 1;
            }
            _ => {
                let remaining = std::str::from_utf8(&source[index..])
                    .expect("source UTF-8 was validated before string decoding");
                let character = remaining
                    .chars()
                    .next()
                    .expect("non-ASCII byte must start one character");
                let end = index + character.len_utf8();
                if character.is_control() {
                    return Err(FrontendError::invalid_string_literal(span(index, end)));
                }
                value.push(character);
                index = end;
            }
        }
    }
    Err(FrontendError::unterminated_string_literal(span(
        start,
        source.len(),
    )))
}

/// Decodes one frozen string escape and returns the next unread byte offset.
fn decode_escape(source: &[u8], start: usize, value: &mut String) -> Result<usize, FrontendError> {
    let Some(escaped) = source.get(start + 1).copied() else {
        return Err(FrontendError::unterminated_string_literal(span(
            start,
            source.len(),
        )));
    };
    let character = match escaped {
        b'"' => '"',
        b'\\' => '\\',
        b'n' => '\n',
        b'r' => '\r',
        b't' => '\t',
        b'0' => '\0',
        b'u' => return decode_unicode_escape(source, start, value),
        _ => {
            return Err(FrontendError::invalid_string_literal(span(
                start,
                (start + 2).min(source.len()),
            )));
        }
    };
    value.push(character);
    Ok(start + 2)
}

/// Decodes one `\u{HEX}` escape containing one to six hexadecimal digits.
fn decode_unicode_escape(
    source: &[u8],
    start: usize,
    value: &mut String,
) -> Result<usize, FrontendError> {
    if source.get(start + 2) != Some(&b'{') {
        return Err(FrontendError::invalid_string_literal(span(
            start,
            (start + 3).min(source.len()),
        )));
    }
    let digits_start = start + 3;
    let mut end = digits_start;
    while source.get(end).is_some_and(u8::is_ascii_hexdigit) && end - digits_start < 6 {
        end += 1;
    }
    if end == digits_start || source.get(end) != Some(&b'}') {
        return Err(FrontendError::invalid_string_literal(span(
            start,
            (end + 1).min(source.len()),
        )));
    }
    let digits = ascii_text(&source[digits_start..end], digits_start)?;
    let scalar = u32::from_str_radix(&digits, 16)
        .ok()
        .and_then(char::from_u32)
        .ok_or_else(|| FrontendError::invalid_string_literal(span(start, end + 1)))?;
    value.push(scalar);
    Ok(end + 1)
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
