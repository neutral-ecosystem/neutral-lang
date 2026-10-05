<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v1 portable architecture target

Status: accepted planning baseline

Neutral v1 is a bounded, typed, immutable, effect-free project language. It
extends the working v0.1 implementation with a complete captured module set,
explicit public APIs, exact data-only vocabulary locks, project-level IR, and a
separately versioned authoring bridge.

```text
host acquisition and resolution
    -> complete CapturedProjectRequest
    -> no-I/O capture and semantic compilation
    -> validated project Neutral IR
    -> reader views / authoring services / external consumers
```

Logical module names, not paths, identify source units. Imports are explicit,
aliased, and resolved only within the supplied closure. Import cycles are
processed as SCCs; semantic cycles remain invalid where the language says they
are. Roots are consumer views, never part of capture or logical equality.

`url` and `path` are inert typed values. They may be carried through Neutral
and interpreted by a vocabulary, but never select, fetch, open, or authorize an
input. External vocabulary/convention consumers own mapper and execution
behavior. An authoring client consumes the authoring bridge and generates
ordinary source, while the compiler remains authoritative.

The scheduled [consumer-readiness supplement](specs/contracts/CONSUMER-READINESS.md)
extends vocabulary composition before `v0.8.0` and data-only consumer/authoring
interfaces before `v0.9.0`. It preserves the no-I/O, effect-free boundary:
operation descriptions, conditions, deferred outputs, symbolic handles, and
capability reports are typed data, not built-in scheduling or execution.

The staged implementation path is [PLAN.md](PLAN.md).
