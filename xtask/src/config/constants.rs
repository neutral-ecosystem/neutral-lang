// SPDX-License-Identifier: Apache-2.0

//! Shared names for Neutral automation commands, packages, and output.

/// Repository configuration schema, unrelated to package release versions.
pub const CONFIG_SCHEMA_VERSION: u32 = 1;
/// Explicit test-runner override; no silent fallback is permitted.
pub const TEST_RUNNER_ENV: &str = "NEUTRAL_TEST_RUNNER";
/// Explicit per-invocation test-report verbosity override.
pub const TEST_VERBOSE_ENV: &str = "NEUTRAL_TEST_VERBOSE";
/// Cargo and nextest's shared color selection.
pub const CARGO_TERM_COLOR_ENV: &str = "CARGO_TERM_COLOR";

/// Rust formatter executable used for environment verification.
pub const RUSTFMT_COMMAND: &str = "rustfmt";
/// Rust toolchain manager used to isolate analysis-only nightly tools.
pub const RUSTUP_COMMAND: &str = "rustup";
/// Valgrind executable used for release allocation and memory review.
pub const VALGRIND_COMMAND: &str = "valgrind";
/// Git executable required for repository and release identity checks.
pub const GIT_COMMAND: &str = "git";
/// POSIX shell executable required by the supported Linux adapter.
pub const POSIX_SHELL_COMMAND: &str = "sh";
/// Archive executable required for release assembly.
pub const TAR_COMMAND: &str = "tar";
/// TLS-capable transfer executable required by verified setup procedures.
pub const CURL_COMMAND: &str = "curl";
/// SHA-256 executable required by verified setup and release procedures.
pub const SHA256_COMMAND: &str = "sha256sum";
/// Kernel/host identification executable used in environment evidence.
pub const UNAME_COMMAND: &str = "uname";
/// Environment variable carrying delimiter-separated rustdoc flags.
pub const CARGO_ENCODED_RUSTDOCFLAGS: &str = "CARGO_ENCODED_RUSTDOCFLAGS";
/// Cargo metadata placeholder replaced while generating the rustdoc landing page.
pub const CARGO_METADATA_PLACEHOLDER: &str = "__NEUTRAL_CARGO_METADATA__";
/// Informational output category prefix.
pub const INFO: &str = "[info]";
/// Error output category prefix.
pub const ERROR: &str = "[error]";
/// Warning output category prefix.
pub const WARN: &str = "[warn]";
/// Machine-readable manifest output category prefix.
pub const MANIFEST: &str = "[manifest]";
/// Generated workspace rustdoc landing-page filename.
pub const RUSTDOC_INDEX_FILE: &str = "index.html";
/// Repository-owned source directory for shared visual assets.
pub const ASSET_DIRECTORY: &str = "assets";
/// Documentation-site directory containing copied visual assets.
pub const DOCUMENTATION_ASSET_DIRECTORY: &str = "assets";
/// Official square Neutral logo filename.
pub const NEUTRAL_LOGO_FILE: &str = "neutral-logo.png";
/// Official rounded Neutral favicon filename.
pub const NEUTRAL_ROUNDED_LOGO_FILE: &str = "neutral-logo-rounded.png";
/// Official transparent Neutral brand-mark filename.
pub const NEUTRAL_TRANSPARENT_LOGO_FILE: &str = "neutral-logo-transparent.png";
/// Official Neutral assets copied into generated documentation.
pub const DOCUMENTATION_ASSET_FILES: &[&str] = &[
    NEUTRAL_LOGO_FILE,
    NEUTRAL_ROUNDED_LOGO_FILE,
    NEUTRAL_TRANSPARENT_LOGO_FILE,
];
/// Rustdoc HTML header fragment relative to the workspace root.
pub const RUSTDOC_HEADER_FILE: &str = "xtask/templates/rustdoc-header.html";
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
/// Explicit inherited-language conformance selection relative to the workspace root.
pub const CONFORMANCE_CONFIG_FILE: &str = "config/conformance.toml";
/// Local automation defaults relative to the workspace root.
pub const AUTOMATION_CONFIG_FILE: &str = "config/automation.toml";
/// Optional Cargo executable override shared with platform adapters.
pub const CARGO_COMMAND_ENV: &str = "NEUTRAL_CARGO_COMMAND";
/// Optional Rust compiler executable override shared with platform adapters.
pub const RUSTC_COMMAND_ENV: &str = "NEUTRAL_RUSTC_COMMAND";
/// Optional generated-results root override.
pub const TEST_RESULTS_ENV: &str = "NEUTRAL_TEST_RESULTS";
/// Active quality-gate configuration relative to the workspace root.
pub const QUALITY_GATES_FILE: &str = "config/quality-gates.toml";
/// Durable quality-document inventory relative to the workspace root.
pub const QUALITY_MANIFEST_FILE: &str = "quality/manifest.toml";
/// Generated human-readable quality status relative to the workspace root.
pub const QUALITY_STATUS_FILE: &str = "quality/STATUS.md";
/// Versioned durable quality evidence root relative to the workspace root.
pub const QUALITY_EVIDENCE_DIRECTORY: &str = "quality/evidence";
/// Ignored commit-bound quality evaluations relative to the result root.
pub const QUALITY_EVALUATION_DIRECTORY: &str = "quality/evaluations";
/// Release-selection configuration relative to the workspace root.
pub const RELEASE_CONFIG_FILE: &str = "config/release.toml";
/// Approved residual-risk record relative to the workspace root.
pub const RESIDUAL_RISKS_FILE: &str = "quality/reviews/residual-risks.md";
/// Root workspace manifest relative to the workspace root.
pub const WORKSPACE_MANIFEST_FILE: &str = "Cargo.toml";
/// Root dependency lock relative to the workspace root.
pub const CARGO_LOCK_FILE: &str = "Cargo.lock";
/// Declared dependency-source policy relative to the workspace root.
pub const DEPENDENCY_SOURCES_FILE: &str = "config/dependency-sources.toml";
/// Generated-output ownership inventory relative to the workspace root.
pub const GENERATED_OUTPUTS_FILE: &str = "config/generated-outputs.toml";
/// Repository-directory ownership inventory relative to the workspace root.
pub const REPOSITORY_LAYOUT_FILE: &str = "config/repository-layout.toml";
/// Durable test-level ownership inventory relative to the workspace root.
pub const TEST_LEVELS_FILE: &str = "config/test-levels.toml";
/// Active test-discovery minima relative to the workspace root.
pub const TEST_SUITES_FILE: &str = "config/test-suites.toml";
/// Durable test-minimum profile used by normal commands.
pub const CURRENT_TEST_PROFILE: &str = "current";
/// Repository license file included in binary distributions.
pub const LICENSE_FILE: &str = "LICENSE";
/// Repository overview included in binary distributions.
pub const ROOT_README_FILE: &str = "README.md";
/// Generated distribution manifest filename.
pub const RELEASE_MANIFEST_FILE: &str = "release-manifest.json";
/// Generated distribution checksums filename.
pub const RELEASE_CHECKSUM_FILE: &str = "SHA256SUMS";
/// Generated installation guide filename.
pub const RELEASE_INSTALL_FILE: &str = "INSTALL.md";
/// Exact locked dependency inventory shipped as the v0 SBOM.
pub const RELEASE_SBOM_FILE: &str = "SBOM-Cargo.lock";
/// Generated build-provenance filename.
pub const RELEASE_PROVENANCE_FILE: &str = "provenance.json";
/// Active portable root relative to the workspace root.
pub const PORTABLE_DIRECTORY: &str = "portable";
/// Active portable plan relative to the workspace root.
pub const PORTABLE_PLAN_FILE: &str = "portable/PLAN.md";
/// Active portable lifecycle identity relative to the workspace root.
pub const PORTABLE_LIFECYCLE_FILE: &str = "portable/lifecycle.toml";
/// Active portable requirements relative to the workspace root.
pub const PORTABLE_REQUIREMENTS_FILE: &str = "portable/specs/REQUIREMENTS.md";
/// Active portable contract directory relative to the workspace root.
pub const PORTABLE_CONTRACT_DIRECTORY: &str = "portable/specs/contracts";
/// Active portable traceability matrix relative to the workspace root.
pub const PORTABLE_TRACEABILITY_FILE: &str = "portable/specs/TRACEABILITY.md";
/// Active portable contract-freeze manifest relative to the workspace root.
pub const PORTABLE_CONTRACT_FREEZE_FILE: &str = "portable/specs/contracts/freeze.toml";
/// Active portable fixture root relative to the workspace root.
pub const PORTABLE_FIXTURE_DIRECTORY: &str = "portable/specs/fixtures";
/// Active portable oracle root relative to the workspace root.
pub const PORTABLE_ORACLE_DIRECTORY: &str = "portable/conformance/oracles";
/// Active portable conformance manifest relative to the workspace root.
pub const PORTABLE_CONFORMANCE_MANIFEST_FILE: &str = "portable/conformance/manifest.toml";
/// Rust marker that identifies a test-only source declaration.
pub const TEST_CONFIGURATION_MARKER: &str = "#[cfg(test)]";
/// Rust marker proving a test module body is stored outside production source.
pub const TEST_PATH_ATTRIBUTE_MARKER: &str = "#[path =";
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
/// Relative root for generated release preparation and packages.
pub const RELEASE_RESULT_DIRECTORY: &str = "release";
/// Binary package subtree beneath the generated release root.
pub const RELEASE_PACKAGE_DIRECTORY: &str = "package";
/// Generated package summary filename.
pub const RELEASE_PACKAGE_SUMMARY_FILE: &str = "package-summary.json";
/// Generated aggregate workflow-log directory relative to the result root.
pub const WORKFLOW_RESULT_DIRECTORY: &str = "workflows";
/// Append-only event-log filename within one workflow run.
pub const WORKFLOW_EVENTS_FILE: &str = "events.jsonl";
/// Current workflow-summary filename within one workflow run.
pub const WORKFLOW_SUMMARY_FILE: &str = "summary.json";
/// Generated version-plan directory relative to the result root.
pub const VERSION_RESULT_DIRECTORY: &str = "version";
/// Generated portable snapshot root relative to the result root.
pub const PORTABLE_SNAPSHOT_DIRECTORY: &str = "portable/snapshot";
