// SPDX-License-Identifier: Apache-2.0

//! Standalone Neutral artifact probe.
//!
//! The Stage 1 binary is deliberately inert: encoded artifact input and probe
//! behavior are introduced only at their scheduled implementation stages.

/// Starts the standalone Neutral artifact probe.
fn main() {
    if let Err(error) = run(std::env::args().skip(1)) {
        eprintln!("[error] {error}");
        std::process::exit(2);
    }
}

/// Validates the Stage 1 probe shell without linking the compiler.
fn run(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    match arguments.into_iter().collect::<Vec<_>>().as_slice() {
        [argument] if argument == "--help" => {
            println!("[info] usage: neutral-probe <artifact>");
            Ok(())
        }
        [argument] if argument == "--version" => {
            println!("[info] neutral-probe {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        [] => Err("an artifact is required; run neutral-probe --help".to_owned()),
        _ => Err("encoded artifact probing is not active during Stage 1".to_owned()),
    }
}

#[cfg(test)]
/// Tests the Stage 1 probe shell.
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
