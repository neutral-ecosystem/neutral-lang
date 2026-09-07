// SPDX-License-Identifier: Apache-2.0

//! Tests the standalone probe command boundary.

use super::run;

#[test]
/// Verifies that the probe accepts its help flag.
fn package_shell_probe_accepts_help() {
    assert!(run(["--help".to_owned()]).is_ok());
}

#[test]
/// Verifies that the probe rejects a missing artifact.
fn package_shell_probe_rejects_empty_arguments() {
    assert!(run(Vec::new()).is_err());
}

#[test]
/// Verifies that the probe reports an unreadable artifact without panicking.
fn package_shell_probe_rejects_an_unreadable_artifact() {
    assert!(run(["definitely-missing-neutral-artifact.nir".to_owned()]).is_err());
}

#[test]
/// Decode errors render a stable code with an offset only when one is available.
fn probe_renders_bounded_decode_errors() {
    let token = neutral_core::CancellationToken::new();
    let malformed = neutral_probe::inspect_encoded(
        &[0; neutral_encoding::constants::HEADER_BYTES],
        neutral_encoding::DecodeLimits::hard(),
        &token,
    )
    .unwrap_err();
    let expected = format!(
        "{} at encoded byte {}",
        malformed.code(),
        malformed.offset().unwrap()
    );
    assert_eq!(super::render_decode_error(malformed), expected);
    token.cancel();
    let cancelled =
        neutral_probe::inspect_encoded(&[], neutral_encoding::DecodeLimits::hard(), &token)
            .unwrap_err();
    assert_eq!(cancelled.offset(), None);
    assert_eq!(
        super::render_decode_error(cancelled),
        neutral_encoding::DecodeErrorClass::Cancelled.code()
    );
}
