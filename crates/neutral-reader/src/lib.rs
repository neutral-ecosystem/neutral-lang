// SPDX-License-Identifier: Apache-2.0

//! Validated, immutable access to Neutral artifacts.
//!
//! This crate owns validation at the external artifact boundary and the reader
//! views exposed after validation. It treats encoded data as untrusted and must
//! not acquire inputs or depend on compiler-private representations.
