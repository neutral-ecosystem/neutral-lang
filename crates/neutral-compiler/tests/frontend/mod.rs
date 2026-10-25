// SPDX-License-Identifier: Apache-2.0

//! Tests for the complete private active frontend pipeline.

use super::{FrontendErrorKind, lexer};

/// Graph scanning intersects successor decoded-string bounds before retaining declaration tokens.
#[test]
fn security_graph_scanner_checks_string_retention_before_resolution() {
    let source = "neu \"1.0\"\nmodule example\npublic string message = \"🙂a\"\n";
    for bound in [4, 5, 6] {
        let result = super::scan_graph_source_bounded(source.as_bytes(), "example", 4, bound, None);
        assert_eq!(result.is_ok(), bound >= 5);
        if let Err(error) = result {
            assert_eq!(error.kind, super::GraphSyntaxErrorKind::LimitExceeded);
        }
    }
}

/// Raw/escaped UTF-8 scalar retention checks the exact bound before adding an over-limit character.
#[test]
fn security_lexer_bounded_scalar_retention_and_cancellation() {
    for source in ["\"🙂a\"", "\"\\u{1F642}a\""] {
        for bound in [4, 5, 6] {
            let result = lexer::lex_bounded(source.as_bytes(), bound, None);
            assert_eq!(result.is_ok(), bound >= 5);
            if let Err(error) = result {
                assert_eq!(error.kind, FrontendErrorKind::RecordLimitExceeded);
            }
        }
    }
    let cancel = neutral_core::CancellationToken::new();
    cancel.cancel();
    for source in [
        b"/* comment */".as_slice(),
        b"\"value\"".as_slice(),
        b"".as_slice(),
    ] {
        assert!(lexer::lex_bounded(source, 64, Some(&cancel)).is_err());
    }
    assert!(
        lexer::lex_bounded(
            b"\"value\"",
            5,
            Some(&neutral_core::CancellationToken::new())
        )
        .is_ok()
    );
}
