// SPDX-License-Identifier: Apache-2.0

//! Tests the stable command parser without host effects.

use super::{ParseOutcome, parse};
use crate::constants;

#[test]
/// Verifies that the CLI shell accepts its help flag.
fn package_shell_cli_accepts_help() {
    assert_eq!(
        parse([constants::HELP.to_owned()]),
        Ok(ParseOutcome::Help(constants::USAGE))
    );
}

#[test]
/// Verifies that the CLI shell rejects a missing command.
fn package_shell_cli_rejects_empty_arguments() {
    assert!(parse(Vec::new()).is_err());
}

#[test]
/// Verifies incomplete vocabulary lock arguments fail before host acquisition.
fn unit_cli_rejects_partial_vocabulary_capture_policy() {
    assert!(
        parse([
            constants::VALIDATE.to_owned(),
            constants::VOCABULARY_BUNDLE.to_owned(),
            "bundle.json".to_owned(),
            "source.neu".to_owned(),
        ])
        .is_err()
    );
}
