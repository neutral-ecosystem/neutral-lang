// SPDX-License-Identifier: Apache-2.0

//! Explicit bounded host adapters for CLI input, output, and compilation.

use crate::{
    command::{CommandKind, CommandOptions, ParseOutcome, VocabularyOptions, parse},
    constants,
};
use neutral_compiler::{
    CaptureError, CompilationFailure, CompilationRequest, CompilationResult, FormatError, compile,
    format as format_source,
};
use neutral_core::{CancellationToken, ResultClass, VocabularyContentDigest};
use neutral_encoding::{ProducerInfo, encode};
use neutral_reader::ValidatedDocument;
use neutral_vocabulary::VocabularyLock;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

/// Monotonic process-local suffix for atomic temporary outputs.
static TEMPORARY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Stable process outcome classes exposed by the command-line boundary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExitClass {
    /// Invalid command syntax or option policy.
    Usage,
    /// Source or vocabulary input could not be acquired.
    Input,
    /// Captured content was rejected.
    Validation,
    /// Output could not be safely committed.
    Output,
    /// Work was cooperatively cancelled.
    Cancelled,
    /// An internal invariant failed.
    Internal,
}

impl ExitClass {
    /// Returns the stable operating-system process exit code.
    #[must_use]
    pub const fn code(self) -> u8 {
        match self {
            Self::Usage => constants::EXIT_USAGE,
            Self::Input => constants::EXIT_INPUT,
            Self::Validation => constants::EXIT_VALIDATION,
            Self::Output => constants::EXIT_OUTPUT,
            Self::Cancelled => constants::EXIT_CANCELLED,
            Self::Internal => constants::EXIT_INTERNAL,
        }
    }
}

/// Safe categorized command failure with no host path or source disclosure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CliFailure {
    /// Stable exit class.
    class: ExitClass,
    /// Safe bounded output lines without category prefixes.
    messages: Vec<String>,
}

impl CliFailure {
    /// Creates one safe single-line command failure.
    fn one(class: ExitClass, message: impl Into<String>) -> Self {
        Self {
            class,
            messages: vec![message.into()],
        }
    }

    /// Returns the stable failure exit class.
    #[must_use]
    pub const fn class(&self) -> ExitClass {
        self.class
    }

    /// Returns safe bounded failure lines.
    #[must_use]
    pub fn messages(&self) -> &[String] {
        &self.messages
    }
}

