// SPDX-License-Identifier: Apache-2.0

//! White-box stale/collision and successor context-key rejection regressions.

use super::*;
use crate::{
    CapturedCompositionProjectRequest, CapturedProjectRequest, CapturedSourceInput,
    ProjectCaptureControls, ProjectCaptureLimitValues, ProjectCaptureLimits,
    capture_composition_project,
};
use neutral_core::{StructuralLimits, profile::LanguageProfile};
use neutral_ir::{LogicalModuleIdentity, ModuleSymbolIdentity};
use neutral_vocabulary::{VocabularyLimits, composition::CompositionLimits};

/// Creates a tiny whole-project request, independent of package versions and portable files.
fn capture(value: &str) -> CapturedCompositionProject {
    let bounds = ProjectCaptureLimitValues {
        total_source_bytes: 4096,
        source_bytes_per_unit: 4096,
        source_units: 4,
        source_id_bytes: 64,
        module_id_bytes: 64,
        vocabulary_units: 4,
        vocabulary_bytes_per_unit: 4096,
        total_vocabulary_bytes: 4096,
        imports_per_module: 4,
        import_edges: 4,
        scc_units: 4,
        declarations: 16,
        diagnostics: 16,
        output_bytes: 16_384,
    };
    let bytes = format!("neu \"1.0\"\nmodule example\npublic num answer = {value}\n").into_bytes();
    capture_composition_project(CapturedCompositionProjectRequest::new(
        CapturedProjectRequest::new(
            profile::CAPTURE_REQUEST_VERSION,
            LanguageProfile::V1_0,
            vec![CapturedSourceInput::new("unit", "example", bytes)],
            Vec::new(),
            ProjectCaptureControls::new(
                ProjectCaptureLimits::new(bounds),
                CancellationToken::new(),
            ),
        ),
        profile::REQUIRED_FEATURES
            .iter()
            .map(|s| (*s).to_owned())
            .collect(),
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
            StructuralLimits::new(4096, 16).unwrap(),
        )),
    ))
    .unwrap()
}
/// Warms one exact private syntax entry through a successful whole compilation.
fn warm(captured: &CapturedCompositionProject) -> CompositionCompilationCache {
    let mut cache = CompositionCompilationCache::new(ProjectCacheLimits {
        source_units: 4,
        source_bytes: 4096,
    })
    .unwrap();
    cache.compile(captured, &CancellationToken::new()).unwrap();
    cache
}

