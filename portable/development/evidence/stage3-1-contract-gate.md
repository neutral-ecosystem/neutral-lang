<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 3.1 module graph contract gate

Status: accepted contract and fixture gate; implementation pending

Date: 2026-09-28  
Target gate: `v0.3.1`

## Frozen decisions

The reviewed [module graph contract](../../specs/contracts/MODULE-GRAPH.md)
defines the ASCII qualified module grammar, exact one-unit identity, mandatory
aliased imports, shared alias namespace, complete supplied-set resolution,
deterministic graph and SCC order, resource bounds, and `NEU-MOD-001` through
`NEU-MOD-008` diagnostics. It retains the Stage 2 closed request schema and
uses its existing per-module import, total-edge, SCC, and source-unit limits.
Condensation depth is bounded by source units; no new request field is added.

## Reviewed fixture inventory

The manifest pins 13 Stage 3 request fixtures and three outcome-oracle files by
SHA-256. The inventory covers a valid two-module import cycle with a
disconnected unit; missing, self, duplicate, alias-collision, and forbidden
import forms; and exact/one-over boundaries for per-module imports, total
edges, and SCC membership. A second cyclic graph contains an invalid semantic
value dependency: Stage 3 must accept its import SCC, while Stage 4 reserves
the semantic rejection as `NEU-XMOD-001`.

The one-unit-per-module failure is already pinned by the Stage 2
`V1-CAP-002-DUPLICATE-MODULE` fixture and remains an inherited gate.

Every fixture has a complete explicit captured-project control table and exact
source bytes. A structural review confirmed that all 13 fixtures parse as
TOML, their source headers match their requested modules, each case has one
oracle result, and the registered hashes match the file bytes.

## Gate verification

```text
cargo xtask fixtures check
cargo xtask portable verify
cargo xtask check
git diff --check
cargo xtask ci pr
```

These checks validate the frozen inventory and inherited implementation. They
do not count Stage 3 graph cases as passing executable conformance tests.
Stage 3.2 must implement the parser, graph, SCC, and diagnostics before the
suite can be activated.

The composed PR workflow passed at `test-results/workflows/ci/pr/run-2-3`.
