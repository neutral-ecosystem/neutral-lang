// SPDX-License-Identifier: Apache-2.0

//! Current source-header selection rejects historical and lookalike profiles.

use neutral_compiler::{ProjectCaptureLimitValues, ProjectCaptureLimits, source_module_name};
use neutral_core::CancellationToken;

/// Header scanning is bounded independently of subsequent source compilation.
fn limits() -> ProjectCaptureLimits {
    ProjectCaptureLimits::new(ProjectCaptureLimitValues {
        total_source_bytes: 4096,
        source_bytes_per_unit: 4096,
        source_units: 1,
        source_id_bytes: 128,
        module_id_bytes: 128,
        vocabulary_units: 1,
        vocabulary_bytes_per_unit: 4096,
        total_vocabulary_bytes: 4096,
        imports_per_module: 16,
        import_edges: 16,
        scc_units: 1,
        declarations: 64,
        diagnostics: 16,
        output_bytes: 4096,
    })
}

/// Exact current source selection is independent of package release versions.
#[test]
fn current_source_header_is_available() {
    assert_eq!(
        source_module_name(
            include_bytes!("../../../examples/minimal.neu"),
            limits(),
            &CancellationToken::new()
        )
        .unwrap(),
        "example"
    );
}

/// Capture accepts canonical snake-case segments without admitting generated-name suffixes.
#[test]
fn source_module_segments_keep_canonical_name_boundaries() {
    for name in ["a", "a1_b2", "a_2", "some_module::child2"] {
        let source = format!("neu \"1.0\"\nmodule {name}\n");
        assert_eq!(
            source_module_name(source.as_bytes(), limits(), &CancellationToken::new()).unwrap(),
            name
        );
    }
    for name in ["A", "1a", "_a", "a_", "a__b", "aB", "é", "a::", "a::::b"] {
        let source = format!("neu \"1.0\"\nmodule {name}\n");
        assert!(
            source_module_name(source.as_bytes(), limits(), &CancellationToken::new()).is_err(),
            "{name}"
        );
    }
}

/// Historical and normalized-looking version spellings never fall back to the current grammar.
#[test]
fn old_unknown_and_lookalike_source_profiles_fail_closed() {
    for version in ["0.1", "01.0", "1", "1.00", "2.0"] {
        let source = format!("neu \"{version}\"\nmodule example\npublic num answer = 42\n");
        assert!(
            source_module_name(source.as_bytes(), limits(), &CancellationToken::new()).is_err(),
            "{version}"
        );
    }
}

/// Cancellation takes precedence over publishing even a checked module spelling.
#[test]
fn header_selection_cancellation_recovers_on_the_same_bytes() {
    let bytes = include_bytes!("../../../examples/minimal.neu");
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert!(
        source_module_name(bytes, limits(), &cancelled)
            .unwrap_err()
            .is_cancelled()
    );
    assert_eq!(
        source_module_name(bytes, limits(), &CancellationToken::new()).unwrap(),
        "example"
    );
}
