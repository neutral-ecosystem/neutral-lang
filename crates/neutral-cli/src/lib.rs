// SPDX-License-Identifier: Apache-2.0

//! Explicit host adapters and stable process policy for the Neutral CLI.
//!
//! Inspection remains outside this package in the independently linked
//! `neutral-probe` executable.

mod command;
pub mod constants;
mod host;

pub use host::{CliFailure, ExitClass, execute};
