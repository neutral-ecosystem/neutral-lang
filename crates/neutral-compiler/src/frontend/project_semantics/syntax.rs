// SPDX-License-Identifier: Apache-2.0

//! Extract declaration roots and type/value shapes from captured tokens.
//!
//! Parsing retains symbol locations and visibility for semantic diagnostics,
//! while imports and requirements remain graph/header concerns. The fallible
//! type parser is shared with composition parsing; name lookup and value typing
//! belong to resolution, not to these syntax recognizers.

use super::{
    BTreeSet, ByteSpan, LogicalModuleIdentity, MAX_PROJECT_INTERFACE_TYPE_DEPTH,
    ModuleSymbolIdentity, ProjectLocationValue, ProjectSemanticFailure, ProjectSymbol,
    ProjectSymbolKind, Root, SourceContentDigest, SourceLocation, Token, TokenKind, TypeExpr,
    V1_SOURCE_PROFILE, diagnostic, diagnostics, graph_names, lexer, parse_name, parse_reference,
    single,
};

/// Parses root declarations with the existing byte-accurate lexer.
pub(super) fn parse_roots(
    module: &str,
    digest: SourceContentDigest,
    bytes: &[u8],
) -> Result<Vec<Root>, ProjectSemanticFailure> {
    let lexed = lexer::lex(bytes).map_err(|error| {
        single(diagnostic(
            diagnostics::INACCESSIBLE_NAME,
            module,
            digest,
            error.span,
        ))
    })?;
    let mut roots = Vec::new();
    let mut statement = Vec::new();
    let mut depth = 0_u64;
    let mut header_lines = 0_u8;
    for token in lexed.tokens {
        if matches!(
            token.kind,
            TokenKind::EndOfFile | TokenKind::PhysicalLineEnd(_)
        ) && depth == 0
        {
            if !statement.is_empty() {
                if header_lines < 2 {
                    header_lines += 1;
                } else if !is_requirement_or_import(&statement) {
                    roots.push(parse_root(module, digest, &statement)?);
                }
                statement.clear();
            }
            continue;
        }
        if matches!(
            token.kind,
            TokenKind::EndOfFile | TokenKind::PhysicalLineEnd(_)
        ) {
            continue;
        }
        match token.kind {
            TokenKind::OpenBrace | TokenKind::OpenBracket | TokenKind::OpenParen => depth += 1,
            TokenKind::CloseBrace | TokenKind::CloseBracket | TokenKind::CloseParen => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
        statement.push(token);
    }
    if !statement.is_empty() {
        return Err(single(diagnostic(
            diagnostics::INACCESSIBLE_NAME,
            module,
            digest,
            statement[0].span,
        )));
    }
    Ok(roots)
}

/// Identifies already graph-validated vocabulary and import statements.
pub(super) fn is_requirement_or_import(tokens: &[Token]) -> bool {
    matches!(tokens[0].kind, TokenKind::Use)
        || matches!(&tokens[0].kind, TokenKind::Identifier(word) if word == graph_names::IMPORT)
}

/// Parses one complete declaration, preserving exact source locations.
#[expect(
    clippy::too_many_lines,
    reason = "one declaration parser retains the exact source span and failure boundary"
)]
fn parse_root(
    module: &str,
    digest: SourceContentDigest,
    tokens: &[Token],
) -> Result<Root, ProjectSemanticFailure> {
    let mut start = 0;
    let public =
        matches!(&tokens[0].kind, TokenKind::Identifier(word) if word == graph_names::PUBLIC);
    if public {
        start = 1;
    }
    let invalid = || {
        single(diagnostic(
            diagnostics::INVALID_SOURCE,
            module,
            digest,
            tokens[0].span,
        ))
    };
    if start >= tokens.len() {
        return Err(invalid());
    }
    if matches!(&tokens[start].kind, TokenKind::Identifier(word) if word == graph_names::PUBLIC) {
        return Err(single(diagnostic(
            diagnostics::INVALID_PUBLIC,
            module,
            digest,
            tokens[start].span,
        )));
    }
    let span = ByteSpan::new(tokens[0].span.start(), tokens[tokens.len() - 1].span.end())
        .expect("ordered declaration span");
    if matches!(tokens[start].kind, TokenKind::Record) {
        let Some(Token {
            kind: TokenKind::Identifier(name),
            ..
        }) = tokens.get(start + 1)
        else {
            return Err(invalid());
        };
        if !matches!(
            tokens.get(start + 2).map(|token| &token.kind),
            Some(TokenKind::OpenBrace)
        ) || !matches!(
            tokens.last().map(|token| &token.kind),
            Some(TokenKind::CloseBrace)
        ) {
            return Err(invalid());
        }
        let fields = parse_fields(&tokens[start + 3..tokens.len() - 1]).ok_or_else(invalid)?;
        return Ok(Root {
            symbol: symbol(
                module,
                digest,
                span,
                name,
                ProjectSymbolKind::Record,
                public,
            ),
            declared_type: None,
            fields,
            value: tokens[start + 3..tokens.len() - 1].to_vec(),
            location: None,
        });
    }
    let Some(eq) = tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::Equals))
    else {
        return Err(invalid());
    };
    if eq <= start + 1 || eq + 1 >= tokens.len() {
        return Err(invalid());
    }
    let TokenKind::Identifier(name) = &tokens[eq - 1].kind else {
        return Err(invalid());
    };
    let ty = parse_complete_type(&tokens[start..eq - 1]).ok_or_else(invalid)?;
    if !validate_value(&tokens[eq + 1..]) {
        return Err(invalid());
    }
    let location = match (&ty, &tokens[eq + 1..]) {
        (
            TypeExpr::Url,
            [
                Token {
                    kind: TokenKind::StringLiteral(value),
                    ..
                },
            ],
        ) => Some(ProjectLocationValue::Url(value.value.clone())),
        (
            TypeExpr::Path,
            [
                Token {
                    kind: TokenKind::StringLiteral(value),
                    ..
                },
            ],
        ) => Some(ProjectLocationValue::Path(value.value.clone())),
        (TypeExpr::Url | TypeExpr::Path, _) => return Err(invalid()),
        _ => None,
    };
    Ok(Root {
        symbol: symbol(
            module,
            digest,
            span,
            name,
            ProjectSymbolKind::Binding,
            public,
        ),
        declared_type: Some(ty),
        fields: Vec::new(),
        value: tokens[eq + 1..].to_vec(),
        location,
    })
}

