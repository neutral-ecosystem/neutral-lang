// SPDX-License-Identifier: Apache-2.0

//! Shared field schema for human-readable and JSON artifact inspection.
//!
//! This consumer-owned projection is not the canonical NIR-CBOR wire schema.

use super::ProbeSummary;
use std::fmt::Write as _;

/// Version of the probe's JSON inspection projection.
pub const SCHEMA_VERSION: u32 = 1;

/// One value shape exposed by the inspection schema.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldValue<'a> {
    /// One required string.
    Text(&'a str),
    /// One optional string, represented as JSON `null` when absent.
    OptionalText(Option<&'a str>),
    /// An ordered list of strings.
    TextList(&'a [String]),
}

/// One named inspection field shared by the text and JSON renderers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Field<'a> {
    /// Stable JSON member name.
    pub json_key: &'static str,
    /// Prefix used by the human-readable probe output.
    pub text_prefix: &'static str,
    /// Value obtained from one validated probe summary.
    pub value: FieldValue<'a>,
}

/// Returns the complete ordered inspection schema and values for one summary.
#[must_use]
pub fn fields(summary: &ProbeSummary) -> [Field<'_>; 12] {
    [
        Field {
            json_key: "module",
            text_prefix: "module",
            value: FieldValue::Text(summary.module()),
        },
        Field {
            json_key: "metadata",
            text_prefix: "metadata",
            value: FieldValue::TextList(summary.metadata()),
        },
        Field {
            json_key: "vocabulary",
            text_prefix: "vocabulary",
            value: FieldValue::OptionalText(summary.vocabulary()),
        },
        Field {
            json_key: "record_types",
            text_prefix: "record",
            value: FieldValue::TextList(summary.record_types()),
        },
        Field {
            json_key: "vocabulary_types",
            text_prefix: "vocabulary-type",
            value: FieldValue::TextList(summary.vocabulary_types()),
        },
        Field {
            json_key: "declarations",
            text_prefix: "declaration",
            value: FieldValue::TextList(summary.declarations()),
        },
        Field {
            json_key: "source_mappings",
            text_prefix: "source-map",
            value: FieldValue::TextList(summary.source_mappings()),
        },
        Field {
            json_key: "value_provenance",
            text_prefix: "value-provenance",
            value: FieldValue::TextList(summary.value_provenance()),
        },
        Field {
            json_key: "field_provenance",
            text_prefix: "field-provenance",
            value: FieldValue::TextList(summary.field_provenance()),
        },
        Field {
            json_key: "reuse_provenance",
            text_prefix: "reuse-provenance",
            value: FieldValue::TextList(summary.reuse_provenance()),
        },
        Field {
            json_key: "reference_provenance",
            text_prefix: "reference-provenance",
            value: FieldValue::TextList(summary.reference_provenance()),
        },
        Field {
            json_key: "diagnostics",
            text_prefix: "diagnostic",
            value: FieldValue::TextList(summary.diagnostics()),
        },
    ]
}

/// Renders the shared inspection fields as indented JSON.
#[must_use]
pub fn render_summary_json(summary: &ProbeSummary) -> String {
    render_fields_json(&fields(summary))
}

/// Renders a validated inspection field projection using the shared JSON rules.
pub(crate) fn render_fields_json(fields: &[Field<'_>]) -> String {
    let mut output = format!("{{\n  \"schema_version\": {SCHEMA_VERSION},\n");
    for (index, field) in fields.iter().enumerate() {
        write!(output, "  \"{}\": ", field.json_key).expect("writing to a String cannot fail");
        match field.value {
            FieldValue::Text(value) | FieldValue::OptionalText(Some(value)) => {
                write_json_string(&mut output, value);
            }
            FieldValue::OptionalText(None) => output.push_str("null"),
            FieldValue::TextList(values) => write_json_list(&mut output, values),
        }
        if index + 1 != fields.len() {
            output.push(',');
        }
        output.push('\n');
    }
    output.push_str("}\n");
    output
}

/// Writes one ordered string list with two-space JSON indentation.
fn write_json_list(output: &mut String, values: &[String]) {
    output.push('[');
    if !values.is_empty() {
        output.push('\n');
        for (index, value) in values.iter().enumerate() {
            output.push_str("    ");
            write_json_string(output, value);
            if index + 1 != values.len() {
                output.push(',');
            }
            output.push('\n');
        }
        output.push_str("  ");
    }
    output.push(']');
}

/// Writes one JSON string with all control characters escaped.
fn write_json_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            control if control <= '\u{001f}' => {
                write!(output, "\\u{:04x}", u32::from(control))
                    .expect("writing to a String cannot fail");
            }
            other => output.push(other),
        }
    }
    output.push('"');
}
