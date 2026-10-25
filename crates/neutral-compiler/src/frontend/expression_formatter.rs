// SPDX-License-Identifier: Apache-2.0

//! Bounded source formatting for the current expression project grammar.

use crate::{CancellationToken, CapturedExpressionProject};
use neutral_core::allocation::RetainCapacity;

use super::{ExpressionCompileFailure, compile_expression_project};
use crate::frontend::expression_codes as codes;

/// Validates the complete captured project, then formats exactly one requested module.
///
/// # Errors
/// Returns the compiler's safe failure when validation, cancellation, module selection,
/// allocation, or the captured output ceiling prevents a complete formatted result.
///
/// Whitespace outside comments and quoted strings is normalized without rewriting tokens.
pub fn format_expression_source(
    captured: &CapturedExpressionProject,
    module: &str,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, ExpressionCompileFailure> {
    compile_expression_project(captured, cancel)?;
    let source = captured
        .sources()
        .iter()
        .find(|source| source.module_id() == module)
        .ok_or(ExpressionCompileFailure {
            code: codes::EXP_SYNTAX,
            location: None,
        })?;
    let formatted = normalize(
        source.bytes(),
        captured.limits().values().output_bytes,
        cancel,
    )?;
    let replay = captured
        .replay_with_source(module, &formatted, cancel.clone())
        .map_err(|error| failure(error.code()))?;
    let checked =
        crate::capture_expression_project(replay).map_err(|error| failure(error.code()))?;
    compile_expression_project(&checked, cancel)?;
    Ok(formatted)
}

/// Renders braces and list delimiters while preserving comments and quoted spellings.
#[allow(clippy::too_many_lines)] // Lexical boundaries and output publication share one bounded scan.
fn normalize(
    source: &[u8],
    limit: u64,
    cancel: &CancellationToken,
) -> Result<Vec<u8>, ExpressionCompileFailure> {
    let mut output = Vec::new();
    let bytes = source;
    let mut index = 0;
    let mut pending_space = false;
    let mut pending_newline = false;
    let mut layout_depth = 0usize;
    while index < bytes.len() {
        match bytes[index] {
            b' ' | b'\t' => {
                pending_space = true;
                index += 1;
            }
            b'\r' | b'\n' => {
                pending_newline = true;
                if bytes[index] == b'\r' && bytes.get(index + 1) == Some(&b'\n') {
                    index += 1;
                }
                while output.last() == Some(&b' ') {
                    output.pop();
                }
                index += 1;
            }
            _ => {
                let start = index;
                if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
                    while index < bytes.len() && !matches!(bytes[index], b'\r' | b'\n') {
                        check_cancel(cancel)?;
                        index += 1;
                    }
                } else if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') {
                    index += 2;
                    while index + 1 < bytes.len() && &bytes[index..index + 2] != b"*/" {
                        check_cancel(cancel)?;
                        index += 1;
                    }
                    index = (index + 2).min(bytes.len());
                } else if bytes[index] == b'$' && bytes.get(index + 1) == Some(&b'"') {
                    index = string_end(bytes, index + 1, true, cancel)?;
                } else if bytes[index] == b'"' {
                    index = string_end(bytes, index, false, cancel)?;
                } else if matches!(bytes[index], b'{' | b'[' | b'}' | b']' | b',') {
                    index += 1;
                } else {
                    while !token_boundary(bytes, index) {
                        check_cancel(cancel)?;
                        index += 1;
                    }
                }
                let token = &bytes[start..index];
                if token == b"{" || token == b"[" {
                    emit_pending(
                        &mut output,
                        pending_space,
                        pending_newline,
                        layout_depth,
                        limit,
                        cancel,
                    )?;
                    if !output.is_empty()
                        && !output.ends_with(b"\n")
                        && output.last() != Some(&b' ')
                    {
                        append(&mut output, b" ", limit, cancel)?;
                    }
                    append(&mut output, token, limit, cancel)?;
                    layout_depth += 1;
                    append(&mut output, b"\n", limit, cancel)?;
                } else if token == b"}" || token == b"]" {
                    layout_depth = layout_depth.saturating_sub(1);
                    if !output.is_empty() && !output.ends_with(b"\n") {
                        append(&mut output, b"\n", limit, cancel)?;
                    }
                    append_indent(&mut output, layout_depth, limit, cancel)?;
                    append(&mut output, token, limit, cancel)?;
                } else if token == b"," && layout_depth > 0 {
                    append(&mut output, b",\n", limit, cancel)?;
                } else {
                    emit_pending(
                        &mut output,
                        pending_space,
                        pending_newline,
                        layout_depth,
                        limit,
                        cancel,
                    )?;
                    if output.last() == Some(&b'}') && token == b"else" {
                        append(&mut output, b" ", limit, cancel)?;
                    }
                    append(&mut output, token, limit, cancel)?;
                }
                pending_space = false;
                pending_newline = false;
            }
        }
    }
    while output.last() == Some(&b' ') || output.last() == Some(&b'\n') {
        output.pop();
    }
    append(&mut output, b"\n", limit, cancel)?;
    Ok(output)
}

