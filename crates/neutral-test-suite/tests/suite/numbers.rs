// SPDX-License-Identifier: Apache-2.0

//! numbers cross-package regressions.

use super::*;

#[test]
/// Verifies the active unit boundary produces exact minimal logical values.
fn unit_minimal_logical_value_is_exact() {
    let artifacts = compile_artifacts(MINIMAL_SOURCE);
    let declaration = &artifacts.logical_document().declarations()[0];
    assert_eq!(declaration.name(), "answer");
    assert_eq!(declaration.resolved_type().to_string(), "num");
    assert_eq!(declaration.value().to_string(), "42/1");
}

#[test]
/// Verifies compilation artifacts pass through the immutable reader boundary.
fn integration_minimal_compiler_to_reader() {
    let document = compile_reader(MINIMAL_SOURCE);
    assert_eq!(document.module_name(), "minimal");
    assert_eq!(document.declarations().len(), 1);
    assert!(document.declaration_by_name("answer").is_some());
}

#[test]
/// Verifies frozen numeric spellings lower to exact normalized public values.
fn conformance_exact_number_positive_oracles() {
    let cases = [
        (NUMBER_FRACTION, "123e-2/1"),
        (NUMBER_EXPONENT, "-125e1/1"),
        (NUMBER_SEPARATORS, "16777216/1"),
        (NUMBER_ZERO, "0/1"),
    ];
    for (source, expected) in cases {
        let artifacts = compile_artifacts(source);
        let declaration = &artifacts.logical_document().declarations()[0];
        assert_eq!(declaration.value().to_string(), expected);
    }
}

#[test]
/// Verifies equivalent exact numeric spellings share definition fingerprints.
fn property_equivalent_number_spellings_normalize_and_fingerprint_equally() {
    let spellings = ["1.2300", "123e-2", "0.01230e2"];
    let artifacts = spellings.map(|spelling| {
        compile_artifacts(
            format!("neu \"0.1\"\nmodule numeric\nnum answer = {spelling}\n").as_bytes(),
        )
    });
    let first = &artifacts[0].logical_document().declarations()[0];
    for artifacts in &artifacts[1..] {
        let declaration = &artifacts.logical_document().declarations()[0];
        assert_eq!(declaration.value(), first.value());
        assert_eq!(declaration.fingerprint(), first.fingerprint());
    }
}

#[test]
/// Verifies frozen malformed numeric spellings produce stable semantic diagnostics.
fn conformance_exact_number_negative_oracles() {
    let cases = [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/numbers/number-invalid-separator.neu"
            ) as &[u8],
            (96, 100),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/numbers/number-missing-fraction.neu"
            ),
            (95, 97),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/numbers/number-invalid-exponent.neu"
            ),
            (95, 98),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/numbers/number-base-prefix.neu"
            ),
            (90, 94),
        ),
    ];
    for (source, span) in cases {
        let failure = compile_failure(source);
        assert_eq!(failure.class(), ResultClass::Semantics);
        assert_eq!(
            failure.diagnostics()[0].code().as_str(),
            diagnostics::INVALID_NUMBER
        );
        assert_eq!(span_pair(failure.diagnostics()[0].primary().span()), span);
    }
}

#[test]
/// Verifies digit and scale bounds reject values before exact-number allocation.
fn security_exact_number_limits_fail_before_ir_allocation() {
    let digit_limits = limits()
        .with_numeric_digits(4)
        .expect("numeric digit limit should be valid");
    let scale_limits = limits()
        .with_numeric_scale(2)
        .expect("numeric scale limit should be valid");
    for (source, limits, span) in [
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/numbers/number-digit-limit.neu"
            ) as &[u8],
            digit_limits,
            (90, 95),
        ),
        (
            include_bytes!(
                "../../../../conformance/releases/v0.1.0/specs/fixtures/negative/numbers/number-scale-limit.neu"
            ),
            scale_limits,
            (90, 93),
        ),
    ] {
        let result = compile(CompilationRequest::new(
            source.to_vec(),
            limits,
            CancellationToken::new(),
        ))
        .expect("numeric fixture bytes should capture");
        let CompilationResult::Failure(failure) = result else {
            panic!("over-limit number must produce no IR");
        };
        assert_eq!(failure.class(), ResultClass::Resource);
        assert_eq!(
            failure.diagnostics()[0].code().as_str(),
            diagnostics::NUMBER_LIMIT_EXCEEDED
        );
        assert_eq!(span_pair(failure.diagnostics()[0].primary().span()), span);
    }
}
