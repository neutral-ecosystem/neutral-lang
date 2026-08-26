// SPDX-License-Identifier: Apache-2.0

//! Shared names for Neutral automation commands, packages, and output.

/// Cargo executable used by workspace automation.
pub const CARGO_COMMAND: &str = "cargo";
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
/// Workspace package name for benchmarks.
pub const NEUTRAL_BENCH: &str = "neutral-bench";
/// Workspace package name for the command-line shell.
pub const NEUTRAL_CLI: &str = "neutral-cli";
/// Workspace package name for the compiler.
pub const NEUTRAL_COMPILER: &str = "neutral-compiler";
/// Workspace package name for the core contract types.
pub const NEUTRAL_CORE: &str = "neutral-core";
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
