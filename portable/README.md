<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral language v0 portable implementation package

This directory is the self-contained execution, tracking, specification, and
conformance package for the version currently implemented by this repository.
When v0 is complete, archive this directory as its immutable version record;
the next version starts from the corresponding portable package in the Neutral
roadmap and is adapted with this repository's implementation evidence.

Start with [PLAN.md](PLAN.md). The mandatory
[contract-freeze gate](development/02-CONTRACT-FREEZE.md) was approved before
production language behavior began.

The machine-readable active-series and archive destinations are in
[`lifecycle.toml`](lifecycle.toml). The verified snapshot and v0-to-v1 rollover
procedure is in
[`development/07-PORTABLE-LIFECYCLE.md`](development/07-PORTABLE-LIFECYCLE.md).

## Layout

```text
README.md                     package lifecycle and navigation
lifecycle.toml                active series and roadmap rollover destinations
ARCHITECTURE.md               normative v0 architecture
PLAN.md                       operational implementation entry point
ROADMAP.md                    product milestone summary
conformance/                  executable manifests, oracles, and review metadata
development/                  numbered lifecycle rules and retained evidence
specs/                        requirements, contracts, decisions, fixtures, examples
```

[`development/06-PROGRESS.md`](development/06-PROGRESS.md) is the
solo-maintained working log. It records only
current focus, immediate actions, blockers, and completed validation evidence;
the specifications and checklists remain acceptance criteria rather than a
second mutable status system.
