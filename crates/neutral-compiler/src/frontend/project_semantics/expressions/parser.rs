// SPDX-License-Identifier: Apache-2.0

//! Successor-only lexical boundaries; historical token acceptance is unchanged.
//!
//! Tokens borrow the captured source and keep original-byte ranges. String decoding
//! and owned syntax retention use the request's fallible allocation/work budget.
//! This module recognizes syntax only: symbol visibility, all-branch typing and
//! template hygiene belong to resolution, not to lexical acceptance.

use super::{
    Budget, ByteSpan, E, ExactNumber, Expression, ParameterPhase, Root, SourceLocation, State,
    Token, TokenKind, TryClone, TypeExpr, alloc, composition_error, copy_slice, error,
    expression_profile, grow, is_protected_name, is_snake_name, is_upper_name, lexer,
    parse_complete_type_fallible, symbol, text,
};
use crate::frontend::expression_codes as ec;

/// Original source spelling and span; decoding happens only at the typed literal boundary.
#[derive(Clone, Copy)]
pub(super) struct Lex<'a> {
    /// Borrowed exact source spelling, including string escapes.
    pub spelling: &'a str,
    /// Original captured half-open byte range.
    pub span: ByteSpan,
}
impl neutral_core::allocation::TryClone for Lex<'_> {
    /// Copies a borrowed nonallocating token.
    fn try_clone(&self) -> Result<Self, neutral_core::allocation::AllocationError> {
        Ok(*self)
    }
}

/// One initializer and optional immutable axis substitution frame.
pub(super) struct BindingSyntax<'a> {
    /// Original initializer tokens; empty only for explicit inputs or expanded cached bodies.
    pub tokens: Vec<Lex<'a>>,
    /// Explicit external-input declaration, without initializer.
    pub input: bool,
    /// Original axis names with already resolved tuple values.
    pub axes: Vec<(&'a str, Expression)>,
}
/// Parameter syntax retains definition scope and explicit source defaults.
pub(super) struct ParameterSyntax<'a> {
    /// Semantic named argument spelling.
    pub name: &'a str,
    /// Explicit static/deferred contract.
    pub phase: ParameterPhase,
    /// Definition-scoped original type syntax.
    pub ty: TypeExpr,
    /// Original default tokens, empty when no default exists.
    pub default: Vec<Lex<'a>>,
    /// Original complete parameter occurrence.
    pub location: SourceLocation,
}
/// Source definitions are separate from exported normalized template contracts.
pub(super) struct TemplateSyntax<'a> {
    /// Stable definition owner, visibility, type and original occurrence.
    pub root: Root,
    /// Parameters in declared order, before substitution.
    pub parameters: Vec<ParameterSyntax<'a>>,
    /// Original expression body, never executable vocabulary text.
    pub body: Vec<Lex<'a>>,
}
/// A finite matrix retains original source axes and optional shape decisions.
pub(super) struct MatrixSyntax<'a> {
    /// Original application owner and occurrence; no synthetic matrix binding is published.
    pub root: Root,
    /// Original qualified template name.
    pub template: Vec<Lex<'a>>,
    /// Caller-scoped named application expressions.
    pub arguments: Vec<Lex<'a>>,
    /// Axes in source order; positions determine generated names.
    pub axes: Vec<(&'a str, Vec<Lex<'a>>)>,
    /// Optional original static Boolean filter.
    pub filter: Vec<Lex<'a>>,
    /// Optional original complete exclusion tuples.
    pub exclusions: Vec<Lex<'a>>,
}

