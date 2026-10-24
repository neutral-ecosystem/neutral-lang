// SPDX-License-Identifier: Apache-2.0

//! Clean compilation and actual caller-owned cache generations must agree.

use super::*;
use crate::{CompositionCompilationCache, ProjectCacheLimits};

/// Changing an earlier unit must not change which source occurrence exceeds a limit.
#[test]
fn expression_cache_preserves_source_order_for_limit_diagnostics() {
    let make = |extra: &str| {
        capture_expression_project(request_with_inputs(
            vec![
                CapturedSourceInput::new(
                    "source:a",
                    "a",
                    format!("neu \"1.0\"\nmodule a\n{extra}\n").into_bytes(),
                ),
                CapturedSourceInput::new(
                    "source:b",
                    "b",
                    b"neu \"1.0\"\nmodule b\nnum z = 1\nnum a = 2\n".to_vec(),
                ),
            ],
            vec![],
            expression_profile::CAPTURE_REQUEST_VERSION,
            expression_profile::SemanticProfiles::try_frozen().unwrap(),
            CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
                StructuralLimits::new(1_048_576, 64).unwrap(),
            )),
            (
                ExpressionLimits {
                    bindings: 2,
                    ..ExpressionLimits::hard()
                },
                ExpansionLimits::hard(),
            ),
            CancellationToken::new(),
        ))
        .unwrap()
    };
    let first = make("");
    let changed = make("num earlier = 0");
    let signal = CancellationToken::new();
    let mut cache = CompositionCompilationCache::new(ProjectCacheLimits {
        source_units: 8,
        source_bytes: 1_048_576,
    })
    .unwrap();
    cache.compile_expression(&first, &signal).unwrap();
    assert_eq!(
        cache.compile_expression(&changed, &signal).unwrap_err(),
        compile_expression_project(&changed, &signal).unwrap_err()
    );
}

/// One cache can change grammars without interpreting historical roots as successor syntax.
#[test]
fn expression_cache_profile_transitions_and_dropped_capture_do_not_leak() {
    let old = crate::tests::project_cache_support::capture_source("public num answer = 42");
    let mut cache = CompositionCompilationCache::new(ProjectCacheLimits {
        source_units: 8,
        source_bytes: 1_048_576,
    })
    .unwrap();
    let signal = CancellationToken::new();
    cache.compile(&old, &signal).unwrap();
    {
        let captured = capture_expression_project(request_with_inputs(
            vec![CapturedSourceInput::new(
                "source:example",
                "example",
                old.sources()[0].bytes().to_vec(),
            )],
            vec![],
            expression_profile::CAPTURE_REQUEST_VERSION,
            expression_profile::SemanticProfiles::try_frozen().unwrap(),
            old.composition_limits(),
            (ExpressionLimits::hard(), ExpansionLimits::hard()),
            CancellationToken::new(),
        ))
        .unwrap();
        let (_, stats) = cache.compile_expression(&captured, &signal).unwrap();
        assert_eq!(
            (
                stats.parsed_units,
                stats.reused_units,
                stats.rejected_entries
            ),
            (1, 0, 1)
        );
    }
    let (actual, stats) = cache.compile(&old, &signal).unwrap();
    assert_eq!(
        (
            stats.parsed_units,
            stats.reused_units,
            stats.rejected_entries
        ),
        (1, 0, 1)
    );
    assert_eq!(
        actual,
        crate::compile_composition_project(&old, &signal).unwrap()
    );
    let source = "neu \"1.0\"\nmodule example::expressions\ntemplate Item(static num value = 2) -> num { value }\npublic expand items = Item(value: axis) for (axis in [1, 2])\n";
    {
        let captured = capture(source, ExpressionLimits::hard(), ExpansionLimits::hard());
        cache.compile_expression(&captured, &signal).unwrap();
    }
    let recaptured = capture(source, ExpressionLimits::hard(), ExpansionLimits::hard());
    let (actual, stats) = cache.compile_expression(&recaptured, &signal).unwrap();
    assert_eq!(stats.reused_units, 1);
    assert_eq!(
        actual,
        compile_expression_project(&recaptured, &signal).unwrap()
    );
}

