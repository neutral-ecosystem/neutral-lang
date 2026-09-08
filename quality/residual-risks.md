<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 9 residual risks

Review date: 2026-09-08. Owner: maintainer. Approval state: open.

Completed and remaining Stage 9 review items are recorded below:

- Coverage passes at 90.57% lines, 90.71% functions, and 81.72% regions
  against its 85%/90%/80% thresholds; this item is closed.
- All five required 900-second fuzz campaigns completed cleanly; this item is
  closed.
- Release and 50,000-iteration extended-soak baselines completed with retained
  peak-RSS evidence, but component-level allocation accounting remains
  unavailable because no supported allocation profiler is installed and the
  repository forbids an unsafe replacement global allocator.
- Mutation is closed: 38/38 caught for the configured critical target and 244
  caught, 27 unviable, and no missed viable mutant in the broader 271-mutant
  review.
- Static review was performed by the sole maintainer and is not independent.

These are evidence gaps, not known correctness defects. Stage 10 must not begin
until the required Stage 9 gates are executed, reviewed, and this record is
closed or explicitly accepted under release policy.