/// Tokenizes expression syntax into borrowed spellings with original byte spans.
///
/// `offset` locates this slice inside the captured source. Newlines remain tokens
/// until declaration parsing has balanced groups; comments never become tokens.
/// Names and operator spelling are recognized here, before typed resolution.
///
/// # Errors
/// Rejects malformed UTF-8, numbers, strings, interpolation, unknown punctuation,
/// allocation failure, exhausted caller budgets, or cancellation.
#[allow(clippy::too_many_lines)] // One contextual byte dispatch keeps cursor and depth accounting explicit.
pub(super) fn lex<'a>(
    bytes: &'a [u8],
    offset: usize,
    state: &mut Budget<'_>,
) -> Result<Vec<Lex<'a>>, E> {
    let source = std::str::from_utf8(bytes).map_err(|_| error(ec::EXP_SYNTAX, None))?;
    // Charge the complete slice once, including skipped whitespace and comments.
    state.charge(bytes.len() as u64)?;
    let mut result = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        state.charge(1)?;
        let start = i;
        match bytes[i] {
            b' ' | b'\t' => {
                i += 1;
                continue;
            }
            b'\r' | b'\n' => {
                i += 1;
                if bytes[start] == b'\r' && bytes.get(i) == Some(&b'\n') {
                    i += 1;
                }
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && !matches!(bytes[i], b'\n' | b'\r') {
                    i += 1;
                }
                continue;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < bytes.len() && &bytes[i..i + 2] != b"*/" {
                    state.charge(1)?;
                    i += 1;
                }
                if i + 1 >= bytes.len() {
                    return Err(error(ec::EXP_SYNTAX, None));
                }
                i += 2;
                continue;
            }
            b'"' => {
                i = string_end(source, i, false, state)?;
            }
            b'$' if bytes.get(i + 1) == Some(&b'"') => {
                i = string_end(source, i + 1, true, state)?;
            }
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                i += 1;
                while bytes
                    .get(i)
                    .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
                {
                    i += 1;
                }
            }
            // Leading signs remain operators; only an exponent sign belongs to this token.
            b'0'..=b'9' => {
                i += 1;
                while bytes
                    .get(i)
                    .is_some_and(|b| b.is_ascii_digit() || matches!(b, b'_' | b'.'))
                {
                    i += 1;
                }
                if bytes.get(i).is_some_and(|b| matches!(b, b'e' | b'E')) {
                    i += 1;
                    if bytes.get(i).is_some_and(|b| matches!(b, b'+' | b'-')) {
                        i += 1;
                    }
                    while bytes
                        .get(i)
                        .is_some_and(|b| b.is_ascii_digit() || *b == b'_')
                    {
                        i += 1;
                    }
                }
                ExactNumber::from_source(
                    &source[start..i],
                    state.expression.coefficient_digits,
                    state.expression.absolute_scale,
                )
                .map_err(|e| {
                    error(
                        if e == neutral_ir::IrError::ExactNumberLimitExceeded {
                            ec::EXP_LIMIT
                        } else {
                            ec::EXP_SYNTAX
                        },
                        None,
                    )
                })?;
            }
            _ => {
                // Prefer a complete two-byte operator over its one-byte prefix.
                if i + 1 < bytes.len()
                    && matches!(
                        &bytes[i..i + 2],
                        b"::"
                            | b"->"
                            | b"=>"
                            | b"++"
                            | b"<="
                            | b">="
                            | b"=="
                            | b"!="
                            | b"&&"
                            | b"||"
                    )
                {
                    i += 2;
                } else if b"{}[]():,?=<>!+-*/".contains(&bytes[i]) {
                    i += 1;
                } else {
                    return Err(error(ec::EXP_SYNTAX, None));
                }
            }
        }
        let span = ByteSpan::new((start + offset) as u64, (i + offset) as u64)
            .or(Err(error(ec::EXP_LIMIT, None)))?;
        state.retain::<Lex<'_>>(1)?;
        grow(&mut result, 1)?;
        result.push(Lex {
            spelling: &source[start..i],
            span,
        });
    }
    Ok(result)
}

