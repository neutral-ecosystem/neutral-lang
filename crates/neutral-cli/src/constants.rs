// SPDX-License-Identifier: Apache-2.0

//! Stable command, option, output, and default-policy spellings.

/// Compile command spelling.
pub const COMPILE: &str = "compile";
/// Validate command spelling.
pub const VALIDATE: &str = "validate";
/// Format command spelling.
pub const FORMAT: &str = "format";
/// Help option spelling.
pub const HELP: &str = "--help";
/// Version option spelling.
pub const VERSION: &str = "--version";
/// Output destination option spelling.
pub const OUTPUT: &str = "--output";
/// Existing-output replacement option spelling.
pub const OVERWRITE: &str = "--overwrite";
/// Explicit pre-work cancellation option spelling.
pub const CANCEL_BEFORE_START: &str = "--cancel-before-start";
/// Standard stream path sentinel.
pub const STANDARD_STREAM: &str = "-";
/// Captured vocabulary bundle path option spelling.
pub const VOCABULARY_BUNDLE: &str = "--vocabulary-bundle";
/// Vocabulary lock identity option spelling.
pub const VOCABULARY_IDENTITY: &str = "--vocabulary-identity";
/// Vocabulary lock release option spelling.
pub const VOCABULARY_VERSION: &str = "--vocabulary-version";
/// Vocabulary lock encoding version option spelling.
pub const VOCABULARY_ENCODING: &str = "--vocabulary-encoding-version";
/// Vocabulary lock schema version option spelling.
pub const VOCABULARY_SCHEMA: &str = "--vocabulary-schema-version";
/// Vocabulary exact digest option spelling.
pub const VOCABULARY_DIGEST: &str = "--vocabulary-digest";
/// Repeatable vocabulary required-feature option spelling.
pub const VOCABULARY_FEATURE: &str = "--vocabulary-feature";
/// Source byte limit option spelling.
pub const MAX_SOURCE_BYTES: &str = "--max-source-bytes";
/// Diagnostic count limit option spelling.
pub const MAX_DIAGNOSTICS: &str = "--max-diagnostics";
/// String byte limit option spelling.
pub const MAX_STRING_BYTES: &str = "--max-string-bytes";
/// Exact-number digit limit option spelling.
pub const MAX_NUMERIC_DIGITS: &str = "--max-numeric-digits";
/// Exact-number scale limit option spelling.
pub const MAX_NUMERIC_SCALE: &str = "--max-numeric-scale";
/// Root declaration limit option spelling.
pub const MAX_DECLARATIONS: &str = "--max-declarations";
/// Record field limit option spelling.
pub const MAX_RECORD_FIELDS: &str = "--max-record-fields";
/// Value nesting depth limit option spelling.
pub const MAX_NESTING_DEPTH: &str = "--max-nesting-depth";
/// List item limit option spelling.
pub const MAX_LIST_ITEMS: &str = "--max-list-items";
/// Value traversal-node limit option spelling.
pub const MAX_TRAVERSAL_NODES: &str = "--max-traversal-nodes";
/// Error output category prefix.
pub const ERROR: &str = "[error]";
/// Informational output category prefix.
pub const INFO: &str = "[info]";
/// Stable generic CLI usage.
pub const USAGE: &str = "neutral-cli <compile|validate|format> [options] <source>";
/// Stable compile command usage.
pub const COMPILE_USAGE: &str = "neutral-cli compile --output <artifact|-> [options] <source|->";
/// Stable validate command usage.
pub const VALIDATE_USAGE: &str = "neutral-cli validate [options] <source|->";
/// Stable format command usage.
pub const FORMAT_USAGE: &str = "neutral-cli format --output <source|-> [options] <source|->";
/// Default captured source byte ceiling.
pub const DEFAULT_SOURCE_BYTES: u64 = 16_777_216;
/// Default retained diagnostic ceiling.
pub const DEFAULT_DIAGNOSTICS: u32 = 64;
/// Default decoded string byte ceiling.
pub const DEFAULT_STRING_BYTES: u64 = 1_048_576;
/// Default exact-number significant digit ceiling.
pub const DEFAULT_NUMERIC_DIGITS: u64 = 1_000_000;
/// Default exact-number absolute scale ceiling.
pub const DEFAULT_NUMERIC_SCALE: u64 = 1_000_000;
/// Default root declaration ceiling.
pub const DEFAULT_DECLARATIONS: u64 = 100_000;
/// Default record field ceiling.
pub const DEFAULT_RECORD_FIELDS: u64 = 100_000;
/// Default recursive value nesting ceiling.
pub const DEFAULT_NESTING_DEPTH: u64 = 128;
/// Default list item ceiling.
pub const DEFAULT_LIST_ITEMS: u64 = 1_000_000;
/// Default recursive value traversal ceiling.
pub const DEFAULT_TRAVERSAL_NODES: u64 = 1_000_000;
/// Maximum attempts to reserve a same-directory atomic temporary name.
pub const MAXIMUM_TEMPORARY_ATTEMPTS: u64 = 128;
/// Temporary output filename marker.
pub const TEMPORARY_MARKER: &str = ".neutral-tmp-";
/// Command usage failure exit code.
pub const EXIT_USAGE: u8 = 2;
/// Host input acquisition failure exit code.
pub const EXIT_INPUT: u8 = 3;
/// Source, vocabulary, or artifact validation failure exit code.
pub const EXIT_VALIDATION: u8 = 4;
/// Output policy or commit failure exit code.
pub const EXIT_OUTPUT: u8 = 5;
/// Cooperative cancellation exit code.
pub const EXIT_CANCELLED: u8 = 6;
/// Internal invariant failure exit code.
pub const EXIT_INTERNAL: u8 = 70;
