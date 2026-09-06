// SPDX-License-Identifier: Apache-2.0

//! Stable source rendering from the compiler-private parsed representation.

use super::{
    ParsedDeclaration, ParsedRecord, ParsedType, ParsedUnit, ParsedValue, Trivia, TriviaKind,
    format_style as style,
};
use crate::language::names;
use std::fmt::Write as _;

/// Formats one valid parsed source unit under the frozen v0 style.
pub(super) fn format_source(source: &[u8], unit: &ParsedUnit) -> Vec<u8> {
    let comments = unit
        .trivia
        .iter()
        .filter(|trivia| {
            matches!(
                trivia.kind,
                TriviaKind::LineComment | TriviaKind::BlockComment
            )
        })
        .collect::<Vec<_>>();
    let mut formatter = Formatter {
        source,
        comments,
        next_comment: 0,
        blocks: Vec::new(),
    };

    formatter.push_root(
        unit.language_header_span.end(),
        &format!(
            "{}{}{}",
            style::LANGUAGE_HEADER_PREFIX,
            names::SOURCE_LANGUAGE_VERSION,
            style::LANGUAGE_HEADER_SUFFIX
        ),
    );
    formatter.push_root(
        unit.module.span.end(),
        &format!("{}{}", style::MODULE_PREFIX, unit.module.name),
    );
    if let Some(vocabulary) = &unit.vocabulary_use {
        formatter.push_root(
            vocabulary.span.end(),
            &format!("{}{}", style::USE_PREFIX, vocabulary.name),
        );
    }
    for declaration in &unit.declarations {
        let rendered = render_declaration(declaration);
        formatter.push_root(declaration_span_end(declaration), &rendered);
    }
    formatter.finish()
}

/// Accumulates canonical root blocks while assigning retained comments.
struct Formatter<'a> {
    /// Original validated UTF-8 source used only to recover comment text.
    source: &'a [u8],
    /// Retained comments in original source order.
    comments: Vec<&'a Trivia>,
    /// Index of the next comment not yet assigned to output.
    next_comment: usize,
    /// Complete canonical top-level blocks.
    blocks: Vec<String>,
}

impl Formatter<'_> {
    /// Adds comments through one root span followed by that rendered construct.
    fn push_root(&mut self, span_end: u64, construct: &str) {
        let mut block = String::new();
        while self
            .comments
            .get(self.next_comment)
            .is_some_and(|comment| comment.span.start() <= span_end)
        {
            let comment = self.comments[self.next_comment];
            append_comment(&mut block, self.source, comment);
            self.next_comment += 1;
        }
        block.push_str(construct);
        self.blocks.push(block);
    }

    /// Emits trailing comments and assembles the final LF-terminated source.
    fn finish(mut self) -> Vec<u8> {
        if self.next_comment < self.comments.len() {
            let mut trailing = String::new();
            for comment in &self.comments[self.next_comment..] {
                append_comment(&mut trailing, self.source, comment);
            }
            while trailing.ends_with(style::NEWLINE) {
                trailing.pop();
            }
            if !trailing.is_empty() {
                self.blocks.push(trailing);
            }
        }
        let mut blocks = self.blocks.into_iter();
        let mut output = blocks.next().unwrap_or_default();
        for (index, block) in blocks.enumerate() {
            if index == 0 {
                output.push(style::HEADER_SEPARATOR);
            } else {
                output.push_str(style::ROOT_SEPARATOR);
            }
            output.push_str(&block);
        }
        output.push(style::NEWLINE);
        output.into_bytes()
    }
}

/// Appends one exact comment with normalized physical newlines.
fn append_comment(output: &mut String, source: &[u8], comment: &Trivia) {
    let start = usize::try_from(comment.span.start()).unwrap_or(usize::MAX);
    let end = usize::try_from(comment.span.end()).unwrap_or(usize::MAX);
    let text = source
        .get(start..end)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .expect("parsed trivia spans must select valid source UTF-8");
    output.push_str(&normalize_newlines(text));
    if !output.ends_with(style::NEWLINE) {
        output.push(style::NEWLINE);
    }
}

/// Converts every supported physical newline spelling to one line feed.
fn normalize_newlines(value: &str) -> String {
    value.replace("\r\n", "\n").replace('\r', "\n")
}

/// Returns the complete original end offset of one root declaration.
fn declaration_span_end(declaration: &ParsedDeclaration) -> u64 {
    match declaration {
        ParsedDeclaration::Record(record) => record.span.end(),
        ParsedDeclaration::Binding(binding) => binding.span.end(),
    }
}