/// Finds the end of a quoted or interpolated string without decoding its value.
///
/// Nested `${...}` groups are scanned only in interpolation mode. Escaped quotes
/// cannot terminate the string, and raw line endings are always syntax errors.
fn string_end(
    source: &str,
    quote: usize,
    interpolation: bool,
    state: &mut Budget<'_>,
) -> Result<usize, E> {
    let bytes = source.as_bytes();
    let mut i = quote + 1;
    while i < bytes.len() {
        state.charge(1)?;
        match bytes[i] {
            b'\\' => {
                i += 2;
            }
            b'"' => return Ok(i + 1),
            b'\r' | b'\n' => return Err(error(ec::EXP_SYNTAX, None)),
            b'$' if interpolation && bytes.get(i + 1) == Some(&b'{') => {
                i = interpolation_end(source, i + 2, state)? + 1;
            }
            _ => i += 1,
        }
    }
    Err(error(ec::EXP_SYNTAX, None))
}
/// Returns the byte index of the matching interpolation close.
///
/// Start at the first byte after `${`. Nested braces consume syntax depth;
/// braces inside quoted or interpolated strings belong to those strings.
///
/// # Errors
/// Rejects an unterminated group, excessive nesting, exhausted work or cancellation.
pub(super) fn interpolation_end(
    source: &str,
    mut i: usize,
    state: &mut Budget<'_>,
) -> Result<usize, E> {
    let bytes = source.as_bytes();
    // The opening `${` was consumed by the caller and still contributes one nesting level.
    let mut depth = 1;
    while i < bytes.len() {
        state.charge(1)?;
        match bytes[i] {
            b'"' => i = string_end(source, i, false, state)?,
            b'$' if bytes.get(i + 1) == Some(&b'"') => i = string_end(source, i + 1, true, state)?,
            b'{' => {
                depth += 1;
                if depth > state.expression.syntax_depth {
                    return Err(error(ec::EXP_LIMIT, None));
                }
                i += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i);
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    Err(error(ec::EXP_SYNTAX, None))
}

/// Decodes an ordinary literal using the shared source-string rules.
///
/// Interpolation removes its own extra escape before calling this helper. The
/// decoded bytes are charged separately from the borrowed original token text.
pub(super) fn string(raw: &str, state: &mut Budget<'_>) -> Result<String, E> {
    let tokens = lexer::lex_bounded(
        raw.as_bytes(),
        usize::try_from(state.expression.retained_bytes).unwrap_or(usize::MAX),
        Some(state.cancel),
    )
    .map_err(|_| {
        error(
            if state.cancel.is_cancelled() {
                ec::EXP_CANCELLED
            } else {
                ec::EXP_SYNTAX
            },
            None,
        )
    })?
    .tokens;
    let Some(Token {
        kind: TokenKind::StringLiteral(value),
        ..
    }) = tokens.first()
    else {
        return Err(error(ec::EXP_SYNTAX, None));
    };
    state.bytes(value.value.len() as u64)?;
    alloc(text(&value.value))
}

/// Converts a source type using the shared historical type parser, with original spans.
pub(super) fn ty(tokens: &[Lex<'_>], source: &[u8]) -> Result<TypeExpr, E> {
    let Some(first) = tokens.first() else {
        return Err(error(ec::EXP_SYNTAX, None));
    };
    let end = tokens.last().ok_or(error(ec::EXP_SYNTAX, None))?.span.end();
    let start = first.span.start();
    let range = usize::try_from(start).or(Err(error(ec::EXP_LIMIT, None)))?
        ..usize::try_from(end).or(Err(error(ec::EXP_LIMIT, None)))?;
    let mut tokens = lexer::lex_bounded(&source[range], usize::MAX, None)
        .map_err(|_| error(ec::EXP_SYNTAX, None))?
        .tokens;
    tokens.retain(|t| !matches!(t.kind, TokenKind::EndOfFile | TokenKind::PhysicalLineEnd(_)));
    for token in &mut tokens {
        token.span = ByteSpan::new(token.span.start() + start, token.span.end() + start)
            .or(Err(error(ec::EXP_LIMIT, None)))?;
    }
    alloc(parse_complete_type_fallible(&tokens))?.ok_or_else(|| error(ec::EXP_SYNTAX, None))
}

/// Reads exactly one local or import-qualified source name.
pub(super) fn name<'a>(
    tokens: &[Lex<'a>],
    cursor: &mut usize,
) -> Result<(Option<&'a str>, &'a str, ByteSpan), E> {
    let first = tokens
        .get(*cursor)
        .ok_or_else(|| error(ec::EXP_SYNTAX, None))?;
    if !identifier(first.spelling) {
        return Err(error(ec::EXP_SYNTAX, None));
    }
    *cursor += 1;
    if tokens.get(*cursor).is_some_and(|t| t.spelling == "::") {
        *cursor += 1;
        let second = tokens
            .get(*cursor)
            .ok_or_else(|| error(ec::EXP_SYNTAX, None))?;
        if !identifier(second.spelling) {
            return Err(error(ec::EXP_SYNTAX, None));
        }
        *cursor += 1;
        Ok((Some(first.spelling), second.spelling, second.span))
    } else {
        Ok((None, first.spelling, first.span))
    }
}
/// Recognizes inert ASCII name tokens before context-specific spelling validation.
pub(super) fn identifier(s: &str) -> bool {
    s.as_bytes()
        .first()
        .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
        && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
/// Consumes one required contextual punctuation or keyword.
pub(super) fn expect(tokens: &[Lex<'_>], cursor: &mut usize, spelling: &str) -> Result<(), E> {
    if tokens.get(*cursor).is_none_or(|t| t.spelling != spelling) {
        return Err(error(ec::EXP_SYNTAX, None));
    }
    *cursor += 1;
    Ok(())
}
/// Borrows one balanced group and advances the caller past its closing delimiter.
///
/// Repeated matching delimiters increase depth; neither this helper nor `split`
/// resolves names or evaluates the expressions inside a group.
pub(super) fn group<'t, 'a>(
    tokens: &'t [Lex<'a>],
    cursor: &mut usize,
    open: &str,
    close: &str,
) -> Result<&'t [Lex<'a>], E> {
    expect(tokens, cursor, open)?;
    let start = *cursor;
    let mut depth = 1;
    while *cursor < tokens.len() {
        let spelling = tokens[*cursor].spelling;
        if spelling == open {
            depth += 1;
        }
        if spelling == close {
            depth -= 1;
        }
        if depth == 0 {
            let end = *cursor;
            *cursor += 1;
            return Ok(&tokens[start..end]);
        }
        *cursor += 1;
    }
    Err(error(ec::EXP_SYNTAX, None))
}
/// Splits top-level comma members without changing source evaluation order.
pub(super) fn split<'t, 'a>(tokens: &'t [Lex<'a>]) -> Result<Vec<&'t [Lex<'a>]>, E> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    for (i, t) in tokens.iter().enumerate() {
        match t.spelling {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => {
                if depth == 0 {
                    return Err(error(ec::EXP_SYNTAX, None));
                }
                depth -= 1;
            }
            _ => {}
        }
        if depth == 0 && t.spelling == "," {
            if start == i {
                return Err(error(ec::EXP_SYNTAX, None));
            }
            grow(&mut result, 1)?;
            result.push(&tokens[start..i]);
            start = i + 1;
        }
    }
    if depth != 0 {
        return Err(error(ec::EXP_SYNTAX, None));
    }
    if start < tokens.len() {
        grow(&mut result, 1)?;
        result.push(&tokens[start..]);
    }
    Ok(result)
}

