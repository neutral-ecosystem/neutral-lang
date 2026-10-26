// SPDX-License-Identifier: Apache-2.0
//! Standalone named-profile harness with no statistical benchmark-framework dependency.
//!
//! Cargo builds this target with `harness = false`, so this entry point selects
//! repetition counts and emits phase records directly. The shared pipeline also
//! runs in crate-owned smoke tests; it defines the work, correctness assertions
//! and timer boundaries, while this file owns profile selection and reporting.
//!
//! Direct invocation defaults to `pr`. `release`, `soak` and `extended-soak` repeat
//! ordinary phases for their respective counts; growth/concurrency remain fixed
//! one-shot scenarios. The public xtask soak command selects `extended-soak`, so
//! its effective count differs from invoking the direct `soak` profile here.
//!
//! `elapsed-ns` is a phase total rather than per-operation latency. `total-ns`
//! includes setup, warm-ups, correctness checks and report printing across the
//! whole run. Stable names/field spellings are consumed by evidence validation;
//! changing presentation must preserve that contract. External profiling and
//! controlled-runner qualification belong to xtask, not this executable.
#[path = "../src/pipeline.rs"]
mod current;
use std::{env, time::Instant};
/// Fast informational samples for the local development loop.
const PR_ITERATIONS: usize = 25;
/// Direct samples used alongside independent instrumentation.
const RELEASE_ITERATIONS: usize = 250;
/// Repeated requests for ordinary soak measurements.
const SOAK_ITERATIONS: usize = 5_000;
/// Extended repeated requests for controlled-runner qualification.
const EXTENDED_SOAK_ITERATIONS: usize = 25_000;
/// Selects a bounded named profile and emits the evidence parser's phase records.
///
/// Unknown profiles exit with status 2 before measuring anything. A phase failure
/// propagates as a panic from the shared pipeline, leaving an incomplete report
/// that must not be accepted as successful measurement evidence.
fn main() {
    let profile = env::args().nth(1).unwrap_or_else(|| "pr".to_owned());
    if profile == "--list" {
        // Cargo's target discovery may list benchmarks; discovery must not execute a workload.
        return;
    }
    let iterations = match profile.as_str() {
        "pr" => PR_ITERATIONS,
        "release" => RELEASE_ITERATIONS,
        "soak" => SOAK_ITERATIONS,
        "extended-soak" => EXTENDED_SOAK_ITERATIONS,
        _ => {
            eprintln!("FAIL unknown performance profile: {profile}");
            std::process::exit(2);
        }
    };
    let start = Instant::now();
    // Keep the callback's records parseable; reporting happens outside each phase's timer.
    current::run(iterations, |name, count, elapsed| {
        println!(
            "PASS performance name={name} iterations={count} elapsed-ns={}",
            elapsed.as_nanos()
        );
    });
    println!(
        "PASS performance profile={profile} total-ns={}",
        start.elapsed().as_nanos()
    );
}
