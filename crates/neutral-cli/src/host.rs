// SPDX-License-Identifier: Apache-2.0

//! Explicit bounded host adapters for CLI input, output, and compilation.

use crate::{
    command::{CommandKind, CommandOptions, ParseOutcome, VocabularyOptions, parse},
    constants,
};
use neutral_compiler::{
    CapturedExpressionProject, CapturedExpressionProjectRequest, CapturedProjectRequest,
    CapturedSourceInput, CapturedVocabularyInput, ExpressionCaptureFailure,
    ExpressionCompileFailure, ProjectCaptureControls, ProjectCaptureLimitValues,
    ProjectCaptureLimits, capture_expression_project, compile_expression_project,
    format_expression_source, source_module_name,
};
use neutral_core::{CancellationToken, VocabularyContentDigest, profile::LanguageProfile};
use neutral_encoding::encode_expression_project;
use neutral_reader::{SemanticLimits, ValidatedExpressionProject};
use neutral_vocabulary::{VocabularyLimits, VocabularyLock, composition::CompositionLimits};
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
            eprintln!("Usage: {usage}");
            Ok(())
        }
        ParseOutcome::Version => {
            eprintln!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        ParseOutcome::Profiles => {
            report_profiles();
            Ok(())
        }
        ParseOutcome::Execute(options) => execute_command(&options),
    }
}

/// Reports the reader-visible profile catalogue in deterministic text form.
fn report_profiles() {
    eprint!(
        "{} profile={} status=development capabilities=",
        constants::INFO,
        neutral_core::profile::V1_SOURCE_PROFILE,
    );
    for (index, feature) in neutral_reader::expression::profile::REQUIRED_FEATURES
        .iter()
        .enumerate()
    {
        if index != 0 {
            eprint!(",");
        }
        eprint!("{feature}");
    }
    eprintln!();
}

/// Executes one validated command policy.
fn execute_command(options: &CommandOptions) -> Result<(), CliFailure> {
    if options.kind == CommandKind::CaptureProject {
        return capture_project_command(options);
    }
    let request = request_from_options(options)?;
    match options.kind {
        CommandKind::Compile => compile_command(&request, options),
        CommandKind::Validate => validate_command(&request),
        CommandKind::Format => format_command(&request, options),
        CommandKind::CaptureProject => unreachable!("project capture returned before compilation"),
    }
}

/// Acquires explicit inputs, constructs the shared request schema, and captures it.
fn capture_project_command(options: &CommandOptions) -> Result<(), CliFailure> {
    let captured = request_from_options(options)?;
    let facts = captured.resource_facts();
    eprintln!(
        "{} capture succeeded source-units={} source-bytes={} vocabulary-units={} vocabulary-bytes={}",
        constants::PASS,
        facts.source_units(),
        facts.total_source_bytes(),
        facts.vocabulary_units(),
        facts.total_vocabulary_bytes()
    );
    Ok(())
}

/// Converts the existing explicit CLI limits into the complete project schema.
fn project_limits(options: &CommandOptions) -> ProjectCaptureLimits {
    let limits = options.limits;
    ProjectCaptureLimits::new(ProjectCaptureLimitValues {
        total_source_bytes: limits.source_bytes(),
        source_bytes_per_unit: limits.source_bytes(),
        source_units: 1,
        source_id_bytes: limits.source_bytes(),
        module_id_bytes: limits.source_bytes(),
        vocabulary_units: 1,
        vocabulary_bytes_per_unit: limits.source_bytes(),
        total_vocabulary_bytes: limits.source_bytes(),
        imports_per_module: limits.declarations(),
        import_edges: limits.declarations(),
        scc_units: 1,
        declarations: limits.declarations(),
        diagnostics: u64::from(limits.diagnostics()),
        output_bytes: neutral_encoding::constants::MAXIMUM_ARTIFACT_BYTES as u64,
    })
}

/// Compiles source and atomically publishes one encoded artifact.
fn compile_command(
    request: &CapturedExpressionProject,
    options: &CommandOptions,
) -> Result<(), CliFailure> {
    let cancel = CancellationToken::new();
    let document = successful_project(request, &cancel)?;
    let encoded = encode_expression_project(&document, &cancel)
        .map_err(|_| CliFailure::one(ExitClass::Validation, "artifact-encoding-limit"))?;
    write_selected_output(options, &encoded)?;
    eprintln!("{} compile succeeded", constants::PASS);
    Ok(())
}

/// Validates source without publishing authoritative output.
fn validate_command(request: &CapturedExpressionProject) -> Result<(), CliFailure> {
    successful_project(request, &CancellationToken::new())?;
    eprintln!("{} validation succeeded", constants::PASS);
    Ok(())
}

/// Formats validated source and atomically publishes ordinary source bytes.
fn format_command(
    request: &CapturedExpressionProject,
    options: &CommandOptions,
) -> Result<(), CliFailure> {
    let formatted = format_expression_source(
        request,
        request.sources()[0].module_id(),
        &CancellationToken::new(),
    )
    .map_err(|error| compilation_failure(&error))?;
    if formatted.len() as u64 > options.limits.source_bytes() {
        return Err(CliFailure::one(
            ExitClass::Validation,
            "formatted-output-limit-exceeded",
        ));
    }
    write_selected_output(options, &formatted)?;
    eprintln!("{} format succeeded", constants::PASS);
    Ok(())
}