/// Retains every original declaration before name resolution or evaluation.
///
/// A physical newline completes a statement only outside balanced groups.
/// Matrix continuation clauses belong to their preceding expansion. A final
/// statement without a line ending follows the same declaration checks.
///
/// # Errors
/// Rejects misplaced headers, mismatched groups, malformed declarations and
/// exhausted request-local work, storage or cancellation controls.
pub(super) fn parse_source<'a>(
    source: &'a crate::CapturedProjectSource,
    state: &mut State<'a, '_>,
) -> Result<(), E> {
    let tokens = lex(source.bytes(), 0, &mut state.budget)?;
    let mut statement: Vec<Lex<'_>> = Vec::new();
    let mut depth = 0_u64;
    let mut headers_done = false;
    for (i, token) in tokens.iter().enumerate() {
        if token.spelling.starts_with(['\n', '\r']) {
            // Matrix clauses may start on the next physical line without ending the expansion.
            let continuation = tokens[i + 1..]
                .iter()
                .find(|t| !t.spelling.starts_with(['\n', '\r']))
                .is_some_and(|t| matches!(t.spelling, "for" | "where" | "exclude"));
            if depth == 0 && !statement.is_empty() && !continuation {
                let header = matches!(statement[0].spelling, "neu" | "module" | "use" | "import");
                if header && headers_done {
                    return Err(error(ec::EXP_SYNTAX, None));
                }
                headers_done |= !header;
                declaration(source, &statement, state)?;
                statement.clear();
            }
            continue;
        }
        match token.spelling {
            "(" | "[" | "{" => depth += 1,
            ")" | "]" | "}" => {
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| error(ec::EXP_SYNTAX, None))?;
            }
            _ => {}
        }
        if depth > state.budget.expression.syntax_depth {
            return Err(error(ec::EXP_LIMIT, None));
        }
        grow(&mut statement, 1)?;
        statement.push(*token);
    }
    if depth != 0 {
        return Err(error(ec::EXP_SYNTAX, None));
    }
    // EOF is also a statement boundary; a final newline is not required.
    if !statement.is_empty() {
        if headers_done && matches!(statement[0].spelling, "neu" | "module" | "use" | "import") {
            return Err(error(ec::EXP_SYNTAX, None));
        }
        declaration(source, &statement, state)?;
    }
    Ok(())
}

