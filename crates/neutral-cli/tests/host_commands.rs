// SPDX-License-Identifier: Apache-2.0

//! Black-box system tests for explicit CLI host effects and exit policy.

use neutral_cli::constants;
use neutral_core::{CancellationToken, VocabularyContentDigest};
use neutral_encoding::{DecodeLimits, decode};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    sync::atomic::{AtomicU64, Ordering},
};

/// Process-local suffix for isolated system-test roots.
static TEST_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(0);
/// Minimal valid source used at the command boundary.
const MINIMAL_SOURCE: &[u8] = b"neu \"0.1\"\nmodule cli_system\nnum answer = 42\n";
/// Invalid source used to prove fail-closed output policy.
const INVALID_SOURCE: &[u8] = b"neu \"0.1\"\nnum answer = 42\n";
/// Expected canonical rendering of the noncanonical formatter source.
const FORMATTED_SOURCE: &[u8] = b"neu \"0.1\"\nmodule cli_system\n\nnum answer = 42\n";

/// Isolated temporary root removed after one system test.
struct TestRoot(PathBuf);

impl TestRoot {
    /// Creates one process-unique isolated test directory.
    fn new(label: &str) -> Self {
        let sequence = TEST_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "neutral-cli-{label}-{}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("isolated CLI test root must be creatable");
        Self(path)
    }

    /// Resolves one child path inside the isolated root.
    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    /// Returns the isolated root path.
    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestRoot {
    /// Removes only the process-unique directory created by this test.
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// Creates a command for the exact Cargo-built CLI executable.
fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_neutral-cli"))
}

/// Runs the CLI with exact UTF-8 arguments and captures both streams.
fn run(arguments: &[&str]) -> Output {
    cli().args(arguments).output().expect("CLI must execute")
}

/// Converts captured standard error to asserted UTF-8.
fn stderr(output: &Output) -> &str {
    std::str::from_utf8(&output.stderr).expect("CLI stderr must be UTF-8")
}

/// Returns whether an isolated root contains an atomic temporary file.
fn has_temporary_file(root: &TestRoot) -> bool {
    fs::read_dir(root.path())
        .expect("test root must remain readable")
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(constants::TEMPORARY_MARKER)
        })
}

#[test]
/// Verifies general and command-specific usage through the built executable.
fn system_cli_usage_and_command_help_are_stable() {
    let missing = run(&[]);
    assert_eq!(
        missing.status.code(),
        Some(i32::from(constants::EXIT_USAGE))
    );
    assert!(stderr(&missing).contains(constants::USAGE));

    for (command, usage) in [
        (constants::COMPILE, constants::COMPILE_USAGE),
        (constants::VALIDATE, constants::VALIDATE_USAGE),
        (constants::FORMAT, constants::FORMAT_USAGE),
    ] {
        let help = run(&[command, constants::HELP]);
        assert_eq!(help.status.code(), Some(0), "{}", stderr(&help));
        assert!(stderr(&help).contains(usage));
        assert!(help.stdout.is_empty());
    }
}

#[test]
/// Exercises compile, validate, format, overwrite, and decoded artifact output.
fn system_cli_file_commands_have_stable_output_policy() {
    let root = TestRoot::new("files");
    let source = root.join("source file λ.neu");
    let artifact = root.join("artifact file λ.nir");
    let formatted = root.join("formatted file λ.neu");
    fs::write(&source, MINIMAL_SOURCE).expect("source fixture must be writable");

    let compile = run(&[
        constants::COMPILE,
        constants::OUTPUT,
        artifact.to_str().expect("test path must be UTF-8"),
        source.to_str().expect("test path must be UTF-8"),
    ]);
    assert_eq!(compile.status.code(), Some(0), "{}", stderr(&compile));
    assert!(stderr(&compile).contains("[info] compile succeeded"));
    let decoded = decode(
        &fs::read(&artifact).expect("artifact must exist"),
        DecodeLimits::hard(),
        &CancellationToken::new(),
    )
    .expect("CLI artifact must decode");
    assert_eq!(decoded.module_name(), "cli_system");

    let validate = run(&[
        constants::VALIDATE,
        source.to_str().expect("test path must be UTF-8"),
    ]);
    assert_eq!(validate.status.code(), Some(0));
    assert!(stderr(&validate).contains("[info] validation succeeded"));

    let format = run(&[
        constants::FORMAT,
        constants::OUTPUT,
        formatted.to_str().expect("test path must be UTF-8"),
        source.to_str().expect("test path must be UTF-8"),
    ]);
    assert_eq!(format.status.code(), Some(0), "{}", stderr(&format));
    assert_eq!(
        fs::read(&formatted).expect("formatted output must exist"),
        FORMATTED_SOURCE
    );

    let no_overwrite = run(&[
        constants::FORMAT,
        constants::OUTPUT,
        formatted.to_str().expect("test path must be UTF-8"),
        source.to_str().expect("test path must be UTF-8"),
    ]);
    assert_eq!(
        no_overwrite.status.code(),
        Some(i32::from(constants::EXIT_OUTPUT))
    );
    assert!(stderr(&no_overwrite).contains("[error] output-exists"));
    assert_eq!(
        fs::read(&formatted).expect("existing output must remain"),
        FORMATTED_SOURCE
    );

    fs::write(&formatted, b"replace me").expect("replacement sentinel must be writable");
    let overwrite = run(&[
        constants::FORMAT,
        constants::OVERWRITE,
        constants::OUTPUT,
        formatted.to_str().expect("test path must be UTF-8"),
        source.to_str().expect("test path must be UTF-8"),
    ]);
    assert_eq!(overwrite.status.code(), Some(0), "{}", stderr(&overwrite));
    assert_eq!(
        fs::read(&formatted).expect("replacement output must exist"),
        FORMATTED_SOURCE
    );
    assert!(!has_temporary_file(&root));
}