/// Builds one stable module-symbol identity from a captured declaration.
fn symbol(
    module: &str,
    digest: SourceContentDigest,
    span: ByteSpan,
    name: &str,
    kind: ProjectSymbolKind,
    is_public: bool,
) -> ProjectSymbol {
    ProjectSymbol {
        identity: ModuleSymbolIdentity::new(
            LogicalModuleIdentity::new(V1_SOURCE_PROFILE, module),
            name,
        ),
        kind,
        is_public,
        source: SourceLocation::new(digest, span),
    }
}

/// Parses comma-separated fields with an optional final comma.
fn parse_fields(tokens: &[Token]) -> Option<Vec<(String, TypeExpr)>> {
    let mut fields = Vec::new();
    let mut names = BTreeSet::new();
    let mut start = 0;
    let mut depth = 0_u64;
    for (index, token) in tokens.iter().enumerate() {
        match token.kind {
            TokenKind::Less
            | TokenKind::OpenBrace
            | TokenKind::OpenBracket
            | TokenKind::OpenParen => depth += 1,
            TokenKind::Greater
            | TokenKind::CloseBrace
            | TokenKind::CloseBracket
            | TokenKind::CloseParen => depth = depth.checked_sub(1)?,
            TokenKind::Comma if depth == 0 => {
                append_field(&tokens[start..index], &mut fields, &mut names)?;
                start = index + 1;
            }
            _ => {}
        }
    }
    if depth != 0 {
        return None;
    }
    if start < tokens.len() {
        append_field(&tokens[start..], &mut fields, &mut names)?;
    }
    Some(fields)
}

/// Validates each field identically with or without a terminating comma.
fn append_field(
    field: &[Token],
    fields: &mut Vec<(String, TypeExpr)>,
    names: &mut BTreeSet<String>,
) -> Option<()> {
    if field.len() < 2 {
        return None;
    }
    if matches!(&field[0].kind, TokenKind::Identifier(word) if word == graph_names::PUBLIC) {
        return None;
    }
    let before_default = field
        .iter()
        .position(|token| matches!(token.kind, TokenKind::Equals))
        .unwrap_or(field.len());
    let name_index = field[..before_default]
        .iter()
        .rposition(|token| matches!(token.kind, TokenKind::Identifier(_)))?;
    if name_index == 0 {
        return None;
    }
    if name_index + 1 != before_default {
        return None;
    }
    let TokenKind::Identifier(name) = &field[name_index].kind else {
        return None;
    };
    if !names.insert(name.clone()) {
        return None;
    }
    let ty = parse_complete_type(&field[..name_index])?;
    if before_default < field.len() && !validate_value(&field[before_default + 1..]) {
        return None;
    }
    fields.push((name.clone(), ty));
    Some(())
}

/// Parses exactly one type expression from token syntax.
fn parse_complete_type(tokens: &[Token]) -> Option<TypeExpr> {
    parse_complete_type_fallible(tokens).ok().flatten()
}

/// Separates malformed grammar from allocation failure when copying a complete successor type.
pub(super) fn parse_complete_type_fallible(
    tokens: &[Token],
) -> Result<Option<TypeExpr>, neutral_core::allocation::AllocationError> {
    let mut index = 0;
    let ty = parse_type_fallible(tokens, &mut index, 0)?;
    Ok(ty.filter(|_| index == tokens.len()))
}

