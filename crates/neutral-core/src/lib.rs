// SPDX-License-Identifier: Apache-2.0

//! Foundational contracts shared across the Neutral implementation.
//!
//! This crate owns source identity, source spans, diagnostics, resource limits,
//! cancellation, and result classification. It must remain independent of the
//! compiler, reader, command-line hosts, and ambient host services.
