// SPDX-License-Identifier: Apache-2.0

//! Owner package for Neutral benchmark harnesses and immutable corpora.
//!
//! This non-published crate must not be a dependency of production packages and
//! must not define language behavior. Tests smoke the same current-pipeline
//! operations used by the standalone performance harness.

#[cfg(test)]
#[path = "composition.rs"]
mod composition;
