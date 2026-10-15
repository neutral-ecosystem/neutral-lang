// SPDX-License-Identifier: Apache-2.0

//! Exact malformed-string spans and interruption boundaries in raw token acquisition.

use super::*;
use crate::frontend::FrontendErrorKind as K;
use neutral_core::{
    CancellationToken,
    allocation::testing::{observe, observe_cancellation},
};

/// Unterminated, malformed and non-scalar source bytes keep distinct exact original spans.
#[test]
fn malformed_strings_preserve_error_kinds_and_byte_spans() {
    for (source, kind, start, end) in [
        (
            b"\"unfinished".as_slice(),
            K::UnterminatedStringLiteral,
            0,
            11,
        ),
        (
            b"\"trailing\\".as_slice(),
            K::UnterminatedStringLiteral,
            9,
            10,
        ),
        (b"\"\\u0041\"".as_slice(), K::InvalidStringLiteral, 1, 4),
        ("\"\u{0085}\"".as_bytes(), K::InvalidStringLiteral, 1, 3),
    ] {
        let error = lex(source).unwrap_err();
        assert_eq!(error.kind, kind, "{source:?}");
        assert_eq!(error.span, span(start, end), "{source:?}");
    }
}

/// Each scalar retention boundary propagates allocation failure and mid-scan cancellation.
#[test]
fn string_and_comment_scans_fail_atomically_at_every_reservation() {
    for source in ["\"é\\n\\u{1F642}\"", "/* one\ntwo */"] {
        let cancel = CancellationToken::new();
        let (baseline, checkpoints) =
            observe(None, || lex_bounded(source.as_bytes(), 64, Some(&cancel)));
        baseline.unwrap();
        assert!(checkpoints > 0);
        for index in 0..checkpoints {
            let (failure, _) = observe(Some(index), || {
                lex_bounded(source.as_bytes(), 64, Some(&cancel))
            });
            assert_eq!(
                failure.unwrap_err().kind,
                K::RecordLimitExceeded,
                "allocation {index}: {source}"
            );
            let signal = CancellationToken::new();
            let (failure, _) = observe_cancellation(index, &signal, || {
                lex_bounded(source.as_bytes(), 64, Some(&signal))
            });
            assert_eq!(
                failure.unwrap_err().kind,
                K::RecordLimitExceeded,
                "cancellation {index}: {source}"
            );
        }
        lex(source.as_bytes()).unwrap();
    }
}

/// Decoded bytes are bounded for raw Unicode, ordinary escapes and Unicode escapes alike.
#[test]
fn decoded_scalar_limits_precede_all_string_push_paths() {
    for source in ["\"é\"", "\"\\n\"", "\"\\u{41}\""] {
        let error = lex_bounded(source.as_bytes(), 0, None).unwrap_err();
        assert_eq!(error.kind, K::RecordLimitExceeded);
    }
    let signal = CancellationToken::new();
    signal.cancel();
    let mut value = String::new();
    assert_eq!(
        Retention {
            string_bytes: 64,
            cancellation: Some(&signal)
        }
        .push(&mut value, 'a', 3)
        .unwrap_err()
        .span,
        span(3, 3)
    );
    assert_eq!(value, "");
}

/// ASCII-only token retention rejects non-ASCII bytes at the supplied source offset.
#[test]
fn ascii_token_retention_checks_its_own_boundary() {
    assert_eq!(ascii_text("é".as_bytes(), 7).unwrap_err().span, span(7, 9));
    assert_eq!(ascii_text(b"name", 7).unwrap(), "name");
}
