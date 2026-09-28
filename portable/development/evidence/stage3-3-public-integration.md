<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 3.3 public graph integration

Status: public integration gate complete; release promotion pending

Date: 2026-09-28  
Target gate: `v0.3.3`

`CapturedProject::module_graph` exposes the complete, deterministic module
graph from the immutable captured closure. It returns a `ModuleGraphFailure`
with no partial graph when validation fails. Public module and import facts
carry source IDs and typed `SourceLocation` values, including the captured
source-content digest and original-byte span. Source-accounted graph
diagnostics carry the same typed location; graph-wide limit failures have no
source location.

The cross-package integration test checks every module and edge of the valid
cycle against its captured source, then checks every diagnostic from the
four-source forbidden-import fixture against the corresponding source bytes.
It confirms source ID, digest, original-byte span, and bounds. A second test
edits one captured source, recaptures the closure, and compares its meaning
and graph to an independently captured, reverse-ordered clean request. The
old graph remains unchanged and the edited graph loses the expected edge.
Graph construction is always from current captured bytes; there is no graph
cache whose stale state could affect this comparison.

## Verification

```text
cargo test -p neutral-test-suite --lib stage3::
cargo clippy -p neutral-compiler -p neutral-test-suite --all-targets -- -D warnings
cargo xtask ci pr
```

The composed PR workflow passed at
`test-results/workflows/ci/pr/run-2-9`. `cargo xtask fixtures check` verified
35 fixture/oracle pairs with no manifest or freeze digest updates, and
`cargo xtask portable verify` passed traceability and package checks. The
Stage 3 executable corpus remains covered by
[Stage 3.4 validation](stage3-4-validation.md).
