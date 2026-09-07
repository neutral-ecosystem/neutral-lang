<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 9 residual risks

Review date: 2026-09-07. Owner: maintainer. Approval state: open.

The following work prevents Stage 9 validation from being approved today:

- Coverage is now measured, but its 84.91% line, 80.46% function, and 75.55%
  region results remain below the configured 85%/90%/80% thresholds.
- All five `cargo-fuzz` targets pass bounded readiness runs, but the required
  900-second-per-target campaigns have not run. LeakSanitizer is incompatible
  with the ptrace-managed development executor, so final campaigns require an
  untraced runner.
- Local PR, release, bounded-soak, and peak-process-RSS runs pass, but a
  dedicated-runner baseline, component-level allocation measurement, and
  retained extended soak run remain outstanding.
- Mutation strength was reconfirmed at 38/38 for the configured critical
  language predicate target, but broader exact-number, layout, graph, limit,
  diagnostic, and decoder mutation review remains.
- Static review was performed by the sole maintainer and is not independent.

These are evidence gaps, not known correctness defects. Stage 10 must not begin
until the required Stage 9 gates are executed, reviewed, and this record is
closed or explicitly accepted under release policy.