/// Builds one captured-input request from explicit source, limits, and vocabulary facts.
fn request_from_options(options: &CommandOptions) -> Result<CapturedExpressionProject, CliFailure> {
    let source = read_selected_input(&options.input, options.limits.source_bytes())?;
    let cancellation = CancellationToken::new();
    if options.cancel_before_start {
        cancellation.cancel();
    }
    let module = source_module_name(&source, project_limits(options), &cancellation)
        .map_err(|error| capture_failure(&error))?;
    if options
        .module_id
        .as_ref()
        .is_some_and(|expected| expected != &module)
    {
        return Err(CliFailure::one(
            ExitClass::Validation,
            "module-header-mismatch",
        ));
    }
    let vocabulary = project_vocabulary(&options.vocabulary, options.limits.source_bytes())?;
    let mut envelope = CapturedProjectRequest::new(
        neutral_reader::expression::profile::CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        vec![CapturedSourceInput::new(
            options.source_id.as_deref().unwrap_or("source:cli"),
            module,
            source,
        )],
        vocabulary.into_iter().collect(),
        ProjectCaptureControls::new(project_limits(options), cancellation),
    );
    if let Some(key) = &options.project_key {
        envelope = envelope.with_project_key(key.clone());
    }
    let hard = neutral_reader::expression::profile::ExpressionLimits::hard();
    let expressions = neutral_reader::expression::profile::ExpressionLimits {
        nodes: hard.nodes.min(options.limits.traversal_nodes()),
        syntax_depth: hard.syntax_depth.min(options.limits.nesting_depth()),
        evaluation_depth: hard.evaluation_depth.min(options.limits.nesting_depth()),
        bindings: hard.bindings.min(options.limits.declarations()),
        coefficient_digits: hard.coefficient_digits.min(options.limits.numeric_digits()),
        absolute_scale: hard.absolute_scale.min(options.limits.numeric_scale()),
        ..hard
    };
    capture_expression_project(CapturedExpressionProjectRequest::new(
        envelope,
        neutral_reader::expression::profile::SemanticProfiles::try_frozen()
            .map_err(|_| CliFailure::one(ExitClass::Validation, "capture-limit-exceeded"))?,
        CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(options.limits)),
        expressions,
        neutral_reader::expression::ExpansionLimits::hard(),
    ))
    .map_err(|error| capture_failure(&error))
}

/// Constructs one optional vocabulary input for the shared project schema.
fn project_vocabulary(
    options: &VocabularyOptions,
    byte_limit: u64,
) -> Result<Option<CapturedVocabularyInput>, CliFailure> {
    if options.is_empty() {
        return Ok(None);
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
    Ok(Some(CapturedVocabularyInput::new(bytes, lock)))
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

/// Compiles through current semantics and checks complete reader authority before publishing bytes.
fn successful_project(
    request: &CapturedExpressionProject,
    cancel: &CancellationToken,
) -> Result<ValidatedExpressionProject, CliFailure> {
    let ir =
        compile_expression_project(request, cancel).map_err(|error| compilation_failure(&error))?;
    let limits = SemanticLimits {
        project: ir.limits,
        composition: request.composition_limits(),
        expression: request.expression_limits(),
        expansion: request.expansion_limits(),
    };
    ValidatedExpressionProject::from_ir(ir, limits, cancel).map_err(|error| {
        CliFailure::one(
            if error == neutral_reader::ExpressionReadError::Cancelled {
                ExitClass::Cancelled
            } else {
                ExitClass::Internal
            },
            "reader-validation-defect",
        )
    })
}

/// Capture errors expose only safe classifications, never the selected host path.
fn capture_failure(error: &ExpressionCaptureFailure) -> CliFailure {
    CliFailure::one(
        if error.is_cancelled() {
            ExitClass::Cancelled
        } else {
            ExitClass::Validation
        },
        error.code(),
    )
}

/// Current source diagnostics retain checked byte ranges without source identities.
fn compilation_failure(failure: &ExpressionCompileFailure) -> CliFailure {
    let class = if failure.is_cancelled() {
        ExitClass::Cancelled
    } else {
        ExitClass::Validation
    };
    let message = if let Some(location) = failure.location {
        format!(
            "{} bytes {}..{}",
            failure.code,
            location.span().start(),
            location.span().end()
        )
    } else {
        failure.code.to_owned()
    };
    CliFailure::one(class, message)
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
            .map_err(stdout_write_failure)
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
    write_temporary(
        temporary_file
            .write_all(bytes)
            .and_then(|()| temporary_file.sync_all()),
    )?;
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

/// Maps a temporary-file write result to its stable CLI outcome.
fn write_temporary(result: io::Result<()>) -> Result<(), CliFailure> {
    result.map_err(|_| CliFailure::one(ExitClass::Output, "output-write-failed"))
}

/// Maps a standard-output write failure to its stable CLI outcome.
fn stdout_write_failure(_: io::Error) -> CliFailure {
    CliFailure::one(ExitClass::Output, "stdout-write-failed")
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

#[cfg(test)]
#[path = "../tests/host/mod.rs"]
mod tests;
