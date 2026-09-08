<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 9 product-quality evaluation

Evaluation date: 2026-09-07. Owner: maintainer.

| Characteristic | Method/evidence | Current conclusion |
| --- | --- | --- |
| Functional suitability | Frozen conformance manifest, logical equality, traceability | Pass |
| Performance efficiency | Five direct release samples, release/extended-soak phase, growth, concurrency, peak-RSS, Valgrind Massif, and Memcheck | Pass on the measured runner; Stage 10 establishes release regression thresholds |
| Compatibility | Version/capability contracts and independent probe | Pass for declared v0 contracts |
| Interaction capability | CLI/process tests and source-linked diagnostics | Pass on declared host matrix |
| Reliability | Repeated/concurrent determinism, cancellation checkpoints, atomic output | Pass |
| Security | Threat model, hostile suites, limits, RustSec and effect-boundary review, five clean 900-second fuzz campaigns | Pass for reviewed v0 boundaries |
| Maintainability | Crate boundaries, rustdoc, no inline tests, Clippy, full coverage gate, and broader mutation review | Pass for reviewed source quality gates |
| Flexibility | Explicit capture, reader, encoding, probe, and host boundaries | Pass |
| Safety | Fail-closed results, no partial authority, no unsafe/native code | Pass for reviewed paths |

The selected numeric thresholds, tools, and pending states are recorded in
`config/quality-gates.toml`. An indeterminate characteristic is never treated as
a pass.
