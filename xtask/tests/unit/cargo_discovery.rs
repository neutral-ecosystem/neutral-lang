// SPDX-License-Identifier: Apache-2.0

//! Resolved Cargo graph and workspace discovery regressions.

use super::*;
use neutral_test_support::TestDirectory;

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
    let fixture = TestDirectory::new().expect("new tree");
    let root = fixture.path();
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
    for override_target in [None, Some(root.join("environment-output"))] {
        let mut child = std::process::Command::new(std::env::current_exe().unwrap());
        child
            .args([
                "--exact",
                "config::cargo_discovery::tests::cargo_discovery_configuration_child",
                "--nocapture",
            ])
            .env("NEUTRAL_TEST_CARGO_DISCOVERY_ROOT", &root)
            .env_remove("CARGO_TARGET_DIR")
            .env_remove("CARGO_BUILD_TARGET_DIR");
        let expected = override_target
            .as_ref()
            .map_or_else(|| root.join("custom-output"), Clone::clone);
        if let Some(target) = override_target {
            child.env("CARGO_TARGET_DIR", target);
        }
        let output = child
            .env("NEUTRAL_TEST_CARGO_DISCOVERY_TARGET", expected)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("1 passed"),
            "the discovery child must run its assertion"
        );
    }
}

/// Runs the actual discovery helper with environment changes confined to this child process.
#[test]
fn cargo_discovery_configuration_child() {
    let Some(root) = std::env::var_os("NEUTRAL_TEST_CARGO_DISCOVERY_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let expected = PathBuf::from(
        std::env::var_os("NEUTRAL_TEST_CARGO_DISCOVERY_TARGET").expect("expected target"),
    );
    let metadata = metadata(&root, false).expect("globbed workspace");
    assert_eq!(metadata.workspace_packages().len(), 1);
    assert_eq!(
        manifests(&root).expect("member manifest"),
        [root.join("packages/member/Cargo.toml")]
    );
    assert_eq!(metadata.target_directory.as_std_path(), expected);
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
