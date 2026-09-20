// SPDX-License-Identifier: Apache-2.0

//! Public profile-discovery boundary tests.

use neutral_core::profile;

#[test]
/// Proves the reader reports the shared catalogue without compiler linkage.
fn reader_profile_catalogue_is_the_shared_public_contract() {
    assert_eq!(
        neutral_reader::language_profiles(),
        profile::language_profiles()
    );
    assert_eq!(
        neutral_reader::language_profiles()
            .iter()
            .map(|descriptor| descriptor.profile().source_version())
            .collect::<Vec<_>>(),
        ["0.1", "1.0"]
    );
}
