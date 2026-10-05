// SPDX-License-Identifier: Apache-2.0

use super::*;
use crate::{
    CAPTURE_REQUEST_VERSION, CapturedProjectRequest, CapturedSourceInput, ProjectCaptureControls,
    ProjectCaptureLimitValues, ProjectCaptureLimits, capture_project, compile_project,
};
use neutral_core::profile::LanguageProfile;

/// Captures a tiny effect-free project with explicitly generous fixture controls.
fn captured(module: &str, value: &str) -> CapturedProject {
    let limits = ProjectCaptureLimits::new(ProjectCaptureLimitValues {
        total_source_bytes: 4096,
        source_bytes_per_unit: 1024,
        source_units: 8,
        source_id_bytes: 64,
        module_id_bytes: 64,
        vocabulary_units: 1,
        vocabulary_bytes_per_unit: 1024,
        total_vocabulary_bytes: 1024,
        imports_per_module: 8,
        import_edges: 16,
        scc_units: 8,
        declarations: 64,
        diagnostics: 16,
        output_bytes: 4096,
    });
    let source = format!(
        "neu \"{}\"\nmodule {module}\npublic string value = \"{value}\"\n",
        LanguageProfile::V1_0.source_version()
    );
    capture_project(CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        vec![CapturedSourceInput::new(
            "unit:test",
            module,
            source.into_bytes(),
        )],
        Vec::new(),
        ProjectCaptureControls::new(limits, CancellationToken::new()),
    ))
    .unwrap()
}

/// Constructs explicit retention budgets from the accepted capture controls.
fn cache(capture: &CapturedProject) -> ProjectCompilationCache {
    ProjectCompilationCache::new(ProjectCacheLimits {
        source_units: capture.limits().values().source_units,
        source_bytes: capture.limits().values().total_source_bytes,
    })
    .unwrap()
}

/// Invalid cache budgets cannot create an unbounded retention policy.
#[test]
fn zero_cache_retention_is_rejected() {
    assert!(
        ProjectCompilationCache::new(ProjectCacheLimits {
            source_units: 0,
            source_bytes: 1
        })
        .is_none()
    );
    assert!(
        ProjectCompilationCache::new(ProjectCacheLimits {
            source_units: 1,
            source_bytes: 0
        })
        .is_none()
    );
}

/// Simulated digest collisions cannot reuse syntax whose exact original bytes differ.
#[test]
fn cache_rejects_digest_collision_with_stale_bytes() {
    let token = CancellationToken::new();
    let first = captured("example", "first");
    let second = captured("example", "second");
    let mut cache = cache(&first);
    cache.compile(&first, &token).unwrap();
    cache.entries.get_mut("example").unwrap().digest = second.sources()[0].digest();
    let (actual, stats) = cache.compile(&second, &token).unwrap();
    assert_eq!(stats.parsed_units, 1);
    assert_eq!(stats.reused_units, 0);
    assert_eq!(stats.rejected_entries, 1);
    assert_eq!(actual, compile_project(&second, &token).unwrap());
}

/// A wrongly indexed source cannot reuse another module's identities or spans.
#[test]
fn cache_rejects_misplaced_module_and_profile_context() {
    let token = CancellationToken::new();
    let first = captured("first", "hello");
    let second = captured("second", "hello");
    let mut cache = cache(&first);
    cache.compile(&first, &token).unwrap();
    let mut entry = cache.entries.remove("first").unwrap();
    entry.digest = second.sources()[0].digest();
    entry.bytes = Arc::from(second.sources()[0].bytes());
    cache.entries.insert("second".to_owned(), entry);
    let (actual, stats) = cache.compile(&second, &token).unwrap();
    assert_eq!(stats.rejected_entries, 1);
    assert_eq!(stats.parsed_units, 1);
    assert_eq!(actual, compile_project(&second, &token).unwrap());
    cache.entries.get_mut("second").unwrap().profile =
        LanguageProfile::V0_1.source_version().to_owned();
    let (actual, stats) = cache.compile(&second, &token).unwrap();
    assert_eq!(stats.rejected_entries, 1);
    assert_eq!(actual, compile_project(&second, &token).unwrap());
}

/// Independent retention limits accept exact bytes and decline one byte over without changing compilation.
#[test]
fn cache_retention_exact_boundary_and_over_boundary() {
    let capture = captured("example", "hello");
    let length = capture.sources()[0].bytes().len() as u64;
    for budget in [length, length - 1] {
        let mut cache = ProjectCompilationCache::new(ProjectCacheLimits {
            source_units: 1,
            source_bytes: budget,
        })
        .unwrap();
        let (actual, _) = cache.compile(&capture, &CancellationToken::new()).unwrap();
        assert_eq!(
            actual,
            compile_project(&capture, &CancellationToken::new()).unwrap()
        );
        assert_eq!(cache.retained_units(), usize::from(budget == length));
        assert_eq!(
            cache.retained_source_bytes(),
            if budget == length { length } else { 0 }
        );
        let (_, stats) = cache.compile(&capture, &CancellationToken::new()).unwrap();
        assert_eq!(stats.reused_units, u64::from(budget == length));
        assert_eq!(stats.parsed_units, u64::from(budget != length));
    }
}

/// Unit retention is deterministic at and over its bound, without pruning complete compilation.
#[test]
fn cache_retention_unit_boundary_preserves_complete_output() {
    let first = captured("first", "hello");
    let second = captured("second", "hello");
    let sources = [&second, &first]
        .into_iter()
        .map(|capture| {
            let source = &capture.sources()[0];
            CapturedSourceInput::new(
                format!("unit:{}", source.module_id()),
                source.module_id(),
                source.bytes().to_vec(),
            )
        })
        .collect();
    let capture = capture_project(CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        first.profile(),
        sources,
        Vec::new(),
        ProjectCaptureControls::new(first.limits(), CancellationToken::new()),
    ))
    .unwrap();
    let token = CancellationToken::new();
    for source_units in [1, 2] {
        let mut cache = ProjectCompilationCache::new(ProjectCacheLimits {
            source_units,
            source_bytes: capture.limits().values().total_source_bytes,
        })
        .unwrap();
        let (cold, stats) = cache.compile(&capture, &token).unwrap();
        assert_eq!(stats.parsed_units, 2);
        assert_eq!(cache.retained_units() as u64, source_units);
        assert_eq!(cache.entries.keys().next().unwrap(), "first");
        let (warm, stats) = cache.compile(&capture, &token).unwrap();
        assert_eq!(stats.reused_units, source_units);
        assert_eq!(stats.parsed_units, 2 - source_units);
        assert_eq!(cold, warm);
        assert_eq!(warm, compile_project(&capture, &token).unwrap());
    }
}
