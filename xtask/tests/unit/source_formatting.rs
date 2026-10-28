// SPDX-License-Identifier: Apache-2.0

//! Neutral source layout regressions for nested blocks, literal preservation, and safe failure.

use super::reflow;

/// A top-level conditional's else belongs to the same statement as its preceding closing brace.
#[test]
fn source_layout_keeps_else_on_the_conditionals_closing_line() {
    for source in [
        "public num value = if enabled { 1 } else { 2 }\n",
        "public num value = if enabled {\n1\n}\nelse {\n2\n}\n",
        "public num value = if enabled { 1 }else{ 2 }\n",
    ] {
        let formatted = reflow(source, 4).unwrap();
        assert!(formatted.contains("} else {"), "{formatted}");
        assert_eq!(reflow(&formatted, 4).unwrap(), formatted);
    }
    let separate = reflow("record A {} elsewhere = 2\n", 4).unwrap();
    assert!(separate.contains("}\nelsewhere"));
}

#[test]
/// Expands source declarations and nested initializers while retaining final-member spelling.
fn source_layout_expands_braces_and_is_idempotent() {
    let source = "neu \"1.0\"\nmodule api\n\npublic record Inner { num count = 42, num r = 3}\npublic Inner value = { count: 8, }\n";
    let expected = "neu \"1.0\"\nmodule api\n\npublic record Inner {\n    num count = 42,\n    num r = 3\n}\npublic Inner value = {\n    count: 8,\n}\n";
    let formatted = reflow(source, 4).unwrap();
    assert_eq!(formatted, expected);
    assert_eq!(reflow(&formatted, 4).unwrap(), formatted);
}

#[test]
/// Leaves punctuation, escapes, Unicode, and newlines inside literal/comment tokens unchanged.
fn source_layout_preserves_literals_and_comments() {
    let source = "// { ignored, }\nstring message = \"é {,} \\\"//\"\n/* { ignored\n   nested /* } */ } */\nrecord R { string s = \"{,}\", }\n";
    let formatted = reflow(source, 4).unwrap();
    assert!(formatted.contains("// { ignored, }"));
    assert!(formatted.contains("\"é {,} \\\"//\""));
    assert!(formatted.contains("/* { ignored\n   nested /* } */ } */"));
    assert_eq!(reflow(&formatted, 4).unwrap(), formatted);
}

#[test]
/// Keeps multiline collection bindings at the root and indents their nested record values.
fn source_layout_handles_records_in_lists_and_separate_declarations() {
    let source = "List<R> items = [\n{x: 1, y: [2, 3]},\n{x: 4}\n]\nrecord R { num x, } record S { num y, }\n";
    let formatted = reflow(source, 4).unwrap();
    assert!(formatted.starts_with("List<R> items = [\n    {\n        x: 1,\n"));
    assert!(formatted.contains("y: [2, 3]\n    },\n"));
    assert!(formatted.contains("\n]\nrecord R {\n    num x,\n}\nrecord S {"));
    assert_eq!(reflow(&formatted, 4).unwrap(), formatted);
}

#[test]
/// Rejects malformed lexical boundaries before any formatted source is returned.
fn source_layout_rejects_unfinished_or_mismatched_input() {
    for source in [
        "record X {",
        "}",
        "[)",
        "\"unterminated",
        "\"escape\\",
        "/* open",
    ] {
        assert!(reflow(source, 4).is_err(), "{source}");
    }
}
