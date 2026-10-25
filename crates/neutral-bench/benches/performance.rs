// SPDX-License-Identifier: Apache-2.0
//! Controlled dependency-free performance, stress, and soak harness.
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
fn main() {
    let profile = env::args().nth(1).unwrap_or_else(|| "pr".to_owned());
    if profile == "--list" {
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
