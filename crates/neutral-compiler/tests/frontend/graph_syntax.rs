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
        "string message = \"ok\"\nimport other as dep",
        "module example",
        "neu \"1.0\"",
    ] {
        let source = format!("neu \"1.0\"\nmodule example\n{body}\n");
        let error = scan_graph_source(source.as_bytes(), "example", 4)
            .err()
            .expect("late vocabulary requirement must fail");
        assert_eq!(error.kind, GraphSyntaxErrorKind::InvalidSyntax, "{body}");
    }
}

/// Neither an empty source nor a lone language header completes the required prefix.
#[test]
fn graph_headers_must_complete_before_publication() {
    for source in ["", "neu \"1.0\"\n"] {
        assert!(scan_graph_source(source.as_bytes(), "example", 1).is_err());
    }
    assert!(scan_graph_source(b"neu \"1.0\"\nmodule example\n", "example", 1).is_ok());
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

/// Graph validation refines captured snake names rather than widening its frozen source grammar.
#[test]
fn graph_module_segments_require_letters_after_underscores() {
    for name in ["a_2", "a::child_2"] {
        let source = format!("neu \"1.0\"\nmodule {name}\n");
        let error = scan_graph_source(source.as_bytes(), name, 1)
            .err()
            .expect("numeric underscore segments violate MODULE-GRAPH");
        assert_eq!(error.kind, GraphSyntaxErrorKind::InvalidSyntax);
    }
    let name = "a1_b2::child_c3";
    let source = format!("neu \"1.0\"\nmodule {name}\n");
    assert!(scan_graph_source(source.as_bytes(), name, 1).is_ok());
}

/// Lexer failures distinguish import acquisition/re-export lines from ordinary malformed expressions.
#[test]
fn unsupported_import_symbols_keep_their_forbidden_classification() {
    for (line, expected) in [
        (
            "  import foo.bar as dep",
            GraphSyntaxErrorKind::ForbiddenImport,
        ),
        (
            "\tpublic import foo/bar as dep",
            GraphSyntaxErrorKind::ForbiddenImport,
        ),
        (
            "public num value = 1 % 2",
            GraphSyntaxErrorKind::InvalidSyntax,
        ),
    ] {
        for newline in ["\n", "\r\n", "\r"] {
            let source = format!("neu \"1.0\"{newline}module example{newline}{line}{newline}");
            let error = scan_graph_source(source.as_bytes(), "example", 1)
                .err()
                .unwrap();
            assert_eq!(error.kind, expected, "{line:?} {newline:?}");
        }
    }
}

/// The shortest complete import has exactly four tokens; missing aliases still fail safely.
#[test]
fn minimum_import_tuple_accepts_exactly_target_as_alias() {
    let source = b"neu \"1.0\"\nmodule example\nimport other as dep\n";
    let syntax = scan_graph_source(source, "example", 1).ok().unwrap();
    assert_eq!(syntax.imports[0].target, "other");
    assert_eq!(syntax.imports[0].alias, "dep");
    for statement in ["import", "import other", "import other as"] {
        let source = format!("neu \"1.0\"\nmodule example\n{statement}\n");
        assert!(scan_graph_source(source.as_bytes(), "example", 1).is_err());
    }
}
