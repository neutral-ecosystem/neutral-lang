// SPDX-License-Identifier: Apache-2.0

//! runtime / workflow responsibilities for repository automation.

use crate::constants::flags;
use crate::{
    Instant, OpenOptions, Path, SystemTime, UNIX_EPOCH, WorkflowStep, command_output, constants,
    fs, output, read_workspace_text, result_root, rustc_command, unique_generated_directory,
    workspace_package_license, workspace_package_version, workspace_root,
};
use serde::Serialize;

/// Independent schema for generated workflow summaries.
const WORKFLOW_SCHEMA_VERSION: u32 = 1;
use std::io::Write as _;

/// One append-only JSONL event; optional errors retain their explicit null representation.
#[derive(Serialize)]
struct WorkflowEvent<'a> {
    /// Workflow name.
    workflow: &'a str,
    /// Selected execution profile.
    profile: &'a str,
    /// Ordered step name.
    step: &'a str,
    /// Event state.
    status: &'a str,
    /// Event time in Unix milliseconds.
    timestamp_unix_ms: u128,
    /// Monotonic step elapsed time.
    duration_ms: u128,
    /// Failure details, or null for nonfailure events.
    error: Option<&'a str>,
}

/// Current workflow state with source and tool provenance.
#[derive(Serialize)]
struct WorkflowSummary<'a> {
    /// Independent generated-document schema.
    schema_version: u32,
    /// Workflow name.
    workflow: &'a str,
    /// Selected execution profile.
    profile: &'a str,
    /// Current state.
    status: &'a str,
    /// Successful steps so far.
    completed_steps: usize,
    /// Number of selected steps.
    total_steps: usize,
    /// Initial Unix time.
    started_at_unix_ms: u128,
    /// Most recent Unix time.
    updated_at_unix_ms: u128,
    /// Checkout identity at this summary update.
    source_commit: String,
    /// Whether the checkout has tracked or untracked changes.
    worktree_clean: bool,
    /// Workspace package version.
    package_version: String,
    /// Workspace license expression.
    license: String,
    /// Selected compiler version.
    rustc: String,
    /// Failure details, or null.
    error: Option<&'a str>,
}

/// Runs ordered workflow steps while retaining start, pass, and failure events.
pub(crate) fn run_recorded_workflow(
    workflow: &str,
    profile: &str,
    steps: Vec<WorkflowStep<'_>>,
) -> Result<(), String> {
    let directory = unique_generated_directory(
        &result_root()?
            .join(constants::WORKFLOW_RESULT_DIRECTORY)
            .join(workflow)
            .join(profile),
    )?;
    let events = directory.join(constants::WORKFLOW_EVENTS_FILE);
    let summary = directory.join(constants::WORKFLOW_SUMMARY_FILE);
    let total = steps.len();
    let started_at = unix_time_millis()?;
    let started = Instant::now();
    write_workflow_summary(
        &summary, workflow, profile, "running", 0, total, started_at, None,
    )?;
    output::start(format!("{workflow} {profile} ({total} steps)"));
    output::file("workflow log", &directory);

    for (index, (name, step)) in steps.into_iter().enumerate() {
        let step_started_at = unix_time_millis()?;
        let step_started = Instant::now();
        append_workflow_event(
            &events,
            workflow,
            profile,
            name,
            "start",
            step_started_at,
            0,
            None,
        )?;
        output::start(format!("{workflow} [{}/{total}] {name}", index + 1));
        match step() {
            Ok(()) => {
                let elapsed = step_started.elapsed().as_millis();
                append_workflow_event(
                    &events,
                    workflow,
                    profile,
                    name,
                    "pass",
                    unix_time_millis()?,
                    elapsed,
                    None,
                )?;
                write_workflow_summary(
                    &summary,
                    workflow,
                    profile,
                    "running",
                    index + 1,
                    total,
                    started_at,
                    None,
                )?;
                output::pass(format!(
                    "{workflow} [{}/{total}] {name} ({})",
                    index + 1,
                    output::duration(step_started.elapsed())
                ));
            }
            Err(error) => {
                let elapsed = step_started.elapsed().as_millis();
                append_workflow_event(
                    &events,
                    workflow,
                    profile,
                    name,
                    "fail",
                    unix_time_millis()?,
                    elapsed,
                    Some(&error),
                )?;
                write_workflow_summary(
                    &summary,
                    workflow,
                    profile,
                    "fail",
                    index,
                    total,
                    started_at,
                    Some(&error),
                )?;
                return Err(format!(
                    "{workflow} [{}/{total}] {name} failed ({}): {error}; workflow log: {}",
                    index + 1,
                    output::duration(step_started.elapsed()),
                    output::path(&directory)
                ));
            }
        }
    }

    write_workflow_summary(
        &summary, workflow, profile, "pass", total, total, started_at, None,
    )?;
    output::pass(format!(
        "{workflow} {profile} ({total}/{total} steps, {})",
        output::duration(started.elapsed())
    ));
    Ok(())
}

/// Appends one machine-readable workflow event without replacing earlier events.
#[allow(clippy::too_many_arguments)]
pub(crate) fn append_workflow_event(
    path: &Path,
    workflow: &str,
    profile: &str,
    step: &str,
    status: &str,
    timestamp_unix_ms: u128,
    duration_ms: u128,
    error: Option<&str>,
) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("could not open workflow event log: {error}"))?;
    let event = WorkflowEvent {
        workflow,
        profile,
        step,
        status,
        timestamp_unix_ms,
        duration_ms,
        error,
    };
    // JSONL must remain one complete compact object per physical line.
    serde_json::to_writer(&mut file, &event)
        .map_err(|error| format!("could not serialize workflow event: {error}"))?;
    writeln!(file).map_err(|error| format!("could not write workflow event log: {error}"))
}

/// Writes the current machine-readable summary for an aggregate workflow.
#[allow(clippy::too_many_arguments)]
pub(crate) fn write_workflow_summary(
    path: &Path,
    workflow: &str,
    profile: &str,
    status: &str,
    completed_steps: usize,
    total_steps: usize,
    started_at_unix_ms: u128,
    error: Option<&str>,
) -> Result<(), String> {
    let root = workspace_root()?;
    let manifest = read_workspace_text(&root, constants::WORKSPACE_MANIFEST_FILE)?;
    let version = workspace_package_version(&manifest)?;
    let license = workspace_package_license(&manifest)?;
    let commit = command_output(constants::GIT_COMMAND, &["rev-parse", "HEAD"])?;
    let worktree_clean =
        command_output(constants::GIT_COMMAND, &["status", "--porcelain"])?.is_empty();
    let rustc = command_output(&rustc_command()?, &[flags::VERSION])?;
    let summary = WorkflowSummary {
        schema_version: WORKFLOW_SCHEMA_VERSION,
        workflow,
        profile,
        status,
        completed_steps,
        total_steps,
        started_at_unix_ms,
        updated_at_unix_ms: unix_time_millis()?,
        source_commit: commit,
        worktree_clean,
        package_version: version,
        license,
        rustc,
        error,
    };
    fs::write(path, super::json::pretty(&summary)?)
        .map_err(|error| format!("could not write workflow summary: {error}"))
}

/// Returns milliseconds elapsed since the Unix epoch for generated evidence.
pub(crate) fn unix_time_millis() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| format!("system clock precedes Unix epoch: {error}"))
}