/// Parses and executes one command using only explicitly selected host resources.
///
/// # Errors
///
/// Returns a categorized, path-safe failure when usage is invalid, acquisition
/// or validation fails, output cannot be committed, work is cancelled, or an
/// internal invariant is violated.
pub fn execute(arguments: impl IntoIterator<Item = String>) -> Result<(), CliFailure> {
    match parse(arguments).map_err(|message| CliFailure::one(ExitClass::Usage, message))? {
        ParseOutcome::Help(usage) => {
            eprintln!("{} usage: {usage}", constants::INFO);
            Ok(())
        }
        ParseOutcome::Version => {
            eprintln!(
                "{} {} {}",
                constants::INFO,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        }
        ParseOutcome::Execute(options) => execute_command(&options),
    }
}

/// Executes one validated command policy.
fn execute_command(options: &CommandOptions) -> Result<(), CliFailure> {
    let request = request_from_options(options)?;
    match options.kind {
        CommandKind::Compile => compile_command(request, options),
        CommandKind::Validate => validate_command(request),
        CommandKind::Format => format_command(request, options),
    }
}

/// Compiles source and atomically publishes one encoded artifact.
fn compile_command(
    request: CompilationRequest,
    options: &CommandOptions,
) -> Result<(), CliFailure> {
    let document = successful_document(compile(request).map_err(|error| capture_failure(&error))?)?;
    let encoded = encode(
        &document,
        &ProducerInfo::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")),
    )
    .map_err(|_| CliFailure::one(ExitClass::Validation, "artifact-encoding-limit"))?;
    write_selected_output(options, encoded.as_bytes())?;
    eprintln!("{} compile succeeded", constants::INFO);
    Ok(())
}

/// Validates source without publishing authoritative output.
fn validate_command(request: CompilationRequest) -> Result<(), CliFailure> {
    ensure_success(compile(request).map_err(|error| capture_failure(&error))?)?;
    eprintln!("{} validation succeeded", constants::INFO);
    Ok(())
}

/// Formats validated source and atomically publishes ordinary source bytes.
fn format_command(request: CompilationRequest, options: &CommandOptions) -> Result<(), CliFailure> {
    let formatted = format_source(request).map_err(format_failure)?;
    write_selected_output(options, formatted.as_bytes())?;
    eprintln!("{} format succeeded", constants::INFO);
    Ok(())
}

/// Builds one captured-input request from explicit source, limits, and vocabulary facts.
fn request_from_options(options: &CommandOptions) -> Result<CompilationRequest, CliFailure> {
    let source = read_selected_input(&options.input, options.limits.source_bytes())?;
    let cancellation = CancellationToken::new();
    if options.cancel_before_start {
        cancellation.cancel();
    }
    let request = CompilationRequest::new(source, options.limits, cancellation);
    captured_vocabulary(request, &options.vocabulary, options.limits.source_bytes())
}

/// Attaches an exact explicitly acquired vocabulary bundle and immutable lock.
fn captured_vocabulary(
    request: CompilationRequest,
    options: &VocabularyOptions,
    byte_limit: u64,
) -> Result<CompilationRequest, CliFailure> {
    if options.is_empty() {
        return Ok(request);
    }
    let bundle_path = required_option(options.bundle.as_ref())?;
    let bytes = read_selected_input(bundle_path, byte_limit)?;
    let digest = VocabularyContentDigest::parse_text(required_option(options.digest.as_ref())?)
        .map_err(|_| CliFailure::one(ExitClass::Usage, "invalid-vocabulary-digest"))?;
    let lock = VocabularyLock::new(
        required_option(options.identity.as_ref())?,
        required_option(options.version.as_ref())?,
        required_option(options.encoding_version.as_ref())?,
        required_option(options.schema_version.as_ref())?,
        digest,
        options.features.clone(),
    )
    .map_err(|_| CliFailure::one(ExitClass::Usage, "invalid-vocabulary-lock"))?;
    Ok(request.with_captured_vocabulary(bytes, lock))
}

/// Borrows one parser-validated required option or reports an internal defect.
fn required_option(value: Option<&String>) -> Result<&str, CliFailure> {
    value
        .map(String::as_str)
        .ok_or_else(|| CliFailure::one(ExitClass::Internal, "missing-validated-option"))
}

/// Reads a file or standard input without exceeding the selected byte ceiling.
fn read_selected_input(selection: &str, byte_limit: u64) -> Result<Vec<u8>, CliFailure> {
    let reader: Box<dyn Read> = if selection == constants::STANDARD_STREAM {
        Box::new(io::stdin())
    } else {
        Box::new(
            File::open(selection)
                .map_err(|_| CliFailure::one(ExitClass::Input, "input-read-failed"))?,
        )
    };
    read_bounded(reader, byte_limit)
}

/// Reads at most one byte beyond a declared limit to classify oversized input.
fn read_bounded(reader: Box<dyn Read>, byte_limit: u64) -> Result<Vec<u8>, CliFailure> {
    let maximum_read = byte_limit.saturating_add(1);
    let mut bytes = Vec::new();
    reader
        .take(maximum_read)
        .read_to_end(&mut bytes)
        .map_err(|_| CliFailure::one(ExitClass::Input, "input-read-failed"))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > byte_limit {
        Err(CliFailure::one(
            ExitClass::Validation,
            "input-limit-exceeded",
        ))
    } else {
        Ok(bytes)
    }
}

/// Opens authoritative artifacts as a validated document or maps their failure.
fn successful_document(result: CompilationResult) -> Result<ValidatedDocument, CliFailure> {
    match result {
        CompilationResult::Success(artifacts) => ValidatedDocument::from_compiler_output(artifacts)
            .map_err(|_| CliFailure::one(ExitClass::Internal, "reader-validation-defect")),
        CompilationResult::Failure(failure) => Err(compilation_failure(&failure)),
    }
}

/// Requires successful compilation when a command publishes no artifact.
fn ensure_success(result: CompilationResult) -> Result<(), CliFailure> {
    match result {
        CompilationResult::Success(_) => Ok(()),
        CompilationResult::Failure(failure) => Err(compilation_failure(&failure)),
    }
}

/// Maps one capture failure to its stable CLI class without path disclosure.
fn capture_failure(error: &CaptureError) -> CliFailure {
    match error {
        CaptureError::Cancelled => CliFailure::one(ExitClass::Cancelled, "cancelled"),
        CaptureError::MissingSource => CliFailure::one(ExitClass::Validation, "missing-source"),
        CaptureError::SourceLimitExceeded { .. } => {
            CliFailure::one(ExitClass::Validation, "source-limit-exceeded")
        }
        CaptureError::SourceDigestMismatch { .. } => {
            CliFailure::one(ExitClass::Validation, "source-digest-mismatch")
        }
    }
}

/// Maps a compilation failure to safe diagnostic code and byte-span lines.
fn compilation_failure(failure: &CompilationFailure) -> CliFailure {
    let class = if failure.class() == ResultClass::Cancellation {
        ExitClass::Cancelled
    } else {
        ExitClass::Validation
    };
    let messages = if failure.diagnostics().is_empty() {
        vec![if class == ExitClass::Cancelled {
            "cancelled".to_owned()
        } else {
            "source-rejected".to_owned()
        }]
    } else {
        failure
            .diagnostics()
            .iter()
            .map(|diagnostic| {
                let span = diagnostic.primary().span();
                format!(
                    "{} bytes {}..{}",
                    diagnostic.code().as_str(),
                    span.start(),
                    span.end()
                )
            })
            .collect()
    };
    CliFailure { class, messages }
}

/// Maps reference formatter failure classes to stable CLI outcomes.
fn format_failure(error: FormatError) -> CliFailure {
    match error {
        FormatError::Capture(error) => capture_failure(&error),
        FormatError::Compilation(failure) => compilation_failure(&failure),
        FormatError::OutputLimitExceeded => {
            CliFailure::one(ExitClass::Validation, "formatted-output-limit-exceeded")
        }
        FormatError::InternalDefect => {
            CliFailure::one(ExitClass::Internal, "formatter-internal-defect")
        }
    }
}

/// Publishes bytes to the explicitly selected standard stream or file destination.
fn write_selected_output(options: &CommandOptions, bytes: &[u8]) -> Result<(), CliFailure> {
    let destination = options
        .output
        .as_deref()
        .ok_or_else(|| CliFailure::one(ExitClass::Internal, "missing-validated-output"))?;
    if destination == constants::STANDARD_STREAM {
        let mut stdout = io::stdout().lock();
        stdout
            .write_all(bytes)
            .and_then(|()| stdout.flush())
            .map_err(|_| CliFailure::one(ExitClass::Output, "stdout-write-failed"))
    } else {
        atomic_write(Path::new(destination), bytes, options.overwrite)
    }
}

/// Commits complete output through a same-directory temporary file.
fn atomic_write(destination: &Path, bytes: &[u8], overwrite: bool) -> Result<(), CliFailure> {
    if !overwrite && destination.exists() {
        return Err(CliFailure::one(ExitClass::Output, "output-exists"));
    }
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    let (temporary_path, mut temporary_file) = create_temporary(parent)?;
    let guard = TemporaryOutput::new(temporary_path.clone());
    temporary_file
        .write_all(bytes)
        .and_then(|()| temporary_file.sync_all())
        .map_err(|_| CliFailure::one(ExitClass::Output, "output-write-failed"))?;
    drop(temporary_file);
    if overwrite {
        fs::rename(&temporary_path, destination)
    } else {
        fs::hard_link(&temporary_path, destination).and_then(|()| fs::remove_file(&temporary_path))
    }
    .map_err(|_| CliFailure::one(ExitClass::Output, "output-commit-failed"))?;
    guard.commit();
    Ok(())
}

/// Creates a collision-resistant temporary output in the destination directory.
fn create_temporary(parent: &Path) -> Result<(PathBuf, File), CliFailure> {
    for _ in 0..constants::MAXIMUM_TEMPORARY_ATTEMPTS {
        let sequence = TEMPORARY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let name = format!(
            "{}{}-{sequence}",
            constants::TEMPORARY_MARKER,
            std::process::id()
        );
        let path = parent.join(name);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(_) => return Err(CliFailure::one(ExitClass::Output, "output-create-failed")),
        }
    }
    Err(CliFailure::one(
        ExitClass::Output,
        "temporary-name-exhausted",
    ))
}

/// Cleanup guard for an uncommitted atomic temporary output.
struct TemporaryOutput {
    /// Exact same-directory temporary path.
    path: PathBuf,
    /// Whether ownership was transferred to the final destination.
    committed: std::cell::Cell<bool>,
}

impl TemporaryOutput {
    /// Creates an armed cleanup guard.
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            committed: std::cell::Cell::new(false),
        }
    }

    /// Marks the temporary path as successfully committed.
    fn commit(&self) {
        self.committed.set(true);
    }
}

impl Drop for TemporaryOutput {
    /// Removes an uncommitted temporary file on every return path.
    fn drop(&mut self) {
        if !self.committed.get() {
            let _ = fs::remove_file(&self.path);
        }
    }
}
