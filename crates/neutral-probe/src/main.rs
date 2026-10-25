// SPDX-License-Identifier: Apache-2.0

//! Standalone Neutral encoded-artifact probe.

use neutral_core::CancellationToken;
use neutral_encoding::DecodeError;
use neutral_probe::{
    expression::{ExpressionProbeError, render_standalone_expression},
    output,
};
use std::{
    fs::File,
    io::{self, Read, Write as _},
    path::Path,
};

/// Starts the standalone Neutral artifact probe.
fn main() {
    if let Err(error) = run(std::env::args().skip(1)) {
        let _ = writeln!(io::stderr().lock(), "{} {error}", output::ERROR);
        std::process::exit(2);
    }
}

/// Runs standalone encoded-artifact inspection without linking the compiler.
fn run(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    match arguments.into_iter().collect::<Vec<_>>().as_slice() {
        [argument] if argument == "--help" => emit_stdout(&format!(
            "Usage: {} [--json] [--root module::declaration ...] <artifact>\n",
            env!("CARGO_PKG_NAME")
        )),
        [argument] if argument == "--version" => emit_stdout(&format!(
            "{} {}\n",
            env!("CARGO_PKG_NAME"),
            env!("CARGO_PKG_VERSION")
        )),
        [] => Err("an artifact is required; run neutral-probe --help".to_owned()),
        arguments => inspect_arguments(arguments),
    }
}

/// Reads, validates, and renders one external artifact path.
fn inspect_path(path: &Path, json: bool, roots: Option<&[String]>) -> Result<(), String> {
    let maximum = neutral_encoding::constants::MAXIMUM_ARTIFACT_BYTES;
    let mut bytes = Vec::new();
    File::open(path)
        .and_then(|file| file.take(maximum as u64 + 1).read_to_end(&mut bytes))
        .map_err(|error| format!("could not read artifact {}: {error}", path.display()))?;
    if bytes.len() > maximum {
        return Err(neutral_encoding::diagnostics::ENCODED_SIZE_LIMIT.to_owned());
    }
    if !bytes.starts_with(&neutral_reader::expression::profile::MAGIC) {
        return Err(format!(
            "unsupported artifact format; expected {}",
            neutral_reader::expression::profile::ENCODING
        ));
    }
    let rendered = render_standalone_expression(&bytes, roots, &CancellationToken::new())
        .map_err(render_expression_error)?;
    if json {
        emit_stdout(&rendered)
    } else {
        for line in rendered.lines() {
            emit_stdout(&format!("{} {line}\n", output::INFO))?;
        }
        Ok(())
    }
}

/// Reports stdout failures as a safe host classification instead of panicking.
fn emit_stdout(text: &str) -> Result<(), String> {
    io::stdout()
        .lock()
        .write_all(text.as_bytes())
        .map_err(|_| "probe-output-failed".to_owned())
}

/// Parses project root selection without changing legacy single-artifact usage.
fn inspect_arguments(arguments: &[String]) -> Result<(), String> {
    let mut json = false;
    let mut roots = Vec::new();
    let mut path = None;
    let mut iter = arguments.iter();
    while let Some(argument) = iter.next() {
        match argument.as_str() {
            "--json" if !json => json = true,
            "--root" => roots.push(
                iter.next()
                    .ok_or("--root requires module::declaration")?
                    .clone(),
            ),
            flag if flag.starts_with('-') => {
                return Err("unknown or duplicate probe option".to_owned());
            }
            artifact if path.is_none() => path = Some(Path::new(artifact)),
            _ => {
                return Err(
                    "exactly one encoded artifact path is required (optionally with --json)"
                        .to_owned(),
                );
            }
        }
    }
    let path = path.ok_or("an artifact is required; run neutral-probe --help")?;
    inspect_path(
        path,
        json,
        if roots.is_empty() { None } else { Some(&roots) },
    )
}

/// Renders one bounded decoder error without exposing hostile artifact content.
fn render_decode_error(error: DecodeError) -> String {
    error.offset().map_or_else(
        || error.code().to_owned(),
        |offset| format!("{} at encoded byte {offset}", error.code()),
    )
}

/// Reports the current checked boundary without exposing private artifact fields.
fn render_expression_error(error: ExpressionProbeError) -> String {
    match error {
        ExpressionProbeError::Decode(error) => render_decode_error(error),
        ExpressionProbeError::Entry(error) => format!(
            "{} {error:?}",
            neutral_reader::expression::profile::PROJECT_VIEW_SCHEMA
        ),
        ExpressionProbeError::Identity(error) => {
            format!(
                "{} identity {error:?}",
                neutral_reader::expression::profile::IDENTITY_PROFILE
            )
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/mod.rs"]
mod tests;
