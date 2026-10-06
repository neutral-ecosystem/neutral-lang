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
#[path = "../tests/project_capture/mod.rs"]
mod project_capture;

#[cfg(test)]
#[path = "../tests/module_graph/mod.rs"]
mod module_graph;

#[cfg(test)]
#[path = "../tests/public_semantics/mod.rs"]
mod public_semantics;

#[cfg(test)]
#[path = "../tests/project_ir/mod.rs"]
mod project_ir;

#[cfg(test)]
#[path = "../tests/project_identity/mod.rs"]
mod project_identity;

#[cfg(test)]
#[path = "../tests/source_pipeline/mod.rs"]
mod source_pipeline;

#[cfg(test)]
#[path = "../tests/composition_contract/mod.rs"]
mod composition_contract;
