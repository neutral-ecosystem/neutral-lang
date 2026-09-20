// SPDX-License-Identifier: Apache-2.0

//! Public Stage 1 profile-dispatch conformance tests.

use neutral_compiler::{
    CompilationFailureDetail, CompilationRequest, CompilationResult, compile, diagnostics,
};
use neutral_core::{CancellationToken, StructuralLimits};

/// Compiles one exact fixture under small deterministic test limits.
fn compile_fixture(source: &[u8]) -> CompilationResult {
    compile(CompilationRequest::new(
        source.to_vec(),
        StructuralLimits::new(4_096, 16).expect("test limits must be valid"),
        CancellationToken::new(),
    ))
    .expect("fixture capture must succeed")
}

/// Extracts the stable public failure detail and first diagnostic code.
fn rejected_contract(result: CompilationResult) -> (CompilationFailureDetail, String) {
    let CompilationResult::Failure(failure) = result else {
        panic!("negative profile fixture must fail")
    };
    let code = failure
        .diagnostics()
        .first()
        .expect("profile failure must retain one diagnostic")
        .code()
        .as_str()
        .to_owned();
    (failure.detail(), code)
}

#[test]
/// Proves explicit v0 dispatch preserves authoritative inherited behavior.
fn conformance_stage1_positive_v0_profile_remains_available() {
    let source = include_bytes!("fixtures/stage1/v0-profile.neu");
    assert!(matches!(
        compile_fixture(source),
        CompilationResult::Success(_)
    ));
}

#[test]
/// Proves exact v1 selection is recognized and rejected as unavailable.
fn conformance_stage1_negative_and_boundary_v1_profiles_fail_closed() {
    for source in [
        include_bytes!("fixtures/stage1/v1-unavailable.neu").as_slice(),
        include_bytes!("fixtures/stage1/exact-v1-profile.neu").as_slice(),
    ] {
        assert_eq!(
            rejected_contract(compile_fixture(source)),
            (
                CompilationFailureDetail::FrontendUnavailable,
                diagnostics::PROFILE_UNAVAILABLE_DIAGNOSTIC.to_owned(),
            )
        );
    }
}

#[test]
/// Proves a v1 lookalike remains an unknown version instead of being normalized.
fn conformance_stage1_lookalike_profile_retains_unknown_version_behavior() {
    let source = include_bytes!("fixtures/stage1/v1-leading-zero.neu");
    assert_eq!(
        rejected_contract(compile_fixture(source)),
        (
            CompilationFailureDetail::SyntaxRejected,
            diagnostics::UNSUPPORTED_LANGUAGE_VERSION.to_owned(),
        )
    );
}

#[test]
/// Proves repeated and concurrent unavailable-profile dispatch is deterministic.
fn conformance_stage1_profile_rejection_is_repeatedly_and_concurrently_deterministic() {
    let source = include_bytes!("fixtures/stage1/v1-unavailable.neu");
    let expected = rejected_contract(compile_fixture(source));
    for _ in 0..32 {
        assert_eq!(rejected_contract(compile_fixture(source)), expected);
    }
    let workers = (0..8)
        .map(|_| {
            std::thread::spawn(|| {
                rejected_contract(compile_fixture(include_bytes!(
                    "fixtures/stage1/v1-unavailable.neu"
                )))
            })
        })
        .collect::<Vec<_>>();
    for worker in workers {
        assert_eq!(
            worker.join().expect("profile worker must not panic"),
            expected
        );
    }
}

#[test]
/// Proves excluded behavior cannot bypass the unavailable v1 profile gate.
fn conformance_stage1_v1_exclusions_fail_before_language_interpretation() {
    for source in [
        include_bytes!("fixtures/stage1/excluded-functions.neu").as_slice(),
        include_bytes!("fixtures/stage1/excluded-effects.neu").as_slice(),
        include_bytes!("fixtures/stage1/excluded-acquisition.neu").as_slice(),
        include_bytes!("fixtures/stage1/excluded-product-semantics.neu").as_slice(),
    ] {
        assert_eq!(
            rejected_contract(compile_fixture(source)),
            (
                CompilationFailureDetail::FrontendUnavailable,
                diagnostics::PROFILE_UNAVAILABLE_DIAGNOSTIC.to_owned(),
            )
        );
    }
}