/// Every warm-generation retention failure leaves the previous cache reusable and complete.
#[test]
fn security_composition_cache_retention_faults_preserve_previous_generation() {
    let captured = capture("42");
    let mut cache = warm(&captured);
    let mut count = 0;
    cache
        .compile_retention_observed(
            &captured,
            &CancellationToken::new(),
            &mut |_| Ok(()),
            &mut || {
                count += 1;
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(count, 4);
    let bytes = cache.retained_source_bytes();
    for fault in 0..count {
        let mut position = 0;
        let result = cache.compile_retention_observed(
            &captured,
            &CancellationToken::new(),
            &mut |_| Ok(()),
            &mut || {
                let current = position;
                position += 1;
                if current == fault {
                    Err(fail(codes::LIMIT, None))
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(result.unwrap_err().code, codes::LIMIT);
        assert_eq!(position, fault + 1);
        assert_eq!(cache.retained_source_bytes(), bytes);
        assert_eq!(cache.retained_units(), 1);
        let (actual, stats) = cache.compile(&captured, &CancellationToken::new()).unwrap();
        assert_eq!(stats.reused_units, 1);
        assert_eq!(
            actual,
            super::super::compile_composition_project(&captured, &CancellationToken::new())
                .unwrap()
        );
    }
}

/// Cache retention accepts the exact byte bound and declines one-over without changing semantics.
#[test]
fn integration_composition_cache_source_retention_boundary_is_exact() {
    let captured = capture("42");
    let size = captured.sources()[0].bytes().len() as u64;
    let expected =
        super::super::compile_composition_project(&captured, &CancellationToken::new()).unwrap();
    for bound in [size - 1, size, size + 1] {
        let mut cache = CompositionCompilationCache::new(ProjectCacheLimits {
            source_units: 1,
            source_bytes: bound,
        })
        .unwrap();
        let (actual, _) = cache.compile(&captured, &CancellationToken::new()).unwrap();
        assert_eq!(actual, expected);
        assert_eq!(cache.retained_units(), usize::from(bound >= size));
        assert_eq!(
            cache.retained_source_bytes(),
            if bound >= size { size } else { 0 }
        );
        let (_, stats) = cache.compile(&captured, &CancellationToken::new()).unwrap();
        assert_eq!(stats.reused_units, u64::from(bound >= size));
    }
}
/// A forged matching digest cannot substitute stale bytes and roots.
#[test]
fn security_composition_cache_collision_filter_requires_exact_bytes() {
    let original = capture("42");
    let changed = capture("43");
    let mut cache = warm(&original);
    cache.entries.get_mut("example").unwrap().digest = changed.sources()[0].digest();
    let (actual, stats) = cache.compile(&changed, &CancellationToken::new()).unwrap();
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
        super::super::compile_composition_project(&changed, &CancellationToken::new()).unwrap()
    );
}
/// A misplaced AST owner or changed feature context cannot hit an otherwise identical entry.
#[test]
fn security_composition_cache_rejects_wrong_module_and_feature_context() {
    let captured = capture("42");
    for wrong_module in [true, false] {
        let mut cache = warm(&captured);
        let entry = cache.entries.get_mut("example").unwrap();
        if wrong_module {
            entry.roots[0].owner = ModuleSymbolIdentity::new(
                LogicalModuleIdentity::new(neutral_core::profile::V1_SOURCE_PROFILE, "other"),
                "answer",
            );
        } else {
            entry.features = Arc::try_new(Vec::new()).unwrap();
        }
        let (actual, stats) = cache.compile(&captured, &CancellationToken::new()).unwrap();
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
            super::super::compile_composition_project(&captured, &CancellationToken::new())
                .unwrap()
        );
    }
}

/// Cancellation arriving after real parsing stops the shared clean/cache pipeline before semantic publication.
#[test]
fn security_composition_cancellation_after_parser_returns_no_project() {
    let captured = capture("42");
    let cancel = CancellationToken::new();
    let mut parsed = 0;
    let result = super::super::compile_with_parser(&captured, &cancel, &mut |source| {
        let roots = parse(source, &captured, &cancel)?;
        parsed += 1;
        cancel.cancel();
        Ok(roots)
    });
    assert_eq!(parsed, 1);
    assert_eq!(result.unwrap_err().code, codes::CANCELLED);
}

/// Every compiler phase, including final publication, atomically rejects cancellation or injected faults.
#[test]
fn security_composition_phase_failures_preserve_successful_cache_generation() {
    let phases = [
        Phase::Graph,
        Phase::Parse,
        Phase::Signatures,
        Phase::InitialScope,
        Phase::Defaults,
        Phase::FinalScope,
        Phase::Values,
        Phase::Bindings,
        Phase::Assemble,
        Phase::Publish,
    ];
    let previous = capture("42");
    let changed = capture("43");
    for phase in phases {
        for cancellation in [true, false] {
            let mut cache = warm(&previous);
            let before = cache.retained_source_bytes();
            let cancel = CancellationToken::new();
            let mut reached = false;
            let failure = cache
                .compile_observed(&changed, &cancel, &mut |current| {
                    if current == phase {
                        reached = true;
                        if cancellation {
                            cancel.cancel();
                        } else {
                            return Err(fail(codes::LIMIT, None));
                        }
                    }
                    Ok(())
                })
                .unwrap_err();
            assert!(reached, "phase {phase:?}");
            assert_eq!(
                failure.code,
                if cancellation {
                    codes::CANCELLED
                } else {
                    codes::LIMIT
                }
            );
            assert_eq!(cache.retained_source_bytes(), before);
            let (actual, stats) = cache.compile(&previous, &CancellationToken::new()).unwrap();
            assert_eq!(stats.parsed_units, 0);
            assert_eq!(
                actual,
                super::super::compile_composition_project(&previous, &CancellationToken::new())
                    .unwrap()
            );
        }
    }
}