/// Newly retained template/matrix syntax cannot replace a generation on an allocation fault.
#[test]
fn expression_cache_candidate_retention_faults_are_atomic() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    let previous = capture(
        "neu \"1.0\"\nmodule example::expressions\npublic num answer = 42\n",
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let candidate = capture(
        include_str!("fixtures/positive/fault-matrix.neu"),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let signal = CancellationToken::new();
    let mut cache = CompositionCompilationCache::new(ProjectCacheLimits {
        source_units: 8,
        source_bytes: 1_048_576,
    })
    .unwrap();
    cache.compile_expression(&previous, &signal).unwrap();
    let (success, count) = observe(None, || cache.compile_expression(&candidate, &signal));
    assert!(success.is_ok());
    assert_ne!(count, 0);
    cache.compile_expression(&previous, &signal).unwrap();
    for index in 0..count {
        let (failed, _) = observe(Some(index), || {
            cache.compile_expression(&candidate, &signal)
        });
        assert!(failed.is_err(), "candidate allocation {index}/{count}");
        let cancelled = CancellationToken::new();
        let (failed, _) = observe_cancellation(index, &cancelled, || {
            cache.compile_expression(&candidate, &cancelled)
        });
        assert!(failed.is_err(), "candidate cancellation {index}/{count}");
        let (recovered, stats) = cache.compile_expression(&previous, &signal).unwrap();
        assert_eq!(stats.reused_units, 1);
        assert_eq!(
            value(&recovered, "answer"),
            &V::Number(ExactNumber::from_source("42", 64, 64).unwrap())
        );
    }
    let (_, stats) = cache.compile_expression(&candidate, &signal).unwrap();
    assert_eq!(stats.parsed_units, 1);
}

/// A failed next generation must neither evict successful syntax nor poison later requests.
#[test]
fn expression_cache_failures_preserve_previous_generation() {
    use neutral_core::allocation::testing::{observe, observe_cancellation};
    let good = capture(
        "neu \"1.0\"\nmodule example::expressions\npublic num result = 42\n",
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let bad = capture(
        "neu \"1.0\"\nmodule example::expressions\npublic num result = 1 / 0\n",
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let signal = CancellationToken::new();
    let mut cache = CompositionCompilationCache::new(ProjectCacheLimits {
        source_units: 8,
        source_bytes: 1_048_576,
    })
    .unwrap();
    let (expected, _) = cache.compile_expression(&good, &signal).unwrap();
    let size = cache.retained_source_bytes();
    assert_eq!(
        cache.compile_expression(&bad, &signal).unwrap_err(),
        compile_expression_project(&bad, &signal).unwrap_err()
    );
    let (_, count) = observe(None, || cache.compile_expression(&good, &signal));
    assert_ne!(count, 0);
    for index in 0..count {
        let (failed, _) = observe(Some(index), || cache.compile_expression(&good, &signal));
        assert!(failed.is_err(), "allocation {index}/{count}");
        assert_eq!(cache.retained_source_bytes(), size);
        assert_eq!(cache.retained_units(), 1);
        let cancelled = CancellationToken::new();
        let (failed, _) = observe_cancellation(index, &cancelled, || {
            cache.compile_expression(&good, &cancelled)
        });
        assert!(failed.is_err(), "cancellation {index}/{count}");
        assert_eq!(cache.retained_source_bytes(), size);
        let (recovered, stats) = cache.compile_expression(&good, &signal).unwrap();
        assert_eq!(recovered, expected);
        assert_eq!(stats.reused_units, 1);
    }
}

/// Retention limits govern cache admission, not the validity or values of the program.
#[test]
fn expression_cache_admission_bounds_are_inclusive() {
    let captured = capture(
        "neu \"1.0\"\nmodule example::expressions\npublic num answer = 42\n",
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let size = captured.sources()[0].bytes().len() as u64;
    let signal = CancellationToken::new();
    let expected = compile_expression_project(&captured, &signal).unwrap();
    for bound in [size - 1, size] {
        let mut cache = CompositionCompilationCache::new(ProjectCacheLimits {
            source_units: 1,
            source_bytes: bound,
        })
        .unwrap();
        assert_eq!(
            cache.compile_expression(&captured, &signal).unwrap().0,
            expected
        );
        let (actual, stats) = cache.compile_expression(&captured, &signal).unwrap();
        assert_eq!(actual, expected);
        assert_eq!(stats.reused_units, u64::from(bound == size));
        assert_eq!(
            cache.retained_source_bytes(),
            if bound == size { size } else { 0 }
        );
    }
}

/// Changed caller grammar controls must not bypass a private template's parser limits.
#[test]
fn expression_cache_rechecks_changed_private_template_controls() {
    let source = "neu \"1.0\"\nmodule example::expressions\ntemplate Item(static num first, static num second) -> num { first }\npublic num answer = 42\n";
    let first = capture(source, ExpressionLimits::hard(), ExpansionLimits::hard());
    let changed = capture(
        source,
        ExpressionLimits::hard(),
        ExpansionLimits {
            parameters: 1,
            ..ExpansionLimits::hard()
        },
    );
    let signal = CancellationToken::new();
    let mut cache = CompositionCompilationCache::new(ProjectCacheLimits {
        source_units: 8,
        source_bytes: 1_048_576,
    })
    .unwrap();
    cache.compile_expression(&first, &signal).unwrap();
    let clean = compile_expression_project(&changed, &signal).unwrap_err();
    assert_eq!(clean.code, ec::TPL_LIMIT);
    assert_eq!(
        cache.compile_expression(&changed, &signal).unwrap_err(),
        clean
    );
    let (_, stats) = cache.compile_expression(&first, &signal).unwrap();
    assert_eq!(stats.reused_units, 1);
}

/// A warm hit must actually reuse syntax, not replay capture or return a previous artifact.
#[test]
fn expression_cache_reuses_syntax_and_rebuilds_changed_units() {
    let first = capture(
        include_str!("fixtures/positive/arithmetic.neu"),
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let mut cache = CompositionCompilationCache::new(ProjectCacheLimits {
        source_units: 8,
        source_bytes: 1_048_576,
    })
    .unwrap();
    let signal = CancellationToken::new();
    let clean = compile_expression_project(&first, &signal).unwrap();
    let (cold, cold_stats) = cache.compile_expression(&first, &signal).unwrap();
    assert_eq!(cold, clean);
    assert_eq!((cold_stats.parsed_units, cold_stats.reused_units), (1, 0));
    let (warm, warm_stats) = cache.compile_expression(&first, &signal).unwrap();
    assert_eq!(warm, clean);
    assert_eq!((warm_stats.parsed_units, warm_stats.reused_units), (0, 1));
    let changed = capture(
        "neu \"1.0\"\nmodule example::expressions\npublic num result = 6 * 7\n",
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let (actual, stats) = cache.compile_expression(&changed, &signal).unwrap();
    assert_eq!(
        actual,
        compile_expression_project(&changed, &signal).unwrap()
    );
    assert_eq!(
        (
            stats.parsed_units,
            stats.reused_units,
            stats.rejected_entries
        ),
        (1, 0, 1)
    );
    assert_eq!(
        value(&actual, "result"),
        &V::Number(ExactNumber::from_source("42", 64, 64).unwrap())
    );
}
