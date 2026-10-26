// SPDX-License-Identifier: Apache-2.0

//! Frozen standalone-document contract selectors, independent of Cargo package releases.
//!
//! These contracts evolve independently even when their current versions coincide.
//! Project, composition and expression selectors remain with their owning models;
//! source-header spellings belong to `neutral_core::profile`.
//!
//! These constants describe the minimal document/scalar companion contracts;
//! they are not the selector for the current complete expression project. Code
//! choosing a capture, codec or identity must use that model's exact tuple rather
//! than infer compatibility from these strings or the workspace package version.
//! Retaining a frozen selector preserves its meaning for shared contracts and
//! vectors; it does not advertise that a historical transport remains available.

/// Frozen logical IR schema version for the minimal document artifact.
pub const LOGICAL_IR_SCHEMA_VERSION: &str = "0.1.0";
/// Frozen document language-behavior version shared by artifacts and producers.
pub const LANGUAGE_BEHAVIOR_VERSION: &str = "0.1.0";
/// Frozen source-map schema version for the minimal document artifact.
pub const SOURCE_MAP_VERSION: &str = "0.1.0";
/// Frozen provenance schema version for the minimal document artifact.
pub const PROVENANCE_VERSION: &str = "0.1.0";
/// Frozen derivation schema version for the minimal document artifact.
pub const DERIVATION_SCHEMA_VERSION: &str = "0.1.0";
