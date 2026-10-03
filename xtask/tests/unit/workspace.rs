// SPDX-License-Identifier: Apache-2.0

//! Regression coverage for invocation-local workspace discovery.

use super::*;

#[test]
/// Shared binaries must discover the invoking checkout and skip isolated nested workspaces.
fn workspace_discovery_follows_the_invocation() {
    let root = std::env::temp_dir().join(format!(
        "neutral-workspace-discovery-{}",
        std::process::id()
    ));
    fs::create_dir(&root).expect("owned test directory must be new");
    for name in ["first", "second"] {
        let checkout = root.join(name);
        fs::create_dir_all(checkout.join("config")).expect("configuration directory");
        fs::create_dir_all(checkout.join("fuzz/subdirectory")).expect("nested directory");
        fs::write(
            checkout.join(constants::WORKSPACE_MANIFEST_FILE),
            "[workspace]\n",
        )
        .expect("workspace manifest");
        fs::write(
            checkout.join(constants::AUTOMATION_CONFIG_FILE),
            "schema_version = 1\n",
        )
        .expect("automation marker");
        fs::write(checkout.join("fuzz/Cargo.toml"), "[workspace]\n")
            .expect("isolated tool manifest");
        let expected = fs::canonicalize(&checkout).expect("canonical checkout");
        assert_eq!(discover(&checkout), Ok(expected.clone()));
        assert_eq!(discover(&checkout.join("fuzz/subdirectory")), Ok(expected));
    }
    assert!(discover(&root).is_err());
    fs::remove_dir_all(root).expect("remove owned temporary fixture");
}