/// Renders one root declaration using canonical spacing and layout.
fn render_declaration(declaration: &ParsedDeclaration) -> String {
    let mut output = String::new();
    match declaration {
        ParsedDeclaration::Record(record) => write_record_declaration(&mut output, record),
        ParsedDeclaration::Binding(binding) => {
            write_type(&mut output, &binding.declared_type);
            output.push(style::TYPE_NAME_SEPARATOR);
            output.push_str(&binding.name);
            output.push_str(style::INITIALIZER_SEPARATOR);
            write_value(&mut output, &binding.value, 0);
        }
    }
    output
}

/// Renders one nominal record declaration with one trailing-comma field per line.
fn write_record_declaration(output: &mut String, record: &ParsedRecord) {
    output.push_str(style::RECORD_PREFIX);
    output.push_str(&record.name);
    output.push_str(" {");
    output.push(style::NEWLINE);
    for field in &record.fields {
        write_indent(output, 1);
        write_type(output, &field.declared_type);
        output.push(style::TYPE_NAME_SEPARATOR);
        output.push_str(&field.name);
        if let Some(default) = &field.default_value {
            output.push_str(style::INITIALIZER_SEPARATOR);
            write_value(output, default, 1);
        }
        output.push(style::MEMBER_SUFFIX);
        output.push(style::NEWLINE);
    }
    output.push('}');
}

/// Renders one parsed type without exposing it as a public syntax model.
fn write_type(output: &mut String, parsed_type: &ParsedType) {
    match parsed_type {
        ParsedType::Num => output.push_str(names::NUM),
        ParsedType::String => output.push_str(names::STRING),
        ParsedType::Bool => output.push_str(names::BOOL),
        ParsedType::Record(name) => output.push_str(name),
        ParsedType::VocabularyRecord { namespace, name } => {
            output.push_str(namespace);
            output.push_str(style::QUALIFIER_SEPARATOR);
            output.push_str(name);
        }
        ParsedType::Nullable(inner) => {
            write_type(output, inner);
            output.push('?');
        }
        ParsedType::List(inner) => {
            output.push_str(names::LIST);
            output.push('<');
            write_type(output, inner);
            output.push('>');
        }
        ParsedType::Ref(inner) => {
            output.push_str(names::REF_TYPE);
            output.push('<');
            write_type(output, inner);
            output.push('>');
        }
    }
}

/// Renders one parsed value with canonical recursive indentation.
fn write_value(output: &mut String, value: &ParsedValue, depth: usize) {
    match value {
        ParsedValue::Number(number) | ParsedValue::Name(number) => output.push_str(number),
        ParsedValue::String(value) => write_string(output, value),
        ParsedValue::Boolean(value) => {
            output.push_str(if *value { names::TRUE } else { names::FALSE });
        }
        ParsedValue::Null => output.push_str(names::NULL),
        ParsedValue::Reference { target, .. } => {
            output.push_str(names::REF);
            output.push('(');
            output.push_str(target);
            output.push(')');
        }
        ParsedValue::Record(fields) => {
            output.push('{');
            output.push(style::NEWLINE);
            for field in fields {
                write_indent(output, depth + 1);
                output.push_str(&field.name);
                output.push_str(style::FIELD_VALUE_SEPARATOR);
                write_value(output, &field.value, depth + 1);
                output.push(style::MEMBER_SUFFIX);
                output.push(style::NEWLINE);
            }
            write_indent(output, depth);
            output.push('}');
        }
        ParsedValue::List(items) if items.is_empty() => output.push_str("[]"),
        ParsedValue::List(items) => {
            output.push('[');
            output.push(style::NEWLINE);
            for item in items {
                write_indent(output, depth + 1);
                write_value(output, &item.value, depth + 1);
                output.push(style::MEMBER_SUFFIX);
                output.push(style::NEWLINE);
            }
            write_indent(output, depth);
            output.push(']');
        }
    }
}

/// Renders one decoded string with the frozen canonical escape spellings.
fn write_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\0' => output.push_str("\\0"),
            character if character.is_control() => {
                write!(output, "\\u{{{:x}}}", u32::from(character))
                    .expect("writing to a string must be infallible");
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

/// Writes a checked number of canonical four-space indentation levels.
fn write_indent(output: &mut String, depth: usize) {
    for _ in 0..depth {
        output.push_str(style::INDENT);
    }
}
