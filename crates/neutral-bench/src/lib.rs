// SPDX-License-Identifier: Apache-2.0

//! Owner package for Neutral benchmark harnesses and immutable corpora.
//!
//! This non-published crate must not be a dependency of production packages and
//! must not define language behavior. Tests smoke the same current-pipeline
//! operations used by the standalone performance harness.
//!
//! # Workload ownership
//!
//! The shared pipeline is compiled into the executable benchmark and, separately,
//! this library's tests. It exercises captured source through compilation,
//! independent reading, wire encoding/decoding, identities, public projection,
//! probing and real parser-cache generations. Production crates remain independent
//! of this package; their APIs and validation rules define the behavior being timed.
//!
//! # Interpreting results
//!
//! Named profiles choose repetition counts for the same embedded corpus. Most
//! phase records report total wall time after one untimed warm-up; growth and
//! concurrent isolation instead run one bounded scenario each. Equality/cache
//! assertions deliberately remain in relevant measured paths to catch invalid
//! work masquerading as an optimization. This is a correctness-checked workload,
//! not a statistical microbenchmark framework or exhaustive conformance suite.
//!
//! The harness emits machine-readable phase/count/duration records. The xtask
//! quality workflow owns source-bound evidence, external instrumentation and
//! qualification policy; a successful harness exit alone does not certify a
//! controlled runner, a performance regression threshold or memory safety.

#[cfg(test)]
#[path = "pipeline.rs"]
mod pipeline;
