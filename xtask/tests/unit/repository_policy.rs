// SPDX-License-Identifier: Apache-2.0

//! Whole-repository smoke checks kept separate from helper unit tests.

/// Current production and independent probe dependency graphs obey the declared boundaries.
#[test]
fn repository_dependency_boundaries_hold() {
    crate::checks::dependencies::check_boundaries().expect("dependency boundaries");
}

/// Test-only declarations retain their approved crate-local placement.
#[test]
fn repository_test_layout_is_valid() {
    crate::checks::test_layout::check_test_layout().expect("test layout");
}

/// Every required specification and fixture retains its traceability mapping.
#[test]
fn repository_traceability_is_complete() {
    crate::checks::traceability::check_traceability().expect("traceability");
}

/// Published workflow commands match the repository command grammar.
#[test]
fn repository_workflow_contract_is_valid() {
    crate::checks::repository::check_workflow_contract().expect("workflow contract");
}

/// Current conformance inputs exist and every configuration file is registered.
#[test]
fn repository_conformance_inventory_is_complete() {
    let root = crate::workspace_root().unwrap();
    let bundle = crate::ReleasedBundle::load(&root).expect("configured inherited bundle");
    crate::checks::traceability::ensure_registered_paths_exist(
        &root,
        &bundle.member("specs/REQUIREMENTS.md"),
    )
    .expect("conformance requirements");
    let mut files = Vec::new();
    crate::runtime::files::collect_regular_files(&root.join("config"), &mut files)
        .expect("configuration files");
    let inventory = files
        .iter()
        .map(|path| {
            path.strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/")
        })
        .collect::<Vec<_>>()
        .join("\n");
    crate::checks::traceability::ensure_inventory_registered(&root, "config", &inventory)
        .expect("configuration inventory");
    assert!(crate::checks::traceability::ensure_inventory_registered(&root, "config", "").is_err());
}
