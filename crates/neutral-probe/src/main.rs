// SPDX-License-Identifier: Apache-2.0

//! Standalone Neutral artifact probe.
//!
//! The binary shell remains deliberately inert until Stage 8 activates encoded
//! artifact input and standalone probe behavior.

/// Error output category prefix.
const ERROR_PREFIX: &str = "[error]";
/// Informational output category prefix.
const INFO_PREFIX: &str = "[info]";

/// Starts the standalone Neutral artifact probe.
fn main() {
    if let Err(error) = run(std::env::args().skip(1)) {
        eprintln!("{ERROR_PREFIX} {error}");
        std::process::exit(2);
    }
}

/// Validates the pre-Stage-8 probe shell without linking the compiler.
fn run(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    match arguments.into_iter().collect::<Vec<_>>().as_slice() {
        [argument] if argument == "--help" => {
            println!(
                "{} usage: {} <artifact>",
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
        [] => Err("an artifact is required; run neutral-probe --help".to_owned()),
        _ => Err("encoded artifact probing is not active before Stage 8".to_owned()),
    }
}

#[cfg(test)]
/// Tests the pre-Stage-8 probe shell.
mod tests {
    use super::run;

    #[test]
    /// Verifies that the probe shell accepts its help flag.
    fn package_shell_probe_accepts_help() {
        assert!(run(["--help".to_owned()]).is_ok());
    }

    #[test]
    /// Verifies that the probe shell rejects a missing artifact.
    fn package_shell_probe_rejects_empty_arguments() {
        assert!(run(Vec::new()).is_err());
    }
}
