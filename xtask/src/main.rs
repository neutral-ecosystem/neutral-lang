// SPDX-License-Identifier: Apache-2.0

//! Repository automation entry point for Neutral.
//!
//! This non-published package owns developer, CI, and evidence workflows. It
//! must remain outside every production dependency graph. Stage 1, Step 2
//! provides the boundary check; the remaining stable automation commands are
//! introduced in Step 3.

/// Runs the repository-automation command-line entry point.
fn main() {
    if let Err(error) = xtask::run(std::env::args().skip(1)) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
