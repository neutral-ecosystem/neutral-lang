// SPDX-License-Identifier: Apache-2.0

//! Current standalone command shell checks.

use std::process::Command;

#[test]
/// Keeps standalone help and version output plain and free of operational prefixes.
fn system_standalone_probe_help_and_version_are_plain_text() {
    for (argument, expected) in [
        (
            "--help",
            "Usage: neutral-probe [--json] [--root module::declaration ...] <artifact>\n"
                .to_owned(),
        ),
        (
            "--version",
            format!("neutral-probe {}\n", env!("CARGO_PKG_VERSION")),
        ),
    ] {
        let result = Command::new(env!("CARGO_BIN_EXE_neutral-probe"))
            .arg(argument)
            .output()
            .expect("probe must execute");
        assert!(result.status.success());
        assert_eq!(result.stdout, expected.as_bytes());
        assert_eq!(result.stderr, []);
    }
}
