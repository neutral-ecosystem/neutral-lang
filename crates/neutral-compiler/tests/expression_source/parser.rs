// SPDX-License-Identifier: Apache-2.0

//! Raw parser boundaries and original byte spans before captured source validation.

use super::super::{CancellationToken, ExpansionLimits, ExpressionLimits};
use super::*;

#[test]
fn raw_lexing_rejects_invalid_utf8_and_preserves_offsets() {
    let cancel = CancellationToken::new();
    let mut budget = Budget {
        expression: ExpressionLimits::hard(),
        expansion: ExpansionLimits::hard(),
        cancel: &cancel,
        work: 0,
        bytes: 0,
        nodes: 0,
        applications: 0,
        tuples: 0,
        expanded: 0,
        expanded_nodes: 0,
        expansion_work: 0,
        expansion_bytes: 0,
    };
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
