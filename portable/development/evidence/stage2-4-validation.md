<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 2.4 capture validation

Status: validation gate complete; released as `v0.3.0`

Date: 2026-09-25  
Toolchain: `rustc 1.98.1 (48a229cea 2026-09-01)`

## Active suites

The Stage 2 fixture corpus is active in `neutral-test-suite`. Its executable
harness consumes all 14 frozen request fixtures and their expected outcomes,
including complete/disconnected closure, duplicate and header mismatch,
exact/missing/extra/conflicting vocabulary locks, equivalent/conflicting host
mappings, and exact/one-over source-unit bounds. The inherited v0.1 suite
continues to run in the same workspace and conformance commands.

## Failure and resource coverage

Malformed headers, every independent structural ceiling, zero limits,
integrity failures, cancellation at every checkpoint, and stable
`NEU-CAP-001` through `NEU-CAP-013` classifications are covered. A dedicated
checkpoint test proves an over-limit request envelope returns after the start
checkpoint, before immutable capture allocation or publication.

## Closure and replay review

Capture retains the complete supplied closure, including disconnected units,
and canonicalizes sources by logical module/source identity and vocabularies by
canonical vocabulary identity. A captured result replays through the same
closed schema with a fresh cancellation token and returns an exactly equal
result. No provisional closure digest or recurring evidence hash is introduced;
canonical capture meaning is tested directly, with identity hashing remaining
reserved for Stage 7.

## Validation

The following commands passed:

```text
cargo test -p neutral-test-suite
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo xtask test conformance
cargo xtask test security
cargo xtask test property
cargo xtask fixtures check
cargo xtask portable verify
cargo xtask check
cargo xtask ci pr
```

No required Stage 2 test was skipped. The composed PR workflow passed at
`test-results/workflows/ci/pr/run-2-1`.
