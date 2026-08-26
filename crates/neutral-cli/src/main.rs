// SPDX-License-Identifier: Apache-2.0

//! Host-facing command line for Neutral.
//!
//! This package owns future filesystem and process adapters for capture,
//! compile, validate, and format commands. It must not serve as the independent
//! probe artifact. Stage 1 intentionally provides no language commands.

/// Error output category prefix.
const ERROR_PREFIX: &str = "[error]";
/// Informational output category prefix.
const INFO_PREFIX: &str = "[info]";

/// Starts the future host-facing Neutral command-line adapter.
fn main() {
    if let Err(error) = run(std::env::args().skip(1)) {
        eprintln!("{ERROR_PREFIX} {error}");
        std::process::exit(2);
    }
}

/// Validates the Stage 1 command shell without performing host acquisition.
fn run(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    match arguments.into_iter().collect::<Vec<_>>().as_slice() {
        [argument] if argument == "--help" => {
            println!(
                "{} usage: {} <command>",
                INFO_PREFIX,
                env!("CARGO_PKG_NAME")
            );
            Ok(())
        }
        [argument] if argument == "--version" => {
            println!(
                "{} {} {}",
                INFO_PREFIX,
                env!("CARGO_PKG_NAME"),
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        }
        [] => Err("a command is required; run neutral-cli --help".to_owned()),
        _ => Err("Neutral host commands are not active during Stage 1".to_owned()),
    }
}

#[cfg(test)]
/// Tests the Stage 1 CLI shell.
mod tests {
    use super::run;

    #[test]
    /// Verifies that the CLI shell accepts its help flag.
    fn package_shell_cli_accepts_help() {
        assert!(run(["--help".to_owned()]).is_ok());
    }

    #[test]
    /// Verifies that the CLI shell rejects a missing command.
    fn package_shell_cli_rejects_empty_arguments() {
        assert!(run(Vec::new()).is_err());
    }
}
