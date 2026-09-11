<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 9 residual-risk review

Review date: 2026-09-08. Owner: maintainer. Approval state: approved by the
sole maintainer for Stage 10 candidate preparation; final release approval
remains subject to the remaining Stage 10 gates.

Completed and remaining Stage 9 review items are recorded below:

- Coverage passes at 90.57% lines, 90.71% functions, and 81.72% regions
  against its 85%/90%/80% thresholds; this item is closed.
- All five required 900-second fuzz campaigns completed cleanly; this item is
  closed.
- Release and 50,000-iteration extended-soak baselines completed with retained
  peak-RSS evidence. Valgrind Massif reports a 524,640 B total peak for both
  profiles, and Memcheck reports zero errors and no definite, indirect, or
  possible leaks. This item is closed.
- Mutation is closed: 38/38 caught for the configured critical target and 244
  caught, 27 unviable, and no missed viable mutant in the broader 271-mutant
  review.
- Static review was performed by the sole maintainer and is not independent.

There are no remaining technical evidence gaps. The sole-maintainer review is
not independent; the maintainer accepted that staffing limitation and approved
the residual-risk treatment for Stage 10 candidate preparation on 2026-09-08.
That approval does not waive any remaining Stage 10 qualification or final
release-approval gate.
