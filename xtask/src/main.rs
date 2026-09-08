// SPDX-License-Identifier: Apache-2.0

//! Repository automation entry point for Neutral.
//!
//! This non-published package owns developer, CI, and evidence workflows. It
//! must remain outside every production dependency graph. The library owns the
//! stable command grammar and repository policy; this binary only maps failures
//! to categorized terminal output.

/// Runs the repository-automation command-line entry point.
fn main() {
    if let Err(error) = xtask::run(std::env::args().skip(1)) {
        eprintln!("{} {error}", xtask::constants::ERROR);
        std::process::exit(1);
    }
}
