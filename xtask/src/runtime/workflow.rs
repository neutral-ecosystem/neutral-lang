// SPDX-License-Identifier: Apache-2.0

//! runtime / workflow responsibilities for repository automation.

use crate::{
    Instant, OpenOptions, Path, SystemTime, UNIX_EPOCH, WorkflowStep, command_output, constants,
    fs, json_string, output, read_workspace_text, result_root, rustc_command,
    unique_generated_directory, workspace_package_license, workspace_package_version,
    workspace_root,
};
use std::io::Write as _;

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
    writeln!(
        file,
        "{{\"workflow\":\"{}\",\"profile\":\"{}\",\"step\":\"{}\",\"status\":\"{}\",\"timestamp_unix_ms\":{timestamp_unix_ms},\"duration_ms\":{duration_ms},\"error\":{}}}",
        json_string(workflow),
        json_string(profile),
        json_string(step),
        json_string(status),
        json_optional_string(error)
    )
    .map_err(|error| format!("could not write workflow event log: {error}"))
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
    let rustc = command_output(&rustc_command()?, &["--version"])?;
    fs::write(
        path,
        format!(
            "{{\n  \"schema_version\": 1,\n  \"workflow\": \"{}\",\n  \"profile\": \"{}\",\n  \"status\": \"{}\",\n  \"completed_steps\": {completed_steps},\n  \"total_steps\": {total_steps},\n  \"started_at_unix_ms\": {started_at_unix_ms},\n  \"updated_at_unix_ms\": {},\n  \"source_commit\": \"{}\",\n  \"worktree_clean\": {worktree_clean},\n  \"package_version\": \"{}\",\n  \"license\": \"{}\",\n  \"rustc\": \"{}\",\n  \"error\": {}\n}}\n",
            json_string(workflow),
            json_string(profile),
            json_string(status),
            unix_time_millis()?,
            json_string(&commit),
            json_string(&version),
            json_string(&license),
            json_string(&rustc),
            json_optional_string(error)
        ),
    )
    .map_err(|error| format!("could not write workflow summary: {error}"))
}

/// Encodes an optional string as one JSON value.
pub(crate) fn json_optional_string(value: Option<&str>) -> String {
    value.map_or_else(
        || "null".to_owned(),
        |value| format!("\"{}\"", json_string(value)),
    )
}

/// Returns milliseconds elapsed since the Unix epoch for generated evidence.
pub(crate) fn unix_time_millis() -> Result<u128, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .map_err(|error| format!("system clock precedes Unix epoch: {error}"))
}
