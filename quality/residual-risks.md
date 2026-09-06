<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 9 residual risks

Review date: 2026-09-06. Owner: maintainer. Approval state: open.

The following work prevents Stage 9 validation from being approved today:

- `cargo-llvm-cov` is installed, but the selected stable compiler environment
  lacks `llvm-tools-preview` and has no `rustup`; configured coverage thresholds
  therefore have no retained measurement.
- Five `cargo-fuzz` targets exist, but no compatible temporary nightly/libFuzzer
  toolchain is present, so the required coverage-guided time budget has not run.
- Local PR, release, bounded-soak, and peak-process-RSS runs pass, but a
  dedicated-runner baseline, component-level allocation measurement, and
  retained extended soak run remain outstanding.
- Mutation strength is proven for the 38-mutant critical language predicate
  target, not yet for every additional exact-number, layout, graph, limit,
  diagnostic, and decoder target named by the testing plan.
- Static review was performed by the sole maintainer and is not independent.

These are evidence gaps, not known correctness defects. Stage 10 must not begin
until the required Stage 9 gates are executed, reviewed, and this record is
closed or explicitly accepted under release policy.
