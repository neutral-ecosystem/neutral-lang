// SPDX-License-Identifier: Apache-2.0

//! Whitespace-only layout for mutable .neu examples, preserving literal and comment bytes.

use crate::{collect_regular_files, configuration, fs, output, workspace_root};
use std::{collections::BTreeSet, path::Path};

#[cfg(test)]
#[path = "../../tests/unit/source_formatting.rs"]
mod tests;

/// Checks or reformats configured source files after every input has passed layout scanning.
pub(crate) fn format_sources(write: bool) -> Result<(), String> {
    let root = workspace_root()?;
    let config = configuration::automation()?.formatting;
    let mut files = Vec::new();
    for directory in &config.roots {
        let directory = root.join(directory);
        if fs::symlink_metadata(&directory)
            .map_err(|error| error.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err(format!(
                "formatting root must not be a symlink: {}",
                directory.display()
            ));
        }
        collect_regular_files(&directory, &mut files)?;
    }
    let files = files.into_iter().collect::<BTreeSet<_>>();
    let mut updates = Vec::new();
    let mut count = 0;
    for file in files {
        let relative = file
            .strip_prefix(&root)
            .map_err(|error| error.to_string())?;
        if file.extension().is_none_or(|extension| extension != "neu")
            || config
                .exclude
                .iter()
                .any(|excluded| relative.starts_with(Path::new(excluded)))
        {
            continue;
        }
        let source = fs::read_to_string(&file)
            .map_err(|error| format!("{}: {error}", relative.display()))?;
        let formatted = reflow(&source, config.indent_width)
            .map_err(|error| format!("{}: {error}", relative.display()))?;
        count += 1;
        if formatted != source {
            updates.push((file, formatted));
        }
    }
    for (file, formatted) in &updates {
        output::file("source layout", file.strip_prefix(&root).unwrap_or(file));
        if write {
            fs::write(file, formatted)
                .map_err(|error| format!("could not format {}: {error}", file.display()))?;
        }
    }
    if !write && !updates.is_empty() {
        return Err("Neutral source layout differs; run `cargo xtask fmt --write`".to_owned());
    }
    output::pass(format!(
        "Neutral source layout: {count} files, {} changed",
        updates.len()
    ));
    Ok(())
}

/// Expands braces and members without changing token spellings or validating language semantics.
fn reflow(source: &str, indent_width: usize) -> Result<String, String> {
    let mut chars = source.char_indices().peekable();
    let mut output = String::new();
    let mut line = String::new();
    let mut line_depth = 0;
    let mut stack = Vec::new();
    let mut previous_newline = false;
    while let Some((start, ch)) = chars.next() {
        if line.is_empty() {
            line_depth = stack.len();
        }
        if let Some(end) = retained_token_end(start, ch, &mut chars)? {
            line.push_str(&source[start..end]);
            if source[start..end].starts_with("//") {
                flush_exact_line(&mut output, &mut line, line_depth, indent_width);
            }
            previous_newline = false;
            continue;
        }
        match ch {
            '{' => {
                if !line.is_empty() && !line.ends_with([' ', '[', '(']) {
                    line.push(' ');
                }
                line.push(ch);
                flush_line(&mut output, &mut line, line_depth, indent_width);
                stack.push(ch);
            }
            '}' => {
                flush_line(&mut output, &mut line, line_depth, indent_width);
                if stack.pop() != Some('{') {
                    return Err("unbalanced braces".to_owned());
                }
                line_depth = stack.len();
                line.push(ch);
            }
            '[' | '(' => {
                line.push(ch);
                stack.push(ch);
            }
            ']' | ')' => {
                let expected = if ch == ']' { '[' } else { '(' };
                if stack.pop() != Some(expected) {
                    return Err("unbalanced delimiters".to_owned());
                }
                if line.is_empty() {
                    line_depth = stack.len();
                }
                line.push(ch);
            }
            ',' => {
                line.truncate(line.trim_end().len());
                line.push(ch);
                if stack.last() == Some(&'{') {
                    flush_line(&mut output, &mut line, line_depth, indent_width);
                } else {
                    line.push(' ');
                }
            }
            // Keep whitespace-only else continuations in the preceding statement.
            '\r' | '\n' if line.trim() == "}" && starts_else(&source[start..]) => continue,
            '\r' | '\n' => {
                if ch == '\r' && chars.peek().is_some_and(|(_, next)| *next == '\n') {
                    chars.next();
                }
                if line.is_empty() && previous_newline && !output.ends_with("\n\n") {
                    output.push('\n');
                }
                flush_line(&mut output, &mut line, line_depth, indent_width);
                previous_newline = true;
                continue;
            }
            ' ' | '\t' => {
                if !line.is_empty() && !line.ends_with(' ') {
                    line.push(' ');
                }
                continue;
            }
            _ => {
                if line.trim() == "}" {
                    if starts_else(&source[start..]) {
                        line.truncate(line.trim_end().len());
                        line.push(' ');
                    } else {
                        flush_line(&mut output, &mut line, line_depth, indent_width);
                        line_depth = stack.len();
                    }
                }
                line.push(ch);
            }
        }
        previous_newline = false;
    }
    if !stack.is_empty() {
        return Err("unclosed delimiter".to_owned());
    }
    flush_line(&mut output, &mut line, line_depth, indent_width);
    while output.ends_with("\n\n") {
        output.pop();
    }
    Ok(output)
}

