<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v1 portable implementation seed

Status: accepted release-train baseline

This portable package is the operational release plan for evolving the working
Neutral implementation from `v0.1.0` to `v1.0.0`. It adds project-scale
language infrastructure; it does not recreate v0 or introduce Flow execution,
Editor UX, acquisition, or vocabulary-specific behavior.

The release train and its non-negotiable gates are in [PLAN.md](PLAN.md).
[ARCHITECTURE.md](ARCHITECTURE.md) states the target boundaries, and
[ROADMAP.md](ROADMAP.md) provides the milestone view.

This is intentionally not a claim that v1 is implemented or conformant. The
portable requirements, contracts, decisions, development pipeline, and
conformance activation manifest are present. Exact fixture bytes, diagnostic
oracles, identity vectors, and execution evidence are stage-owned release
assets and must be complete before `v1.0.0`.