#[test]
/// Exercises source standard input and binary artifact standard output.
fn system_cli_compile_supports_explicit_standard_streams() {
    let mut child = cli()
        .args([
            constants::COMPILE,
            constants::OUTPUT,
            constants::STANDARD_STREAM,
            constants::STANDARD_STREAM,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("streaming CLI must spawn");
    child
        .stdin
        .take()
        .expect("child stdin must be piped")
        .write_all(MINIMAL_SOURCE)
        .expect("source must reach child stdin");
    let output = child.wait_with_output().expect("streaming CLI must finish");
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(stderr(&output).contains("[info] compile succeeded"));
    let decoded = decode(
        &output.stdout,
        DecodeLimits::hard(),
        &CancellationToken::new(),
    )
    .expect("stdout must contain only the encoded artifact");
    assert_eq!(decoded.module_name(), "cli_system");
}

#[test]
/// Proves validation failure and cancellation never replace an existing output.
fn system_cli_failure_and_cancellation_leave_no_partial_output() {
    let root = TestRoot::new("fail-closed");
    let source = root.join("invalid.neu");
    let destination = root.join("protected.nir");
    let sentinel = b"existing authoritative bytes";
    fs::write(&source, INVALID_SOURCE).expect("invalid source must be writable");
    fs::write(&destination, sentinel).expect("sentinel output must be writable");

    let invalid = run(&[
        constants::COMPILE,
        constants::OVERWRITE,
        constants::OUTPUT,
        destination.to_str().expect("test path must be UTF-8"),
        source.to_str().expect("test path must be UTF-8"),
    ]);
    assert_eq!(
        invalid.status.code(),
        Some(i32::from(constants::EXIT_VALIDATION))
    );
    assert_eq!(
        fs::read(&destination).expect("sentinel must remain"),
        sentinel
    );

    let limited = run(&[
        constants::COMPILE,
        constants::MAX_SOURCE_BYTES,
        "1",
        constants::OVERWRITE,
        constants::OUTPUT,
        destination.to_str().expect("test path must be UTF-8"),
        source.to_str().expect("test path must be UTF-8"),
    ]);
    assert_eq!(
        limited.status.code(),
        Some(i32::from(constants::EXIT_VALIDATION))
    );
    assert!(stderr(&limited).contains("[error] input-limit-exceeded"));
    assert_eq!(
        fs::read(&destination).expect("sentinel must remain after limit failure"),
        sentinel
    );

    fs::write(&source, MINIMAL_SOURCE).expect("valid source must be writable");
    let cancelled = run(&[
        constants::COMPILE,
        constants::CANCEL_BEFORE_START,
        constants::OVERWRITE,
        constants::OUTPUT,
        destination.to_str().expect("test path must be UTF-8"),
        source.to_str().expect("test path must be UTF-8"),
    ]);
    assert_eq!(
        cancelled.status.code(),
        Some(i32::from(constants::EXIT_CANCELLED))
    );
    assert!(stderr(&cancelled).contains("[error] cancelled"));
    assert_eq!(
        fs::read(&destination).expect("sentinel must remain"),
        sentinel
    );
    assert!(!has_temporary_file(&root));
}

#[test]
/// Verifies missing inputs and output failures do not disclose supplied host paths.
fn system_cli_host_failures_are_path_safe_and_cleanup_atomic_temporary_files() {
    let root = TestRoot::new("disclosure");
    let secret_path = root.join("credential=do-not-print.neu");
    let missing = run(&[
        constants::VALIDATE,
        secret_path.to_str().expect("test path must be UTF-8"),
    ]);
    assert_eq!(
        missing.status.code(),
        Some(i32::from(constants::EXIT_INPUT))
    );
    assert!(!stderr(&missing).contains("credential=do-not-print"));
    assert!(stderr(&missing).contains("[error] input-read-failed"));

    let source = root.join("source.neu");
    fs::write(&source, MINIMAL_SOURCE).expect("source must be writable");
    let commit_failure = run(&[
        constants::FORMAT,
        constants::OVERWRITE,
        constants::OUTPUT,
        root.path().to_str().expect("test path must be UTF-8"),
        source.to_str().expect("test path must be UTF-8"),
    ]);
    assert_eq!(
        commit_failure.status.code(),
        Some(i32::from(constants::EXIT_OUTPUT))
    );
    assert!(!has_temporary_file(&root));
}

#[test]
/// Exercises exact host-supplied vocabulary bundle and lock acquisition.
fn system_cli_vocabulary_resolution_is_explicit() {
    let root = TestRoot::new("vocabulary");
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("CLI crate must be inside the workspace");
    let source =
        workspace.join("portable/spec/v0/fixtures/positive/vocabulary/minimal-vocabulary.neu");
    let bundle =
        workspace.join("portable/spec/v0/fixtures/vocabulary/bundles/positive/comprehensive.json");
    let bundle_bytes = fs::read(&bundle).expect("vocabulary bundle must be readable");
    let digest = VocabularyContentDigest::from_bytes(&bundle_bytes).to_string();
    let destination = root.join("vocabulary.nir");
    let output = run(&[
        constants::COMPILE,
        constants::OUTPUT,
        destination.to_str().expect("test path must be UTF-8"),
        constants::VOCABULARY_BUNDLE,
        bundle.to_str().expect("test path must be UTF-8"),
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
        source.to_str().expect("test path must be UTF-8"),
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(destination.is_file());
}

#[cfg(unix)]
#[test]
/// Verifies an unwritable destination returns the output class without partial bytes.
fn system_cli_permission_failure_leaves_no_output() {
    use std::os::unix::fs::PermissionsExt as _;

    let root = TestRoot::new("permission");
    let source = root.join("source.neu");
    let restricted = root.join("restricted");
    let destination = restricted.join("formatted.neu");
    fs::write(&source, MINIMAL_SOURCE).expect("source must be writable");
    fs::create_dir(&restricted).expect("restricted directory must be creatable");
    fs::set_permissions(&restricted, fs::Permissions::from_mode(0o500))
        .expect("test permissions must be configurable");

    let output = run(&[
        constants::FORMAT,
        constants::OUTPUT,
        destination.to_str().expect("test path must be UTF-8"),
        source.to_str().expect("test path must be UTF-8"),
    ]);
    fs::set_permissions(&restricted, fs::Permissions::from_mode(0o700))
        .expect("test permissions must be restorable");
    assert_eq!(
        output.status.code(),
        Some(i32::from(constants::EXIT_OUTPUT))
    );
    assert!(!destination.exists());
    assert!(!has_temporary_file(&root));
}

#[test]
/// Verifies a closed standard-output consumer produces the stable output failure.
fn system_cli_broken_pipe_has_stable_exit_class() {
    let root = TestRoot::new("broken-pipe");
    let source = root.join("large.neu");
    let payload = "x".repeat(262_144);
    fs::write(
        &source,
        format!("neu \"0.1\"\nmodule broken_pipe\nstring payload = \"{payload}\"\n"),
    )
    .expect("large valid source must be writable");
    let mut child = cli()
        .args([
            constants::FORMAT,
            constants::OUTPUT,
            constants::STANDARD_STREAM,
            source.to_str().expect("test path must be UTF-8"),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("broken-pipe CLI must spawn");
    drop(child.stdout.take());
    let output = child
        .wait_with_output()
        .expect("broken-pipe CLI must finish");
    assert_eq!(
        output.status.code(),
        Some(i32::from(constants::EXIT_OUTPUT)),
        "{}",
        stderr(&output)
    );
    assert!(stderr(&output).contains("[error] stdout-write-failed"));
}