/// Emits retained whitespace between lexical tokens, with the active indentation.
fn emit_pending(
    output: &mut Vec<u8>,
    space: bool,
    newline: bool,
    depth: usize,
    limit: u64,
    cancel: &CancellationToken,
) -> Result<(), ExpressionCompileFailure> {
    if output.ends_with(b"\n") {
        append_indent(output, depth, limit, cancel)?;
        return Ok(());
    }
    if !output.is_empty() && !output.ends_with(b"\n") {
        if newline {
            append(output, b"\n", limit, cancel)?;
            append_indent(output, depth, limit, cancel)?;
        } else if space && output.last() != Some(&b' ') {
            append(output, b" ", limit, cancel)?;
        }
    }
    Ok(())
}

/// Identifies lexical boundaries the formatter must keep as separate chunks.
fn token_boundary(bytes: &[u8], index: usize) -> bool {
    index >= bytes.len()
        || bytes[index].is_ascii_whitespace()
        || matches!(bytes[index], b'{' | b'}' | b'[' | b']' | b',' | b'"')
        || (bytes[index] == b'/'
            && bytes
                .get(index + 1)
                .is_some_and(|b| matches!(b, b'/' | b'*')))
        || (bytes[index] == b'$' && bytes.get(index + 1) == Some(&b'"'))
}

/// Appends one four-space indentation level per open brace or list.
fn append_indent(
    output: &mut Vec<u8>,
    depth: usize,
    limit: u64,
    cancel: &CancellationToken,
) -> Result<(), ExpressionCompileFailure> {
    for _ in 0..depth {
        append(
            output,
            crate::frontend::format_style::INDENT.as_bytes(),
            limit,
            cancel,
        )?;
    }
    Ok(())
}

/// Grows output fallibly after checking cancellation and the captured byte ceiling.
fn append(
    output: &mut Vec<u8>,
    bytes: &[u8],
    limit: u64,
    cancel: &CancellationToken,
) -> Result<(), ExpressionCompileFailure> {
    if cancel.is_cancelled() {
        return Err(failure(codes::EXP_CANCELLED));
    }
    let new_len = output
        .len()
        .checked_add(bytes.len())
        .filter(|n| *n as u64 <= limit)
        .ok_or_else(|| failure(codes::EXP_LIMIT))?;
    output
        .try_retain(new_len - output.len())
        .map_err(|_| failure(codes::EXP_LIMIT))?;
    output.extend_from_slice(bytes);
    Ok(())
}

/// Finds the end of a quoted string, including nested interpolation expressions.
fn string_end(
    bytes: &[u8],
    quote: usize,
    interpolation: bool,
    cancel: &CancellationToken,
) -> Result<usize, ExpressionCompileFailure> {
    let mut cursor = quote + 1;
    while cursor < bytes.len() {
        check_cancel(cancel)?;
        match bytes[cursor] {
            b'\\' => cursor = (cursor + 2).min(bytes.len()),
            b'"' => return Ok(cursor + 1),
            b'$' if interpolation && bytes.get(cursor + 1) == Some(&b'{') => {
                cursor = interpolation_end(bytes, cursor + 2, cancel)?.saturating_add(1);
            }
            _ => cursor += 1,
        }
    }
    Err(failure(codes::EXP_SYNTAX))
}

/// Finds the matching closing brace of one nested interpolation expression.
fn interpolation_end(
    bytes: &[u8],
    mut cursor: usize,
    cancel: &CancellationToken,
) -> Result<usize, ExpressionCompileFailure> {
    let mut depth = 1usize;
    while cursor < bytes.len() {
        check_cancel(cancel)?;
        match bytes[cursor] {
            b'"' => cursor = string_end(bytes, cursor, false, cancel)?,
            b'$' if bytes.get(cursor + 1) == Some(&b'"') => {
                cursor = string_end(bytes, cursor + 1, true, cancel)?;
            }
            b'{' => {
                depth += 1;
                cursor += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Ok(cursor);
                }
                cursor += 1;
            }
            _ => cursor += 1,
        }
    }
    Err(failure(codes::EXP_SYNTAX))
}

/// Returns the request cancellation failure when the cooperative signal is set.
fn check_cancel(cancel: &CancellationToken) -> Result<(), ExpressionCompileFailure> {
    if cancel.is_cancelled() {
        Err(failure(codes::EXP_CANCELLED))
    } else {
        Ok(())
    }
}

/// Creates a location-free safe formatter failure.
fn failure(code: &'static str) -> ExpressionCompileFailure {
    ExpressionCompileFailure {
        code,
        location: None,
    }
}
