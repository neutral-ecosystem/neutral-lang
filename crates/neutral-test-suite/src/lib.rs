// SPDX-License-Identifier: Apache-2.0

//! Owner package for executable cross-package Neutral tests.
//!
//! Smoke, integration, system, conformance, determinism, and security tests are
//! introduced here as their stages become active. This non-published crate must
//! not provide production APIs or duplicate normative fixtures.

#[cfg(test)]
#[path = "../tests/suite/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/stage9/mod.rs"]
mod stage9;
