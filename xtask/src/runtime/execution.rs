// SPDX-License-Identifier: Apache-2.0

//! runtime / execution responsibilities for repository automation.

use crate::{Command, Path, cargo_command, constants, output, workspace_root};

/// Runs one already-resolved executable with inherited standard streams.
pub(crate) fn run_program(program: &Path, arguments: &[&std::ffi::OsStr]) -> Result<(), String> {
    let printable = arguments
        .iter()
        .map(|value| value.to_string_lossy())
        .collect::<Vec<_>>();
    let printable = printable
        .iter()
        .map(std::convert::AsRef::as_ref)
        .collect::<Vec<_>>();
    output::invocation(&program.to_string_lossy(), &printable);
    let status = Command::new(program)
        .current_dir(workspace_root()?)
        .env(constants::CARGO_TERM_COLOR_ENV, output::child_color())
        .args(arguments)
        .status()
        .map_err(|error| format!("could not run {}: {error}", program.display()))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{} failed with {status}", program.display()))
}

/// Runs Cargo with inherited standard streams and converts failures to task errors.
pub(crate) fn run_cargo(arguments: &[&str]) -> Result<(), String> {
    let cargo = cargo_command()?;
    output::invocation(&cargo, arguments);
    let status = Command::new(&cargo)
        .current_dir(workspace_root()?)
        .args(arguments)
        .env(constants::CARGO_TERM_COLOR_ENV, output::child_color())
        .status()
        .map_err(|error| format!("could not run {} {}: {error}", cargo, arguments.join(" ")))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("{} {} failed with {status}", cargo, arguments.join(" ")))
}

/// Returns trimmed UTF-8 output from a successful command.
pub(crate) fn command_output(command: &str, arguments: &[&str]) -> Result<String, String> {
    let output = Command::new(command)
        .args(arguments)
        .output()
        .map_err(|error| format!("could not run {command}: {error}"))?;
    if !output.status.success() {
        return Err(format!("{command} failed with {}", output.status));
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|error| format!("{command} emitted non-UTF-8 output: {error}"))
}
