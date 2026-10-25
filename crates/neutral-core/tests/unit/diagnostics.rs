// SPDX-License-Identifier: Apache-2.0

//! Private-field regressions for diagnostic canonical ordering.

use super::*;
use crate::{ByteSpan, SourceContentDigest};

#[test]
/// Verifies every diagnostic tie-breaker agrees with equality and ordered collections.
fn diagnostic_ordering_resolves_equal_source_positions() {
    let location = SourceLocation::new(
        SourceContentDigest::from_bytes(b"source"),
        ByteSpan::new(0, 1).unwrap(),
    );
    let baseline = Diagnostic::new(
        DiagnosticCode::new("AAA").unwrap(),
        DiagnosticLayer::Syntax,
        DiagnosticSeverity::Note,
        location,
        Vec::new(),
        false,
    );
    assert_eq!(baseline.cmp(&baseline), std::cmp::Ordering::Equal);
    let mut variants = Vec::new();
    let mut value = baseline.clone();
    value.code = DiagnosticCode::new("BBB").unwrap();
    variants.push(value);
    variants.push(baseline.clone().with_related(vec![location]));
    let mut value = baseline.clone();
    value.parameters.push("detail".into());
    variants.push(value);
    let mut value = baseline.clone();
    value.layer = DiagnosticLayer::Semantics;
    variants.push(value);
    let mut value = baseline.clone();
    value.severity = DiagnosticSeverity::Error;
    variants.push(value);
    let mut value = baseline.clone();
    value.truncated = true;
    variants.push(value);
    for value in variants {
        assert!(baseline < value);
        assert!(value > baseline);
        assert_ne!(baseline, value);
    }
}