/// Recognizes the complete continuation keyword without merging declarations such as `elsewhere`.
fn starts_else(tail: &str) -> bool {
    tail.trim_start()
        .strip_prefix("else")
        .is_some_and(|tail| tail.starts_with(|ch: char| ch.is_whitespace() || ch == '{'))
}

/// Consumes one literal or comment as an exact byte span, shielding its punctuation from layout.
fn retained_token_end(
    start: usize,
    ch: char,
    chars: &mut std::iter::Peekable<std::str::CharIndices<'_>>,
) -> Result<Option<usize>, String> {
    if ch == '"' {
        while let Some((index, next)) = chars.next() {
            if next == '\\' {
                chars.next().ok_or("unterminated string escape")?;
            } else if next == '"' {
                return Ok(Some(index + 1));
            }
        }
        return Err("unterminated string".to_owned());
    }
    if ch != '/' {
        return Ok(None);
    }
    match chars.peek() {
        Some((_, '/')) => {
            let mut end = start + 1;
            while let Some(&(index, next)) = chars.peek() {
                if matches!(next, '\r' | '\n') {
                    break;
                }
                end = index + next.len_utf8();
                chars.next();
            }
            Ok(Some(end))
        }
        Some((_, '*')) => {
            chars.next();
            let mut depth = 1;
            while let Some((index, next)) = chars.next() {
                if next == '/' && chars.peek().is_some_and(|(_, ch)| *ch == '*') {
                    chars.next();
                    depth += 1;
                } else if next == '*' && chars.peek().is_some_and(|(_, ch)| *ch == '/') {
                    chars.next();
                    depth -= 1;
                    if depth == 0 {
                        return Ok(Some(index + 2));
                    }
                }
            }
            Err("unterminated block comment".to_owned())
        }
        _ => Ok(None),
    }
}

/// Appends a nonempty physical line with nesting indentation and an LF terminator.
fn flush_line(output: &mut String, line: &mut String, depth: usize, indent_width: usize) {
    line.truncate(line.trim_end().len());
    flush_exact_line(output, line, depth, indent_width);
}

/// Writes a line without trimming protected comment content.
fn flush_exact_line(output: &mut String, line: &mut String, depth: usize, indent_width: usize) {
    if !line.is_empty() {
        output.push_str(&" ".repeat(depth * indent_width));
        output.push_str(line);
        output.push('\n');
        line.clear();
    }
}
