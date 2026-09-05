// SPDX-License-Identifier: Apache-2.0

//! Shared names for Neutral automation commands, packages, and output.

/// Cargo executable used by workspace automation.
pub const CARGO_COMMAND: &str = "cargo";
/// Environment variable carrying delimiter-separated rustdoc flags.
pub const CARGO_ENCODED_RUSTDOCFLAGS: &str = "CARGO_ENCODED_RUSTDOCFLAGS";
/// Cargo metadata placeholder replaced while generating the rustdoc landing page.
pub const CARGO_METADATA_PLACEHOLDER: &str = "__NEUTRAL_CARGO_METADATA__";
/// Rust compiler executable used for toolchain verification.
pub const RUSTC_COMMAND: &str = "rustc";
/// Informational output category prefix.
pub const INFO: &str = "[info]";
/// Error output category prefix.
pub const ERROR: &str = "[error]";
/// Warning output category prefix.
pub const WARN: &str = "[warn]";
/// Machine-readable manifest output category prefix.
pub const MANIFEST: &str = "[manifest]";
/// Generated rustdoc output directory relative to the workspace root.
pub const RUSTDOC_OUTPUT_DIRECTORY: &str = "target/doc";
/// Generated workspace rustdoc landing-page filename.
pub const RUSTDOC_INDEX_FILE: &str = "index.html";
/// Rustdoc HTML header fragment relative to the workspace root.
pub const RUSTDOC_HEADER_FILE: &str = "xtask/src/rustdoc-header.html";
/// Rustdoc option that injects shared markup into every generated page.
pub const RUSTDOC_HTML_HEADER_FLAG: &str = "--html-in-header";
/// Separator used by Cargo's encoded compiler-flag environment variables.
pub const RUSTDOC_FLAG_SEPARATOR: char = '\u{1f}';
/// Rust configuration prefix used to invalidate cached docs after header changes.
pub const RUSTDOC_HEADER_CFG_PREFIX: &str = "neutral_rustdoc_header";
/// FNV-1a offset basis used for the non-security rustdoc cache token.
pub const RUSTDOC_HEADER_HASH_OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a prime used for the non-security rustdoc cache token.
pub const RUSTDOC_HEADER_HASH_PRIME: u64 = 0x0000_0100_0000_01b3;
/// Workspace package name for benchmarks.
pub const NEUTRAL_BENCH: &str = "neutral-bench";
/// Workspace package name for the command-line shell.
pub const NEUTRAL_CLI: &str = "neutral-cli";
/// Workspace package name for the compiler.
pub const NEUTRAL_COMPILER: &str = "neutral-compiler";
/// Workspace package name for the core contract types.
pub const NEUTRAL_CORE: &str = "neutral-core";
/// Workspace package name for external artifact encoding.
pub const NEUTRAL_ENCODING: &str = "neutral-encoding";
/// Workspace package name for the intermediate representation.
pub const NEUTRAL_IR: &str = "neutral-ir";
/// Workspace package name for the probe executable.
pub const NEUTRAL_PROBE: &str = "neutral-probe";
/// Workspace package name for the reader.
pub const NEUTRAL_READER: &str = "neutral-reader";
/// Workspace package name for the test suite.
pub const NEUTRAL_TEST_SUITE: &str = "neutral-test-suite";
/// Workspace package name for test support.
pub const NEUTRAL_TEST_SUPPORT: &str = "neutral-test-support";
/// Workspace package name for vocabulary definitions.
pub const NEUTRAL_VOCABULARY: &str = "neutral-vocabulary";
/// Workspace package name for this automation crate.
pub const XTASK: &str = "xtask";
