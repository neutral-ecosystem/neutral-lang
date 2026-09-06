// SPDX-License-Identifier: Apache-2.0

//! Standalone Neutral encoded-artifact probe.

use neutral_core::CancellationToken;
use neutral_encoding::{DecodeError, DecodeLimits};
use neutral_probe::{ProbeSummary, inspect_encoded, output};
use std::{fs, path::Path};

/// Starts the standalone Neutral artifact probe.
fn main() {
    if let Err(error) = run(std::env::args().skip(1)) {
        eprintln!("{} {error}", output::ERROR);
        std::process::exit(2);
    }
}

/// Runs standalone encoded-artifact inspection without linking the compiler.
fn run(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    match arguments.into_iter().collect::<Vec<_>>().as_slice() {
        [argument] if argument == "--help" => {
            println!(
                "{} usage: {} <artifact>",
                output::INFO,
                env!("CARGO_PKG_NAME")
            );
            Ok(())
        }
        [argument] if argument == "--version" => {
            println!(
                "{} {} {}",
                output::INFO,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        }
        [] => Err("an artifact is required; run neutral-probe --help".to_owned()),
        [artifact] => inspect_path(Path::new(artifact)),
        _ => Err("exactly one encoded artifact path is required".to_owned()),
    }
}

/// Reads, validates, and renders one external artifact path.
fn inspect_path(path: &Path) -> Result<(), String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("could not read artifact {}: {error}", path.display()))?;
    let summary = inspect_encoded(&bytes, DecodeLimits::hard(), &CancellationToken::new())
        .map_err(render_decode_error)?;
    for line in render_summary(&summary) {
        println!("{} {line}", output::INFO);
    }
    Ok(())
}

/// Renders one bounded decoder error without exposing hostile artifact content.
fn render_decode_error(error: DecodeError) -> String {
    error.offset().map_or_else(
        || error.code().to_owned(),
        |offset| format!("{} at encoded byte {offset}", error.code()),
    )
}

/// Renders deterministic generic observations as one line per public reader view.
fn render_summary(summary: &ProbeSummary) -> Vec<String> {
    let mut lines = vec![format!("module {}", summary.module())];
    if let Some(vocabulary) = summary.vocabulary() {
        lines.push(format!("vocabulary {vocabulary}"));
    }
    lines.extend(
        summary
            .record_types()
            .iter()
            .map(|record| format!("record {record}")),
    );
    lines.extend(
        summary
            .vocabulary_types()
            .iter()
            .map(|record| format!("vocabulary-type {record}")),
    );
    lines.extend(
        summary
            .declarations()
            .iter()
            .map(|declaration| format!("declaration {declaration}")),
    );
    lines.extend(
        summary
            .field_provenance()
            .iter()
            .map(|record| format!("field-provenance {record}")),
    );
    lines.extend(
        summary
            .reuse_provenance()
            .iter()
            .map(|record| format!("reuse-provenance {record}")),
    );
    lines.extend(
        summary
            .reference_provenance()
            .iter()
            .map(|record| format!("reference-provenance {record}")),
    );
    lines
}

#[cfg(test)]
/// Tests the standalone probe command boundary.
mod tests {
    use super::run;

    #[test]
    /// Verifies that the probe accepts its help flag.
    fn package_shell_probe_accepts_help() {
        assert!(run(["--help".to_owned()]).is_ok());
    }

    #[test]
    /// Verifies that the probe rejects a missing artifact.
    fn package_shell_probe_rejects_empty_arguments() {
        assert!(run(Vec::new()).is_err());
    }

    #[test]
    /// Verifies that the probe reports an unreadable artifact without panicking.
    fn package_shell_probe_rejects_an_unreadable_artifact() {
        assert!(run(["definitely-missing-neutral-artifact.nir".to_owned()]).is_err());
    }
}
