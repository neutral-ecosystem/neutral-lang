// SPDX-License-Identifier: Apache-2.0

//! Neutral compilation pipeline boundary.
//!
//! This crate owns capture contracts, the private frontend and semantic model,
//! and lowering into public logical IR. Its pure captured-input compilation
//! path must not use filesystem, process, environment, network, locale, or clock
//! authority. No language behavior is implemented during Stage 1.
