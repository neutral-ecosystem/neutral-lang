// SPDX-License-Identifier: Apache-2.0

//! Boundary tests for the private graph grammar and source phase machine.

use super::{GraphSyntaxErrorKind, lexer, parse_module_name, scan_graph_source, valid_graph_name};

/// Qualified names require a final name and alternate names with exactly one separator.
#[test]
fn graph_module_names_reject_incomplete_token_sequences() {
    for source in ["", "a::", "a::b::", "a b", "::a", "a::::b"] {
        let lexed = lexer::lex(source.as_bytes()).unwrap();
        let tokens = &lexed.tokens[..lexed.tokens.len() - 1];
        assert_eq!(parse_module_name(tokens).ok(), Some(None), "{source}");
    }
}

/// Rejects each malformed header independently of its otherwise valid module spelling.
#[test]
fn graph_headers_require_both_module_keyword_and_expected_name() {
    for header in ["public example", "module other", "module", "module a::"] {
        let source = format!("neu \"1.0\"\n{header}\n");
        let error = scan_graph_source(source.as_bytes(), "example", 4)
            .err()
            .expect("malformed header must fail");
        assert_eq!(error.kind, GraphSyntaxErrorKind::InvalidSyntax, "{header}");
    }
}

/// Retains vocabulary and deeply qualified imports in their permitted source order.
#[test]
fn graph_vocabulary_precedes_qualified_imports_and_declarations() {
    let source = b"neu \"1.0\"\nmodule example\nuse Vocabulary as dict\nimport a::b::c::d as dep\nstring message = \"ok\"\n";
    let syntax = scan_graph_source(source, "example", 1).ok().unwrap();
    assert_eq!(syntax.vocabulary_aliases, ["dict"]);
    assert_eq!(syntax.imports.len(), 1);
    assert_eq!(syntax.imports[0].target, "a::b::c::d");
    assert_eq!(syntax.imports[0].alias, "dep");
    for body in [
        "import other as dep\nuse Vocabulary as dict",
        "string message = \"ok\"\nuse Vocabulary as dict",
    ] {
        let source = format!("neu \"1.0\"\nmodule example\n{body}\n");
        let error = scan_graph_source(source.as_bytes(), "example", 4)
            .err()
            .expect("late vocabulary requirement must fail");
        assert_eq!(error.kind, GraphSyntaxErrorKind::InvalidSyntax, "{body}");
    }
}

/// Validates vocabulary identity, separator, alias grammar, and collisions independently.
#[test]
fn graph_vocabulary_rejects_each_invalid_field_and_alias_collision() {
    for (body, kind) in [
        (
            "use vocabulary as dict",
            GraphSyntaxErrorKind::InvalidSyntax,
        ),
        (
            "use Vocabulary alias dict",
            GraphSyntaxErrorKind::InvalidSyntax,
        ),
        (
            "use Vocabulary as bad__alias",
            GraphSyntaxErrorKind::InvalidSyntax,
        ),
        (
            "use Vocabulary as example",
            GraphSyntaxErrorKind::AliasCollision,
        ),
        (
            "use Vocabulary as dict\nuse Other as dict",
            GraphSyntaxErrorKind::AliasCollision,
        ),
    ] {
        let source = format!("neu \"1.0\"\nmodule example\n{body}\n");
        let error = scan_graph_source(source.as_bytes(), "example", 4)
            .err()
            .expect("invalid vocabulary requirement must fail");
        assert_eq!(error.kind, kind, "{body}");
    }
}

/// Rejects incomplete qualification and invalid alias fields without indexing beyond the line.
#[test]
fn graph_imports_reject_incomplete_paths_and_each_invalid_alias_field() {
    for statement in [
        "import a::b::",
        "import a::b::c::",
        "import a:: as dep",
        "import other alias dep",
        "import other as bad__alias",
    ] {
        let source = format!("neu \"1.0\"\nmodule example\n{statement}\n");
        let error = scan_graph_source(source.as_bytes(), "example", 4)
            .err()
            .expect("malformed import must fail");
        assert_eq!(
            error.kind,
            GraphSyntaxErrorKind::InvalidSyntax,
            "{statement}"
        );
    }
}

/// Keeps ASCII snake names distinct from keywords, graph words, and malformed segments.
#[test]
fn graph_names_require_valid_segments_and_exclude_reserved_words() {
    for name in ["a", "module1", "a1_b2", "several_words"] {
        assert!(valid_graph_name(name), "{name}");
    }
    for name in [
        "", "A", "1a", "_a", "a_", "a__b", "a_2", "a-b", "é", "aB", "true", "record", "import",
        "as", "public",
    ] {
        assert!(!valid_graph_name(name), "{name}");
    }
}
