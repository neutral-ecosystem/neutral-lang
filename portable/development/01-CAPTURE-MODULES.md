<!-- SPDX-License-Identifier: Apache-2.0 -->

# 01 — Capture and modules

Status: accepted operational plan

## Stages 1–3

This phase delivers the project boundary before any public cross-module
semantics are exposed.

### Stage 1 — `v0.1.0 -> v0.2.0`

Freeze the v1 delta, profile dispatch, resource-control model, diagnostic
families, migration policy, and fixture inventory. `neu "1.0"` must reject all
unimplemented behavior deterministically rather than falling through to v0.

### Stage 2 — `v0.2.0 -> v0.3.0`

Implement `CapturedProjectRequest` and complete no-I/O capture: exact source
bytes, logical module/source identities, exact vocabulary locks, request/header
matching, bounded closure validation, and immutable captured-project output.
Every supplied source is a project member, including disconnected modules.

### Stage 3 — `v0.3.0 -> v0.4.0`

Implement one source unit per logical module; `module` headers; mandatory,
aliased logical-module imports; deterministic graph construction; and SCC
collection. Imports neither contain paths/URLs nor perform resolution. A module
cycle is valid unless a separately defined semantic cycle is invalid.

## Exit evidence

Positive and negative multi-file request fixtures cover duplicate modules,
header/request mismatch, missing/self imports, alias collision, disconnected
members, shuffled request order, SCCs, limits, and no-I/O enforcement.
