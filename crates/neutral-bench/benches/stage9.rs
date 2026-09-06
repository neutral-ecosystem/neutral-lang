// SPDX-License-Identifier: Apache-2.0

//! Controlled dependency-free Stage 9 performance, stress, and soak harness.

use neutral_compiler::{CompilationRequest, CompilationResult, compile};
use neutral_core::{CancellationToken, StructuralLimits};
use neutral_encoding::{DecodeLimits, ProducerInfo, decode, encode};
use neutral_probe::summarize;
use neutral_reader::ValidatedDocument;
use std::{
    env,
    fmt::Write as _,
    hint::black_box,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

/// Output category used by benchmark status lines.
const INFO: &str = "[info]";
/// Error category used by invalid benchmark invocations.
const ERROR: &str = "[error]";
/// Stable benchmark producer envelope name.
const BENCH_PRODUCER: &str = "neutral-bench";
/// Cargo test-list argument accepted by a harness-free benchmark target.
const TEST_LIST_ARGUMENT: &str = "--list";
/// Fast informational local/PR profile name.
const PR_PROFILE: &str = "pr";
/// Repeated controlled release-candidate profile name.
const RELEASE_PROFILE: &str = "release";
/// Bounded stress/soak profile name.
const SOAK_PROFILE: &str = "soak";
/// Current benchmark package version.
const BENCH_PRODUCER_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Small PR-profile iteration count.
const PR_ITERATIONS: usize = 25;
/// Repeated release-profile iteration count.
const RELEASE_ITERATIONS: usize = 250;
/// Bounded local soak iteration count.
const SOAK_ITERATIONS: usize = 5_000;
/// Worker count used for the isolation/concurrency profile.
const CONCURRENCY_WORKERS: usize = 8;
/// Representative immutable source corpus entry.
const REPRESENTATIVE_SOURCE: &[u8] =
    include_bytes!("../../../portable/specs/fixtures/positive/lists/record-list-default.neu");

/// Runs the selected controlled benchmark profile.
fn main() {
    let profile = env::args().nth(1).unwrap_or_else(|| PR_PROFILE.to_owned());
    if profile == TEST_LIST_ARGUMENT {
        return;
    }
    let iterations = match profile.as_str() {
        PR_PROFILE => PR_ITERATIONS,
        RELEASE_PROFILE => RELEASE_ITERATIONS,
        SOAK_PROFILE => SOAK_ITERATIONS,
        _ => {
            eprintln!("{ERROR} unknown performance profile: {profile}");
            std::process::exit(2);
        }
    };
    let limits = benchmark_limits();
    let compile_elapsed = measure(iterations, || {
        black_box(compile_success(REPRESENTATIVE_SOURCE, limits));
    });
    let artifacts = compile_success(REPRESENTATIVE_SOURCE, limits);
    let reader_elapsed = measure(iterations, || {
        black_box(
            ValidatedDocument::from_compiler_output(Arc::clone(&artifacts))
                .expect("compiler output must remain reader-valid"),
        );
    });
    let reader = ValidatedDocument::from_compiler_output(artifacts)
        .expect("compiler output must remain reader-valid");
    let encode_elapsed = measure(iterations, || {
        black_box(
            encode(
                &reader,
                &ProducerInfo::new(BENCH_PRODUCER, BENCH_PRODUCER_VERSION),
            )
            .expect("representative artifacts must encode"),
        );
    });
    let encoded = encode(
        &reader,
        &ProducerInfo::new(BENCH_PRODUCER, BENCH_PRODUCER_VERSION),
    )
    .expect("representative artifacts must encode");
    let decode_elapsed = measure(iterations, || {
        black_box(
            decode(
                encoded.as_bytes(),
                DecodeLimits::hard(),
                &CancellationToken::new(),
            )
            .expect("representative artifact must decode"),
        );
    });
    let probe_elapsed = measure(iterations, || {
        black_box(summarize(&reader));
    });
    let growth_elapsed = growth_profile(limits);
    let concurrency_elapsed = concurrency_profile(limits);

    report("compile-end-to-end", iterations, compile_elapsed);
    report("reader-validation", iterations, reader_elapsed);
    report("artifact-encoding", iterations, encode_elapsed);
    report("artifact-decoding", iterations, decode_elapsed);
    report("probe-traversal", iterations, probe_elapsed);
    report("declaration-growth", 4, growth_elapsed);
    report(
        "concurrent-isolation",
        CONCURRENCY_WORKERS,
        concurrency_elapsed,
    );
    println!(
        "{INFO} performance artifact-bytes={} source-bytes={} profile={profile}",
        encoded.as_bytes().len(),
        REPRESENTATIVE_SOURCE.len()
    );
}

/// Returns deterministic limits large enough for the benchmark corpus.
fn benchmark_limits() -> StructuralLimits {
    StructuralLimits::new(1_048_576, 32)
        .expect("benchmark limits must be nonzero")
        .with_declarations(4_096)
        .expect("benchmark declaration limit must be nonzero")
        .with_traversal_nodes(65_536)
        .expect("benchmark traversal limit must be nonzero")
}

/// Compiles one source and returns authoritative artifacts.
fn compile_success(
    source: &[u8],
    limits: StructuralLimits,
) -> Arc<neutral_ir::CompilationArtifacts> {
    let result = compile(CompilationRequest::new(
        source.to_vec(),
        limits,
        CancellationToken::new(),
    ))
    .expect("benchmark source must capture");
    let CompilationResult::Success(artifacts) = result else {
        panic!("benchmark source must compile");
    };
    artifacts
}

/// Measures a deterministic count of operations with one monotonic clock sample.
fn measure(mut iterations: usize, mut operation: impl FnMut()) -> Duration {
    operation();
    let start = Instant::now();
    while iterations > 0 {
        operation();
        iterations -= 1;
    }
    start.elapsed()
}

/// Exercises growth across increasing declaration counts.
fn growth_profile(limits: StructuralLimits) -> Duration {
    let start = Instant::now();
    for declarations in [1_usize, 8, 64, 256] {
        let mut source = String::from("neu \"0.1\"\nmodule growth\n");
        for index in 0..declarations {
            writeln!(source, "num value{index} = {index}")
                .expect("writing into a String must not fail");
        }
        black_box(compile_success(source.as_bytes(), limits));
    }
    start.elapsed()
}

/// Exercises independent compilation on a fixed number of concurrent workers.
fn concurrency_profile(limits: StructuralLimits) -> Duration {
    let start = Instant::now();
    let workers = (0..CONCURRENCY_WORKERS)
        .map(|_| {
            thread::spawn(move || {
                black_box(compile_success(REPRESENTATIVE_SOURCE, limits));
            })
        })
        .collect::<Vec<_>>();
    for worker in workers {
        worker.join().expect("benchmark worker must not panic");
    }
    start.elapsed()
}

/// Prints one stable benchmark measurement line.
fn report(name: &str, iterations: usize, elapsed: Duration) {
    println!(
        "{INFO} performance name={name} iterations={iterations} elapsed-ns={}",
        elapsed.as_nanos()
    );
}
