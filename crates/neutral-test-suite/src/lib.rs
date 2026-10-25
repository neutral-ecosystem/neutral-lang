// SPDX-License-Identifier: Apache-2.0

//! Owner package for executable cross-package Neutral tests.
//!
//! Smoke, integration, system, conformance, determinism, and security tests are
//! grouped here by responsibility. This non-published crate must
//! not provide production APIs or duplicate normative fixtures.

#[cfg(test)]
#[path = "../tests/suite/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "../tests/hardening/mod.rs"]
mod hardening;

#[cfg(test)]
#[path = "../tests/source_pipeline/mod.rs"]
mod source_pipeline;

#[cfg(test)]
#[path = "../tests/composition_contract/mod.rs"]
mod composition_contract;

#[cfg(test)]
#[path = "../tests/expression_pipeline/mod.rs"]
mod expression_pipeline;
