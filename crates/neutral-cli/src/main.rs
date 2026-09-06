// SPDX-License-Identifier: Apache-2.0

//! Host-facing command line for Neutral.
//!
//! This package owns explicit filesystem and process adapters for capture,
//! compile, validate, and format commands. It does not provide inspection,
//! which remains the independent `neutral-probe` responsibility.

/// Starts the host-facing Neutral command-line shell.
fn main() {
    if let Err(failure) = neutral_cli::execute(std::env::args().skip(1)) {
        for message in failure.messages() {
            eprintln!("{} {message}", neutral_cli::constants::ERROR);
        }
        std::process::exit(i32::from(failure.class().code()));
    }
}