/// Retains one complete source declaration without evaluating its initializer.
fn declaration<'a>(
    source: &'a crate::CapturedProjectSource,
    tokens: &[Lex<'a>],
    state: &mut State<'a, '_>,
) -> Result<(), E> {
    if matches!(tokens[0].spelling, "neu" | "module" | "use" | "import") {
        return Ok(());
    }
    let public = tokens[0].spelling == "public";
    let start = usize::from(public);
    let location = SourceLocation::new(
        source.digest(),
        ByteSpan::new(
            tokens[0].span.start(),
            tokens.last().ok_or(error(ec::EXP_SYNTAX, None))?.span.end(),
        )
        .or(Err(error(ec::EXP_LIMIT, None)))?,
    );
    if matches!(
        tokens.get(start).map(|t| t.spelling),
        Some("record" | "variant")
    ) {
        let mut old = lexer::lex_bounded(
            &source.bytes()[usize::try_from(location.span().start())
                .or(Err(error(ec::EXP_LIMIT, Some(location))))?
                ..usize::try_from(location.span().end())
                    .or(Err(error(ec::EXP_LIMIT, Some(location))))?],
            usize::try_from(state.budget.expression.retained_bytes).unwrap_or(usize::MAX),
            Some(state.budget.cancel),
        )
        .map_err(|_| error(ec::EXP_SYNTAX, Some(location)))?
        .tokens;
        old.retain(|t| !matches!(t.kind, TokenKind::EndOfFile | TokenKind::PhysicalLineEnd(_)));
        for t in &mut old {
            t.span = ByteSpan::new(
                t.span.start() + location.span().start(),
                t.span.end() + location.span().start(),
            )
            .or(Err(error(ec::EXP_LIMIT, Some(location))))?;
        }
        let root = super::super::nominal::syntax::parse_declaration(
            source,
            &old,
            state.captured.composition_limits(),
        )
        .map_err(composition_error)?;
        state.insert_root(root)?;
        return Ok(());
    }
    if tokens.get(start).is_some_and(|t| t.spelling == "template") {
        return template(source, tokens, start, public, location, state);
    }
    if tokens.get(start).is_some_and(|t| t.spelling == "expand") {
        return matrix(source, tokens, start, public, location, state);
    }
    let input = tokens.get(start).is_some_and(|t| t.spelling == "input");
    let begin = start + usize::from(input);
    let end = if input {
        tokens.len()
    } else {
        tokens
            .iter()
            .position(|t| t.spelling == "=")
            .ok_or_else(|| error(ec::EXP_SYNTAX, Some(location)))?
    };
    if end <= begin + 1 {
        return Err(error(ec::EXP_SYNTAX, Some(location)));
    }
    let name = tokens[end - 1].spelling;
    if !expression_profile::is_binding_name(name) || is_protected_name(name) {
        return Err(error(ec::EXP_SYNTAX, Some(location)));
    }
    let root = Root {
        owner: symbol(source.module_id(), name)?,
        public,
        location,
        ty: Some(ty(&tokens[begin..end - 1], source.bytes())?),
        fields: Vec::new(),
        tokens: Vec::new(),
        variant: false,
    };
    let owner = alloc(root.owner.try_clone())?;
    state.insert_root(root)?;
    let syntax = BindingSyntax {
        tokens: if input {
            Vec::new()
        } else {
            alloc(copy_slice(&tokens[end + 1..]))?
        },
        input,
        axes: Vec::new(),
    };
    alloc(state.bindings.insert(owner, syntax))?;
    Ok(())
}
/// Retains a template signature, ordered parameter defaults and unevaluated body.
///
/// Required parameters cannot follow defaults, and names must be unique before
/// any caller arguments or expansion tuples can be considered.
fn template<'a>(
    source: &'a crate::CapturedProjectSource,
    tokens: &[Lex<'a>],
    start: usize,
    public: bool,
    location: SourceLocation,
    state: &mut State<'a, '_>,
) -> Result<(), E> {
    let name = tokens
        .get(start + 1)
        .ok_or_else(|| error(ec::TPL_SYNTAX, Some(location)))?
        .spelling;
    if !is_upper_name(name) || is_protected_name(name) {
        return Err(error(ec::TPL_NAME, Some(location)));
    }
    let mut cursor = start + 2;
    let members = group(tokens, &mut cursor, "(", ")")?;
    let mut parameters = Vec::new();
    // Preserve declaration order so a later required parameter cannot silently shift arguments.
    let mut default_seen = false;
    for member in split(members)? {
        let phase = match member.first().map(|t| t.spelling) {
            Some("static") => ParameterPhase::Static,
            Some("deferred") => ParameterPhase::Deferred,
            _ => return Err(error(ec::TPL_SYNTAX, Some(location))),
        };
        let end = member
            .iter()
            .position(|t| t.spelling == "=")
            .unwrap_or(member.len());
        if end < 3 {
            return Err(error(ec::TPL_SYNTAX, Some(location)));
        }
        let name = member[end - 1].spelling;
        if !is_snake_name(name)
            || is_protected_name(name)
            || parameters
                .iter()
                .any(|p: &ParameterSyntax<'_>| p.name == name)
        {
            return Err(error(ec::TPL_NAME, Some(location)));
        }
        if default_seen && end == member.len() {
            return Err(error(ec::TPL_NAME, Some(location)));
        }
        default_seen |= end < member.len();
        if parameters.len() as u64 >= state.budget.expansion.parameters {
            return Err(error(ec::TPL_LIMIT, Some(location)));
        }
        grow(&mut parameters, 1)?;
        parameters.push(ParameterSyntax {
            name,
            phase,
            ty: ty(&member[1..end - 1], source.bytes())?,
            default: if end < member.len() {
                alloc(copy_slice(&member[end + 1..]))?
            } else {
                Vec::new()
            },
            location: SourceLocation::new(
                source.digest(),
                ByteSpan::new(
                    member[0].span.start(),
                    member
                        .last()
                        .ok_or(error(ec::TPL_SYNTAX, Some(location)))?
                        .span
                        .end(),
                )
                .or(Err(error(ec::TPL_LIMIT, Some(location))))?,
            ),
        });
    }
    expect(tokens, &mut cursor, "->")?;
    let open = tokens[cursor..]
        .iter()
        .position(|t| t.spelling == "{")
        .ok_or_else(|| error(ec::TPL_SYNTAX, Some(location)))?
        + cursor;
    let result = ty(&tokens[cursor..open], source.bytes())?;
    cursor = open;
    let body = alloc(copy_slice(group(tokens, &mut cursor, "{", "}")?))?;
    if cursor != tokens.len() {
        return Err(error(ec::TPL_SYNTAX, Some(location)));
    }
    let root = Root {
        owner: symbol(source.module_id(), name)?,
        public,
        location,
        ty: Some(result),
        fields: Vec::new(),
        tokens: Vec::new(),
        variant: false,
    };
    let owner = alloc(root.owner.try_clone())?;
    state.check_name(&owner)?;
    if state.templates.len() as u64 >= state.budget.expansion.definitions {
        return Err(error(ec::TPL_LIMIT, Some(location)));
    }
    alloc(state.templates.insert(
        owner,
        TemplateSyntax {
            root,
            parameters,
            body,
        },
    ))?;
    Ok(())
}
/// Parses a finite matrix's application, axes, optional filter and exclusions.
fn matrix<'a>(
    source: &'a crate::CapturedProjectSource,
    tokens: &[Lex<'a>],
    start: usize,
    public: bool,
    location: SourceLocation,
    state: &mut State<'a, '_>,
) -> Result<(), E> {
    let name = tokens
        .get(start + 1)
        .ok_or_else(|| error(ec::TPL_SYNTAX, Some(location)))?
        .spelling;
    if !is_snake_name(name) || is_protected_name(name) {
        return Err(error(ec::TPL_NAME, Some(location)));
    }
    let mut cursor = start + 2;
    expect(tokens, &mut cursor, "=")?;
    let template_start = cursor;
    name_fn(tokens, &mut cursor)?;
    let template = alloc(copy_slice(&tokens[template_start..cursor]))?;
    let arguments = alloc(copy_slice(group(tokens, &mut cursor, "(", ")")?))?;
    expect(tokens, &mut cursor, "for")?;
    let axis_tokens = group(tokens, &mut cursor, "(", ")")?;
    let mut axes = Vec::new();
    for axis in split(axis_tokens)? {
        if axis.len() < 3
            || axis[1].spelling != "in"
            || !is_snake_name(axis[0].spelling)
            || axes
                .iter()
                .any(|a: &(&str, Vec<Lex<'_>>)| a.0 == axis[0].spelling)
        {
            return Err(error(ec::TPL_AXIS, Some(location)));
        }
        grow(&mut axes, 1)?;
        axes.push((axis[0].spelling, alloc(copy_slice(&axis[2..]))?));
    }
    if axes.is_empty() || axes.len() as u64 > state.budget.expansion.axes {
        return Err(error(ec::TPL_AXIS, Some(location)));
    }
    let mut filter = Vec::new();
    if tokens.get(cursor).is_some_and(|t| t.spelling == "where") {
        cursor += 1;
        let end = tokens[cursor..]
            .iter()
            .position(|t| t.spelling == "exclude")
            .map_or(tokens.len(), |i| i + cursor);
        filter = alloc(copy_slice(&tokens[cursor..end]))?;
        cursor = end;
    }
    let mut exclusions = Vec::new();
    if tokens.get(cursor).is_some_and(|t| t.spelling == "exclude") {
        cursor += 1;
        exclusions = alloc(copy_slice(&tokens[cursor..]))?;
        cursor = tokens.len();
    }
    if cursor != tokens.len() {
        return Err(error(ec::TPL_SYNTAX, Some(location)));
    }
    let root = Root {
        owner: symbol(source.module_id(), name)?,
        public,
        location,
        ty: None,
        fields: Vec::new(),
        tokens: Vec::new(),
        variant: false,
    };
    state.check_name(&root.owner)?;
    grow(&mut state.matrices, 1)?;
    state.matrices.push(MatrixSyntax {
        root,
        template,
        arguments,
        axes,
        filter,
        exclusions,
    });
    Ok(())
}
/// Advances past one checked qualified name without retaining another spelling.
fn name_fn(tokens: &[Lex<'_>], cursor: &mut usize) -> Result<(), E> {
    name(tokens, cursor).map(|_| ())
}

#[cfg(test)]
#[path = "../../../../tests/expression_source/parser.rs"]
mod tests;
