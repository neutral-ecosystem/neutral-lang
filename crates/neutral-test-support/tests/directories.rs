// SPDX-License-Identifier: Apache-2.0

//! Temporary fixture ownership and cleanup independent of the package under test.

use neutral_test_support::TestDirectory;

/// Dropping one fixture removes its files without affecting another live fixture.
#[test]
fn temporary_directories_have_independent_ownership() {
    let first = TestDirectory::new().unwrap();
    let second = TestDirectory::new().unwrap();
    let first_path = first.path().to_owned();
    let second_path = second.path().to_owned();
    assert_ne!(first_path, second_path);
    std::fs::write(first.path().join("input"), "first").unwrap();
    std::fs::write(second.path().join("input"), "second").unwrap();
    drop(first);
    assert!(!first_path.exists());
    assert_eq!(
        std::fs::read_to_string(second.path().join("input")).unwrap(),
        "second"
    );
    drop(second);
    assert!(!second_path.exists());
}
