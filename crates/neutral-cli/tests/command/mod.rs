// SPDX-License-Identifier: Apache-2.0

//! Tests the stable command parser without host effects.

use super::{CommandKind, ParseOutcome, parse};
use crate::constants;

/// Positive numeric value used for every explicit test limit.
const TEST_LIMIT: &str = "7";

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

#[test]
/// Verifies a complete compile command retains every explicit host policy.
fn unit_cli_parses_complete_compile_policy() {
    let digest = neutral_core::VocabularyContentDigest::from_bytes(b"bundle").to_string();
    let arguments = [
        constants::COMPILE,
        "source.neu",
        constants::OUTPUT,
        "artifact.neuir",
        constants::OVERWRITE,
        constants::CANCEL_BEFORE_START,
        constants::VOCABULARY_BUNDLE,
        "bundle.json",
        constants::VOCABULARY_IDENTITY,
        "Fixture",
        constants::VOCABULARY_VERSION,
        "0.1.0",
        constants::VOCABULARY_ENCODING,
        "0.1",
        constants::VOCABULARY_SCHEMA,
        "0.1",
        constants::VOCABULARY_DIGEST,
        &digest,
        constants::VOCABULARY_FEATURE,
        "neutral.feature/records@1",
        constants::MAX_SOURCE_BYTES,
        TEST_LIMIT,
        constants::MAX_DIAGNOSTICS,
        TEST_LIMIT,
        constants::MAX_STRING_BYTES,
        TEST_LIMIT,
        constants::MAX_NUMERIC_DIGITS,
        TEST_LIMIT,
        constants::MAX_NUMERIC_SCALE,
        TEST_LIMIT,
        constants::MAX_DECLARATIONS,
        TEST_LIMIT,
        constants::MAX_RECORD_FIELDS,
        TEST_LIMIT,
        constants::MAX_NESTING_DEPTH,
        TEST_LIMIT,
        constants::MAX_LIST_ITEMS,
        TEST_LIMIT,
        constants::MAX_TRAVERSAL_NODES,
        TEST_LIMIT,
    ]
    .map(str::to_owned);
    let ParseOutcome::Execute(options) = parse(arguments).expect("command should parse") else {
        panic!("complete command should execute");
    };
    assert_eq!(options.kind, CommandKind::Compile);
    assert_eq!(options.input, "source.neu");
    assert_eq!(options.output.as_deref(), Some("artifact.neuir"));
    assert!(options.overwrite);
    assert!(options.cancel_before_start);
    assert_eq!(options.limits.source_bytes(), 7);
    assert_eq!(options.limits.diagnostics(), 7);
    assert_eq!(options.limits.string_bytes(), 7);
    assert_eq!(options.limits.numeric_digits(), 7);
    assert_eq!(options.limits.numeric_scale(), 7);
    assert_eq!(options.limits.declarations(), 7);
    assert_eq!(options.limits.record_fields(), 7);
    assert_eq!(options.limits.nesting_depth(), 7);
    assert_eq!(options.limits.list_items(), 7);
    assert_eq!(options.limits.traversal_nodes(), 7);
    assert_eq!(options.vocabulary.features, ["neutral.feature/records@1"]);
}

#[test]
/// Verifies command help, version, and command-specific output policy.
fn unit_cli_classifies_nonexecuting_and_output_policy_paths() {
    assert_eq!(
        parse([constants::VERSION.to_owned()]),
        Ok(ParseOutcome::Version)
    );
    assert_eq!(
        parse([constants::FORMAT.to_owned(), constants::HELP.to_owned()]),
        Ok(ParseOutcome::Help(constants::FORMAT_USAGE))
    );
    assert!(parse([constants::COMPILE.to_owned(), "source.neu".to_owned()]).is_err());
    assert!(
        parse([
            constants::VALIDATE.to_owned(),
            "source.neu".to_owned(),
            constants::OUTPUT.to_owned(),
            "output".to_owned(),
        ])
        .is_err()
    );
    assert!(
        parse([
            constants::FORMAT.to_owned(),
            "source.neu".to_owned(),
            constants::OUTPUT.to_owned(),
            constants::STANDARD_STREAM.to_owned(),
            constants::OVERWRITE.to_owned(),
        ])
        .is_err()
    );
}

#[test]
/// Verifies malformed, repeated, and competing input arguments fail closed.
fn unit_cli_rejects_malformed_argument_combinations() {
    for arguments in [
        vec!["unknown".to_owned()],
        vec![
            constants::VALIDATE.to_owned(),
            "source.neu".to_owned(),
            "second.neu".to_owned(),
        ],
        vec![
            constants::VALIDATE.to_owned(),
            "source.neu".to_owned(),
            "--unknown".to_owned(),
        ],
        vec![
            constants::VALIDATE.to_owned(),
            "source.neu".to_owned(),
            constants::MAX_SOURCE_BYTES.to_owned(),
            "0".to_owned(),
        ],
        vec![
            constants::VALIDATE.to_owned(),
            constants::STANDARD_STREAM.to_owned(),
            constants::VOCABULARY_BUNDLE.to_owned(),
            constants::STANDARD_STREAM.to_owned(),
            constants::VOCABULARY_IDENTITY.to_owned(),
            "Fixture".to_owned(),
            constants::VOCABULARY_VERSION.to_owned(),
            "0.1.0".to_owned(),
            constants::VOCABULARY_ENCODING.to_owned(),
            "0.1".to_owned(),
            constants::VOCABULARY_SCHEMA.to_owned(),
            "0.1".to_owned(),
            constants::VOCABULARY_DIGEST.to_owned(),
            neutral_core::VocabularyContentDigest::from_bytes(b"bundle").to_string(),
        ],
        // Missing value for option requiring value
        vec![
            constants::COMPILE.to_owned(),
            "source.neu".to_owned(),
            constants::OUTPUT.to_owned(),
        ],
        // Option set twice
        vec![
            constants::COMPILE.to_owned(),
            "source.neu".to_owned(),
            constants::OUTPUT.to_owned(),
            "out1".to_owned(),
            constants::OUTPUT.to_owned(),
            "out2".to_owned(),
        ],
        // Non-positive or non-integer for parse_u64 and parse_u32
        vec![
            constants::VALIDATE.to_owned(),
            "source.neu".to_owned(),
            constants::MAX_DIAGNOSTICS.to_owned(),
            "abc".to_owned(),
        ],
        vec![
            constants::VALIDATE.to_owned(),
            "source.neu".to_owned(),
            constants::MAX_DIAGNOSTICS.to_owned(),
            "0".to_owned(),
        ],
        vec![
            constants::VALIDATE.to_owned(),
            "source.neu".to_owned(),
            constants::MAX_STRING_BYTES.to_owned(),
            "abc".to_owned(),
        ],
        // Missing input source argument
        vec![
            constants::VALIDATE.to_owned(),
            constants::MAX_SOURCE_BYTES.to_owned(),
            "100".to_owned(),
        ],
    ] {
        assert!(parse(arguments).is_err());
    }
}
