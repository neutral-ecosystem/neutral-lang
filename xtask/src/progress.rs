// SPDX-License-Identifier: Apache-2.0

//! Derived development progress from the active portable and generated gates.

use std::{fs, path::Path};

use crate::{
    command_output, constants, is_safe_result_path, read_workspace_text, workspace_package_version,
    workspace_root,
};

/// A compact view of checklist completion and the next open task.
#[derive(Debug, Default, Eq, PartialEq)]
struct ChecklistProgress {
    /// Number of checked tasks.
    done: usize,
    /// Number of unchecked tasks.
    remaining: usize,
    /// Heading that owns the first unchecked task.
    next_heading: Option<String>,
    /// First unchecked task in source order.
    next_task: Option<String>,
}

/// Prints the current work item and whether local CI covers the current commit.
pub(crate) fn show() -> Result<(), String> {
    let root = workspace_root()?;
    let version = workspace_package_version(&read_workspace_text(
        &root,
        constants::WORKSPACE_MANIFEST_FILE,
    )?)?;
    let head = command_output("git", &["rev-parse", "HEAD"])?;
    let dirty = !command_output("git", &["status", "--porcelain"])?.is_empty();
    let stage = active_conformance_stage(&root)?;

    println!(
        "{} package: v{version}; active conformance stage: {stage}",
        constants::INFO
    );
    println!(
        "{} source: {}{}",
        constants::INFO,
        head.trim(),
        if dirty { " (dirty)" } else { "" }
    );

    let portable = root.join("portable");
    if portable.is_dir() {
        let checklist = checklist_path(&portable)?;
        let content = fs::read_to_string(&checklist)
            .map_err(|error| format!("could not read {}: {error}", checklist.display()))?;
        let progress = parse_checklist(&content);
        println!(
            "{} checklist: {} complete, {} remaining ({})",
            constants::INFO,
            progress.done,
            progress.remaining,
            checklist
                .strip_prefix(&root)
                .unwrap_or(&checklist)
                .display()
        );
        if let Some(task) = progress.next_task {
            println!(
                "{} next: {} — {task}",
                constants::INFO,
                progress
                    .next_heading
                    .unwrap_or_else(|| "unsectioned".to_owned())
            );
        } else {
            println!("{} next: review release qualification", constants::INFO);
        }
    } else {
        println!(
            "{} portable: absent; install the next reviewed plan",
            constants::INFO
        );
    }

    let current_ci = !dirty && has_passing_ci_for_head(&root, head.trim())?;
    println!(
        "{} local CI for current clean HEAD: {}",
        constants::INFO,
        if current_ci { "pass" } else { "not recorded" }
    );
    println!(
        "{} action: {}",
        constants::INFO,
        if dirty {
            "run `cargo xtask dev`, then commit and run `cargo xtask ci pr`"
        } else if current_ci {
            "work on the next unchecked task; retain only exceptional review decisions"
        } else {
            "run `cargo xtask ci pr` before promotion"
        }
    );
    Ok(())
}

/// Derives the highest active suite stage, ignoring planned suites and future cases.
pub(crate) fn active_conformance_stage(root: &Path) -> Result<u8, String> {
    let manifest = root.join(constants::PORTABLE_CONFORMANCE_MANIFEST_FILE);
    if !manifest.is_file() {
        return Ok(0);
    }
    let content = fs::read_to_string(&manifest)
        .map_err(|error| format!("could not read {}: {error}", manifest.display()))?;
    parse_active_stage(&content)
}

/// Parses only suite-level activation, never case-level future activation.
fn parse_active_stage(manifest: &str) -> Result<u8, String> {
    let mut maximum = 0;
    let mut in_suite = false;
    let mut stage = None;
    let mut required = false;
    for line in manifest.lines().map(str::trim) {
        if line == "[[suite]]" || line == "[[case]]" {
            if in_suite && required {
                maximum = maximum.max(stage.ok_or("required suite has no active_from_stage")?);
            }
            in_suite = line == "[[suite]]";
            stage = None;
            required = false;
            continue;
        }
        if !in_suite {
            continue;
        }
        if let Some(value) = line.strip_prefix("active_from_stage =") {
            stage = Some(
                value
                    .trim()
                    .parse::<u8>()
                    .map_err(|error| format!("invalid active_from_stage: {error}"))?,
            );
        } else if line == "status = \"required\"" {
            required = true;
        }
    }
    if in_suite && required {
        maximum = maximum.max(stage.ok_or("required suite has no active_from_stage")?);
    }
    Ok(maximum)
}

/// Selects the checklist declared by the portable, falling back to its plan.
fn checklist_path(portable: &Path) -> Result<std::path::PathBuf, String> {
    let lifecycle = portable.join("lifecycle.toml");
    let content = fs::read_to_string(&lifecycle)
        .map_err(|error| format!("could not read {}: {error}", lifecycle.display()))?;
    let relative = content
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("checklist = \"")?.strip_suffix('"'))
        .unwrap_or("PLAN.md");
    if !is_safe_result_path(Path::new(relative)) {
        return Err(format!("unsafe portable checklist path: {relative}"));
    }
    Ok(portable.join(relative))
}

/// Counts Markdown task markers and identifies the first open task.
fn parse_checklist(content: &str) -> ChecklistProgress {
    let mut progress = ChecklistProgress::default();
    let mut heading = String::new();
    for line in content.lines().map(str::trim) {
        if line.starts_with('#') {
            heading = line.trim_start_matches('#').trim().to_owned();
        } else if line.starts_with("- [x] ") {
            progress.done += 1;
        } else if let Some(task) = line.strip_prefix("- [ ] ") {
            progress.remaining += 1;
            if progress.next_task.is_none() {
                progress.next_heading = Some(heading.clone());
                progress.next_task = Some(task.to_owned());
            }
        }
    }
    progress
}

/// Checks ignored workflow summaries for a passing run bound to this exact HEAD.
fn has_passing_ci_for_head(root: &Path, head: &str) -> Result<bool, String> {
    let directory = root.join("test-results/workflows/ci/pr");
    if !directory.is_dir() {
        return Ok(false);
    }
    let marker = format!("\"source_commit\":\"{head}\"");
    for entry in fs::read_dir(&directory)
        .map_err(|error| format!("could not inspect {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("could not inspect CI run: {error}"))?;
        let summary = entry.path().join("summary.json");
        if summary.is_file() {
            let content = fs::read_to_string(&summary)
                .map_err(|error| format!("could not read {}: {error}", summary.display()))?;
            if content.contains(&marker) && content.contains("\"status\":\"pass\"") {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[cfg(test)]
#[path = "../tests/unit/progress.rs"]
mod tests;
