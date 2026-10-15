<!-- SPDX-License-Identifier: Apache-2.0 -->

# Quality and analysis

[< Back to Neutral](../README.md) • [Documentation Hub](README.md)

Repository quality policy is tracked under `config/` and `quality/`. Generated
machine-readable evidence belongs under ignored `test-results/` directories and
must not be edited manually.

Release measurements are produced automatically by the commands below. Each
successful run retains tool stdout/stderr, native reports, exact report digests,
compiler identity, actual measurement-tool versions, duration, and an input
fingerprint. Git-discovered tracked and non-ignored untracked files participate,
including Python oracles, scripts, templates, fixtures and configuration regardless
of extension. Top-level `docs/*.md` guides, `README.md` files, maintained
`quality/STATUS.md` and generated approval `record.toml` files are excluded.
Other Markdown, including executable language examples, remains an input;
changing an HTML template is not a documentation-only change.
The fingerprint/receipt schema change intentionally invalidates earlier receipts.
Historical `status` and `observed_*` configuration fields are not passing evidence.

Each input fingerprint/gate has an exclusive OS lock and separate `run-*` report
directories. Another measurement of the same gate/inputs fails with an
already-running message; independent gates can run concurrently. The `current`
pointer selects the latest attempt, and `receipt.json` appears only after all
reports and input freshness pass validation. A failed explicit rerun cannot reuse
an older retained success. Inspect the printed run directory, fix the cause, and
rerun the gate; do not manufacture a receipt or delete an active lock file.

## Quality workflow

| Need | Command |
| --- | --- |
| Run the normal gate | `cargo xtask quality` |
| Run release qualification | `cargo xtask quality --profile release` |
| Inspect durable status | `cargo xtask quality status` |
| Render maintained status | `cargo xtask quality render` |
| Verify maintained evidence | `cargo xtask quality verify` |
| Evaluate retained evidence | `cargo xtask quality evaluate --profile pr\|release` |

Every aggregate workflow records `events.jsonl` and `summary.json` beneath its
run directory in `test-results/workflows/`.

## Coverage and fuzzing

`cargo xtask test all` is the primary test command. Coverage and mutation also
use Nextest by default through their tools' native integrations; install it with
`cargo install cargo-nextest --locked`. Doctests run separately through Cargo;
fuzz campaigns and performance profiles use their dedicated runners.

Coverage-guided fuzzing and LLVM coverage use an isolated nightly toolchain;
stable remains the project default.

```sh
rustup toolchain install nightly --profile minimal
rustup component add --toolchain nightly llvm-tools-preview
cargo install cargo-llvm-cov cargo-fuzz
```

```sh
RUSTUP_TOOLCHAIN=nightly cargo xtask coverage
RUSTUP_TOOLCHAIN=nightly cargo xtask fuzz smoke
RUSTUP_TOOLCHAIN=nightly cargo xtask fuzz campaign
```

Full fuzz campaigns use the per-target budget configured in
`config/quality-gates.toml`; the command prints target count, per-target budget,
and estimated total time before starting. A live progress bar shows each target's
command elapsed time and estimated remaining budget; compilation/startup can
extend the estimate, and only successful exit plus validated fuzzer reports
confirm completion.
Full stdout/stderr remain retained in the reported evidence directory. Mutable
fuzz corpora, findings, and crash artifacts are not normative conformance data.

`cargo xtask coverage` reuses passing evidence for identical measurement inputs.
A changed source, test, fixture, lockfile, or configuration invalidates that result;
the command clears LLVM coverage data and runs the complete workspace again.

Coverage HTML is written to
`test-results/analysis/coverage/html/index.html`; its machine-readable summary
is `test-results/analysis/coverage/coverage.json`. Thresholds and reviewed
exclusions are defined in [`config/quality-gates.toml`](../config/quality-gates.toml).

Raw LLVM profiles (`*.profraw`) are generated coverage data, never source files.
The coverage runner configures `LLVM_PROFILE_FILE` beneath Cargo's target
directory. Running an instrumented binary directly without that setting can
leave `default_*.profraw` in its working directory; use the coverage command
above instead. Misplaced profiles can be moved into ignored
`test-results/analysis/coverage/` storage without changing crate structure.

## Release measurements

Measurements print their report directory once, then a labeled start, exact
command, elapsed-time progress, and pass/fail result for each tool invocation.
Tools without a fixed budget (including profilers) show elapsed time rather than
an invented percentage or ETA. Redirected logs receive periodic plain-text
updates instead of terminal control sequences. A passing tool invocation is
distinct from the final `evidence verified` result: only the latter confirms
acceptance of the retained reports. Failure output identifies the captured
stderr report to inspect.

Install the current analysis tools and the system's Valgrind package:

```sh
cargo install cargo-mutants cargo-audit --locked
```

For a release, `cargo xtask release prepare [version]` automatically runs missing
or stale measurements and selects nightly where required. No separate measurement
sequence is needed. Use the individual commands below to inspect a gate or diagnose
a failure on the final source inputs:

```sh
RUSTUP_TOOLCHAIN=nightly cargo xtask coverage
cargo xtask mutate --full
cargo xtask test performance --profile release
```

For development, `cargo xtask mutate` keeps a persistent local campaign and writes
no release receipt. Successful results are reused when the inputs are unchanged.
For unchanged interrupted or failed inputs, it uses cargo-mutants `--iterate` to
retry surviving and unfinished mutations. New standalone `.rs` test files can be
added conservatively without discarding the campaign. Modifying or deleting an
existing test, or changing source, configuration, dependencies, or the toolchain,
invalidates the campaign and starts it fresh.
Execution environment changes also invalidate it. When iteration leaves no pending
mutants, xtask runs the complete unmutated suite with the mutation test profile
before accepting the new inputs. The cache is local ignored output, removed by
`cargo xtask clean`; the first development campaign starts fresh rather than
importing historical release reports.

