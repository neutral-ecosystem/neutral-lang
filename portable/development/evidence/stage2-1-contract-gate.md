<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 2.1 captured-project contract gate

Status: accepted; implementation pending

Date: 2026-09-20  
Package release during review: `0.1.0`  
Target gate: `v0.2.1`

## Frozen contract

The reviewed [captured project request contract](../../specs/contracts/CAPTURE-REQUEST.md)
defines the closed `neutral.capture/v1` envelope, exact source and vocabulary
inputs, complete processing controls, independent structural limits,
cancellation checkpoints, fail-closed result envelope, `NEU-CAP` diagnostic
family, and the separate `NEU-HOST-001` adapter failure.

The contract distinguishes logical source identity, logical module identity,
exact byte identity, and future captured-closure identity. It excludes project
keys, host locations/mappings, input order, and allocation order from capture
meaning. It reserves the public hash transcript for Stage 7 instead of creating
a temporary identity algorithm.

## Reviewed fixture inventory

The manifest pins 14 Stage 2 request fixtures and six oracle files by SHA-256:

- complete two-unit capture and retention of a disconnected third unit;
- duplicate source ID and duplicate module identity;
- request/header profile mismatch and module mismatch;
- exact, missing, extra, and conflicting vocabulary locks;
- equivalent host mappings and a conflicting host mapping rejected before
  capture; and
- two supplied units against an exact limit of one.

All request fixtures include complete explicit processing controls. Fixture
paths and host locations are harness data only and are not request fields or
acquisition instructions.

## Gate verification

The following checks passed:

```text
cargo xtask portable verify
cargo xtask check
git diff --check
cargo xtask ci pr
```

The freeze manifest pins the capture contract, conformance manifest, and
fixture/oracle review. Stage 2.2 must implement these accepted outcomes and
activate executable tests; no fixture in this gate is counted as passing
implementation evidence.

The final repository-wide CI composition passed at
`test-results/workflows/ci/pr/run-2-9`.

## Deliberately open work

- Stage 1 release promotion remains owner-controlled.
- `CapturedProjectRequest` and immutable project capture are not implemented.
- No resolver/no-I/O proof, shuffled-order equivalence, malformed-input,
  cancellation, or full limit suite is claimed yet.
- Import graph closure and SCC processing remain Stage 3 responsibilities.
