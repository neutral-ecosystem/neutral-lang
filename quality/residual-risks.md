<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 9 residual risks

Review date: 2026-09-07. Owner: maintainer. Approval state: open.

The following work prevents Stage 9 validation from being approved today:

- The official whole-workspace coverage gate remains below its configured
  85%/90%/80% line/function/region thresholds. A production-only diagnostic
  run reached 92.99%/89.15%/83.92%, but it still misses the function threshold
  and is not a substitute for the configured gate.
- All five required 900-second fuzz campaigns completed cleanly on an untraced
  runner. This item is closed.
- Release and 50,000-iteration extended-soak baselines completed with retained
  peak-RSS evidence, but component-level allocation accounting remains
  unavailable because no supported allocation profiler is installed and the
  repository forbids an unsafe replacement global allocator.
- Mutation strength remains 38/38 for the configured critical language target.
  The broader 271-mutant review caught 178, missed 66, and found 27 unviable
  mutants; exact-number and decoder-boundary tests need further strengthening
  before this review can close.
- Static review was performed by the sole maintainer and is not independent.

These are evidence gaps, not known correctness defects. Stage 10 must not begin
until the required Stage 9 gates are executed, reviewed, and this record is
closed or explicitly accepted under release policy.
