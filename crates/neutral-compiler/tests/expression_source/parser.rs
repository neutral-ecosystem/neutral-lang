// SPDX-License-Identifier: Apache-2.0

//! Raw parser boundaries and original byte spans before captured source validation.

use super::super::{CancellationToken, ExpansionLimits, ExpressionLimits};
use super::*;

#[test]
fn raw_lexing_rejects_invalid_utf8_and_preserves_offsets() {
    let cancel = CancellationToken::new();
    let mut budget = budget(&cancel);
    let failure = lex(&[0xff], 0, &mut budget).err().unwrap();
    assert_eq!(failure, error(ec::EXP_SYNTAX, None));
    assert_eq!(budget.work, 0);
    assert_eq!(budget.bytes, 0);

    let tokens = lex(b"num value = 2", 7, &mut budget).unwrap();
    assert_eq!(tokens.len(), 4);
    assert_eq!(tokens[0].spelling, "num");
    assert_eq!(tokens[0].span, ByteSpan::new(7, 10).unwrap());
    assert_eq!(tokens[3].spelling, "2");
    assert_eq!(tokens[3].span, ByteSpan::new(19, 20).unwrap());
    assert!(ty(&tokens[..1], b"       num value = 2").is_ok());
    assert_eq!(ty(&[], b"").unwrap_err(), error(ec::EXP_SYNTAX, None));
}

/// Fresh independently bounded parsing state for one raw source operation.
fn budget(cancel: &CancellationToken) -> Budget<'_> {
    Budget {
        expression: ExpressionLimits::hard(),
        expansion: ExpansionLimits::hard(),
        cancel,
        work: 0,
        bytes: 0,
        nodes: 0,
        applications: 0,
        tuples: 0,
        expanded: 0,
        expanded_nodes: 0,
        expansion_work: 0,
        expansion_bytes: 0,
    }
}

/// Unterminated quoted and interpolated groups reject rather than exposing an incomplete token.
#[test]
fn raw_strings_and_interpolations_reject_truncated_groups_and_limits() {
    let cancel = CancellationToken::new();
    for source in [
        b"\"unfinished".as_slice(),
        b"$\"${1",
        b"$\"${\"unfinished",
        b"\"trailing\\",
    ] {
        assert_eq!(
            lex(source, 0, &mut budget(&cancel))
                .err()
                .expect("invalid raw source"),
            error(ec::EXP_SYNTAX, None)
        );
    }
    let mut bounded = budget(&cancel);
    bounded.expression.syntax_depth = 1;
    assert_eq!(
        interpolation_end("{{value}}", 1, &mut bounded).unwrap_err(),
        error(ec::EXP_LIMIT, None)
    );
    let source = r#"$"${$"inner ${1}"}""#;
    let tokens = lex(source.as_bytes(), 0, &mut budget(&cancel)).unwrap();
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].spelling, source);
    assert_eq!(
        string("name", &mut budget(&cancel)),
        Err(error(ec::EXP_SYNTAX, None))
    );
}

/// Inner comment and string scans observe work exhaustion and cancellation before publication.
#[test]
fn raw_scans_check_inner_work_and_cancellation_boundaries() {
    for source in ["/* long comment */", "\"long string\"", "$\"${value}\""] {
        let cancel = CancellationToken::new();
        let mut baseline = budget(&cancel);
        lex(source.as_bytes(), 0, &mut baseline).unwrap();
        for work in 0..baseline.work {
            let mut bounded = budget(&cancel);
            bounded.expression.work = work;
            assert_eq!(
                lex(source.as_bytes(), 0, &mut bounded).err(),
                Some(error(ec::EXP_LIMIT, None)),
                "work {work}: {source}"
            );
        }
        cancel.cancel();
        assert_eq!(
            lex(source.as_bytes(), 0, &mut budget(&cancel))
                .err()
                .expect("cancelled scan must fail"),
            error(ec::EXP_CANCELLED, None)
        );
    }
}

/// Group/name helpers reject mismatched delimiters, unbalanced members and non-name qualifiers.
#[test]
fn raw_group_member_and_qualified_name_boundaries_are_exact() {
    let cancel = CancellationToken::new();
    for source in [")", "(", "1,,2", ",1"] {
        let tokens = lex(source.as_bytes(), 0, &mut budget(&cancel)).unwrap();
        assert_eq!(
            split(&tokens).err(),
            Some(error(ec::EXP_SYNTAX, None)),
            "{source}"
        );
    }
    for source in ["(value", "[value]"] {
        let tokens = lex(source.as_bytes(), 0, &mut budget(&cancel)).unwrap();
        assert_eq!(
            group(&tokens, &mut 0, "(", ")").err(),
            Some(error(ec::EXP_SYNTAX, None))
        );
    }
    let tokens = lex(b"name::7", 0, &mut budget(&cancel)).unwrap();
    assert_eq!(
        name(&tokens, &mut 0).unwrap_err(),
        error(ec::EXP_SYNTAX, None)
    );
}
