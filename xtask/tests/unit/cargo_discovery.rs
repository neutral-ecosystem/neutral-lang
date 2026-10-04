// SPDX-License-Identifier: Apache-2.0

//! Resolved Cargo graph and workspace discovery regressions.

use super::*;

/// Dependency aliases never change the package names enforced by boundary policy.
#[test]
fn automation_cargo_discovery_uses_resolved_package_ids() {
    let root = crate::workspace_root().expect("workspace root");
    let mut metadata = metadata(&root, true).expect("resolved metadata");
    let expected =
        direct_dependencies(&metadata, crate::constants::NEUTRAL_COMPILER).expect("normal edges");
    for node in &mut metadata.resolve.as_mut().expect("resolved graph").nodes {
        for dependency in &mut node.deps {
            dependency.name = "renamed_dependency".to_owned();
        }
    }
    assert_eq!(
        direct_dependencies(&metadata, crate::constants::NEUTRAL_COMPILER).expect("renamed edges"),
        expected
    );
    let packages =
        closure(&metadata, crate::constants::NEUTRAL_COMPILER, false).expect("normal closure");
    assert!(packages.contains(crate::constants::NEUTRAL_COMPILER));
    assert!(!packages.contains(crate::constants::XTASK));
    assert!(!packages.contains("toml"));
    let probe = closure(&metadata, crate::constants::NEUTRAL_PROBE, true).expect("probe closure");
    assert!(!probe.contains(crate::constants::NEUTRAL_COMPILER));
    assert!(package(&metadata, "missing-package").is_err());
    metadata.resolve = None;
    assert!(direct_dependencies(&metadata, crate::constants::NEUTRAL_COMPILER).is_err());
    assert!(closure(&metadata, crate::constants::NEUTRAL_COMPILER, false).is_err());
}

/// Cargo expands globs, applies excludes, and selects configured target directories.
#[test]
fn automation_cargo_discovery_respects_workspace_configuration() {
    let root = std::env::temp_dir().join(format!("neutral-cargo-discovery-{}", std::process::id()));
    std::fs::create_dir(&root).expect("new tree");
    std::fs::create_dir_all(root.join("packages/member/src")).expect("member tree");
    std::fs::create_dir_all(root.join("packages/excluded/src")).expect("excluded tree");
    std::fs::create_dir_all(root.join(".cargo")).expect("Cargo configuration");
    std::fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers=['packages/*']\nexclude=['packages/excluded']\nresolver='3'\n",
    )
    .expect("workspace");
    for name in ["member", "excluded"] {
        std::fs::write(
            root.join(format!("packages/{name}/Cargo.toml")),
            format!("[package]\nname='{name}'\nversion='1.0.0'\nedition='2024'\n"),
        )
        .expect("package");
        std::fs::write(
            root.join(format!("packages/{name}/src/lib.rs")),
            "// Empty discovery fixture.\n",
        )
        .expect("library");
    }
    std::fs::write(
        root.join("Cargo.lock"),
        "version=4\n[[package]]\nname='member'\nversion='1.0.0'\n",
    )
    .expect("lock");
    std::fs::write(
        root.join(".cargo/config.toml"),
        "[build]\ntarget-dir='custom-output'\n",
    )
    .expect("output configuration");
    let root = root.canonicalize().expect("canonical root");
    let metadata = metadata(&root, false).expect("globbed workspace");
    assert_eq!(metadata.workspace_packages().len(), 1);
    assert_eq!(
        manifests(&root).expect("member manifest"),
        [root.join("packages/member/Cargo.toml")]
    );
    if std::env::var_os("CARGO_TARGET_DIR").is_none() {
        assert_eq!(
            metadata.target_directory.as_std_path(),
            root.join("custom-output")
        );
    }
    std::fs::remove_dir_all(root).expect("remove owned tree");
}

/// Benchmark discovery rejects unrelated and missing executable build records.
#[test]
fn automation_cargo_build_messages_require_the_selected_target() {
    assert!(
        benchmark_executable(
            "a normal build log\n{\"reason\":\"build-finished\",\"success\":true}\n",
            "bench"
        )
        .is_err()
    );
}