/// Bounds recursive grammar before fallibly retaining spelling or wrapper storage.
fn parse_type_fallible(
    tokens: &[Token],
    index: &mut usize,
    depth: usize,
) -> Result<Option<TypeExpr>, neutral_core::allocation::AllocationError> {
    use neutral_core::allocation::{boxed, text};
    if depth > MAX_PROJECT_INTERFACE_TYPE_DEPTH {
        return Ok(None);
    }
    let Some(first) = tokens.get(*index) else {
        return Ok(None);
    };
    *index += 1;
    let mut ty = match &first.kind {
        TokenKind::Num => TypeExpr::Num,
        TokenKind::StringType => TypeExpr::String,
        TokenKind::BoolType => TypeExpr::Bool,
        TokenKind::Identifier(name) if name == graph_names::URL => TypeExpr::Url,
        TokenKind::Identifier(name) if name == graph_names::PATH => TypeExpr::Path,
        TokenKind::Identifier(name) | TokenKind::ProtectedName(name) => {
            if matches!(
                tokens.get(*index).map(|token| &token.kind),
                Some(TokenKind::DoubleColon)
            ) {
                *index += 1;
                let Some(second) = tokens.get(*index) else {
                    return Ok(None);
                };
                let TokenKind::Identifier(target) = &second.kind else {
                    return Ok(None);
                };
                *index += 1;
                TypeExpr::Nominal(Some(text(name)?), text(target)?, second.span)
            } else {
                TypeExpr::Nominal(None, text(name)?, first.span)
            }
        }
        TokenKind::List | TokenKind::RefType => {
            if !matches!(
                tokens.get(*index).map(|token| &token.kind),
                Some(TokenKind::Less)
            ) {
                return Ok(None);
            }
            *index += 1;
            let Some(inner) = parse_type_fallible(tokens, index, depth + 1)? else {
                return Ok(None);
            };
            if !matches!(
                tokens.get(*index).map(|token| &token.kind),
                Some(TokenKind::Greater)
            ) {
                return Ok(None);
            }
            *index += 1;
            let inner = boxed(inner)?;
            if matches!(first.kind, TokenKind::List) {
                TypeExpr::List(inner)
            } else {
                TypeExpr::Ref(inner)
            }
        }
        _ => return Ok(None),
    };
    if matches!(
        tokens.get(*index).map(|token| &token.kind),
        Some(TokenKind::Question)
    ) {
        *index += 1;
        ty = TypeExpr::Nullable(boxed(ty)?);
    }
    Ok(Some(ty))
}

/// Checks complete inherited contextual-value syntax, including qualified names.
fn validate_value(tokens: &[Token]) -> bool {
    let mut index = 0;
    parse_value_shape(tokens, &mut index, 0).is_some() && index == tokens.len()
}

/// Consumes one bounded scalar, reuse, reference, record, or list value.
fn parse_value_shape(tokens: &[Token], index: &mut usize, depth: usize) -> Option<()> {
    if depth > MAX_PROJECT_INTERFACE_TYPE_DEPTH {
        return None;
    }
    let token = tokens.get(*index)?;
    match token.kind {
        TokenKind::Number(_)
        | TokenKind::StringLiteral(_)
        | TokenKind::True
        | TokenKind::False
        | TokenKind::Null => {
            *index += 1;
        }
        TokenKind::Identifier(_) => {
            let (_, end) = parse_name(tokens, *index)?;
            *index = end;
        }
        TokenKind::RefValue => {
            let (_, end) = parse_reference(tokens, *index)?;
            *index = end;
        }
        TokenKind::OpenBracket => {
            *index += 1;
            while !matches!(
                tokens.get(*index).map(|token| &token.kind),
                Some(TokenKind::CloseBracket)
            ) {
                parse_value_shape(tokens, index, depth + 1)?;
                if matches!(
                    tokens.get(*index).map(|token| &token.kind),
                    Some(TokenKind::Comma)
                ) {
                    *index += 1;
                } else if !matches!(
                    tokens.get(*index).map(|token| &token.kind),
                    Some(TokenKind::CloseBracket)
                ) {
                    return None;
                }
            }
            *index += 1;
        }
        TokenKind::OpenBrace => {
            *index += 1;
            while !matches!(
                tokens.get(*index).map(|token| &token.kind),
                Some(TokenKind::CloseBrace)
            ) {
                if !matches!(
                    tokens.get(*index).map(|token| &token.kind),
                    Some(TokenKind::Identifier(_))
                ) {
                    return None;
                }
                *index += 1;
                if !matches!(
                    tokens.get(*index).map(|token| &token.kind),
                    Some(TokenKind::Colon)
                ) {
                    return None;
                }
                *index += 1;
                parse_value_shape(tokens, index, depth + 1)?;
                if !matches!(
                    tokens.get(*index).map(|token| &token.kind),
                    Some(TokenKind::Comma)
                ) {
                    return None;
                }
                *index += 1;
            }
            *index += 1;
        }
        _ => return None,
    }
    Some(())
}