`cargo xtask mutate --full` always starts a fresh campaign and records release
evidence through the existing receipt pipeline. Release preparation automatically
selects this mode when mutation evidence is missing or stale. Both modes measure
every production file in `mutation.critical_targets` in
`config/quality-gates.toml`, spanning syntax, semantic restrictions, decoding,
reader references and identity framing. The command disables separate cargo-mutants
configuration so it cannot silently narrow that reviewed scope, and tests each
mutation against the workspace, including the cross-package suite. Receipts must
cover the whole configured list; the caught threshold is unchanged. Expanded
scope requires new measurements, not reuse of historical single-file results.
The campaign defaults to two parallel workers without reducing the mutation scope.
Set `testing.mutation_jobs` in `config/automation.toml` to a positive worker count;
more workers use more RAM and temporary disk space. This applies to both test backends.
Baseline and mutant runs both test the whole workspace, so automatic timeouts
are calibrated against the complete suite. The dedicated Nextest `mutation` profile
runs fast tests before allocation-failure sweeps and stops at the first failure:
one failing test already establishes that a mutant is caught. No tests are excluded;
the baseline and surviving mutations still execute the complete suite. Select this
profile through `testing.mutation_profile` in `config/automation.toml`; ordering
belongs in `config/nextest.toml`, independently of normal CI.
Workspace selection is forwarded with `--cargo-arg=--workspace`; combining this
with `--test-workspace true` would duplicate the flag in Nextest mutant runs.
Copies retain Git metadata so repository-policy tests do not resolve a parent
checkout; ignored generated build/report trees remain excluded.
Progress shows completed/discovered mutants and caught/missed/unviable/timeout
counts from native reports, excluding the baseline. Failures link directly to
the survivor list and native outcomes. Before reports are available, it shows elapsed time; the final evidence
validation, not the counter, determines whether the campaign passes.
This increases memory and disk usage. On systems with a small RAM-backed `/tmp`,
use disk-backed temporary storage:

```sh
mkdir -p target/mutation-tmp
TMPDIR="$PWD/target/mutation-tmp" cargo xtask mutate --full
```

For an optional broader investigation, use the native tool (not a substitute for
the configured gate):

```sh
cargo mutants --workspace --no-config --cargo-arg=--workspace --gitignore true --copy-vcs true --test-tool nextest --output test-results/analysis/mutation-workspace -- --config-file config/nextest.toml --profile mutation --fail-fast --ignore-default-filter --no-tests fail
```

Fuzz campaigns and extended soak are separate, opt-in analysis, not release-quality requirements.
Run `RUSTUP_TOOLCHAIN=nightly cargo xtask fuzz campaign` when needed; release
preparation, CI qualification, approval, and evidence retention do not require them.
Run `cargo xtask test performance --profile soak` explicitly for the extended
stress/soak campaign with 25,000 iterations per repeated phase, including its
Massif and Memcheck measurements.

The performance commands automatically capture phase/end-to-end, growth, and
concurrency timings, Massif heap snapshots, and Memcheck allocation/leak results.
Distinct composition phases cover multi-unit/vocabulary capture, graph construction,
clean compilation, reader validation, encoding/decoding, public views, all identity
partitions, warm and changed-unit cache execution, growth, and concurrency.
Cache and decoding measurements also check results against clean compilation.
Old document measurements remain separately labeled for compatibility; old-only
reports no longer satisfy the current performance gate.
`performance.maximum_ns_per_iteration` in `config/quality-gates.toml` defines
both the required phase inventory and inclusive timing ceilings. Every direct
benchmark phase must appear exactly once with positive integer iterations/time;
missing, duplicate, malformed or over-budget samples fail. Massif's largest
`mem_heap_B` sample must not exceed `performance.maximum_heap_bytes`; Memcheck
must still report no errors. Profiler-instrumented timings are not compared
against direct-run ceilings.

These initial regression envelopes have at least 10× headroom over a local
release sample (slowest repeated phase about 178 µs/iteration; growth about
468 µs/iteration; useful heap peak 618,699 bytes). They detect gross regressions,
not small slowdowns or a portable performance guarantee. Calibrate tighter
ceilings from repeated, otherwise idle runs on the intended controlled runner;
review changes to workload, compiler or hardware explicitly rather than raising
ceilings automatically. Policy edits invalidate previous measurement receipts.
The optional soak uses the benchmark's extended-soak profile. Use an otherwise
idle controlled runner; comparing measurements from different machines is not a
performance regression test.

Release preparation evaluates the committed candidate automatically. Evaluation
runs fresh RustSec checks for every declared dependency lock and verifies the
measured gates. Missing, stale, modified, incomplete, or failing reports block
release qualification; manual Markdown approval cannot substitute for them.

`cargo xtask release publish` records approval and automatically retains verified
reports under `quality/evidence/v<version>/gates/<input-fingerprint>/`. Commit
only the compact approval record and maintained status (publication does this for
you), never these raw
snapshots: they are ignored local evidence and may contain personal paths or
other host details. Existing historical approval records remain unchanged.
A fresh release runner regenerates missing measurements during `release qualify`;
the workflow installs their tools first. It does not transfer private workstation
reports or treat an approval summary as measured proof.
Advisory scans expire according to `config/automation.toml` and are rerun during
release qualification. No additional handwritten per-command evidence is needed.
