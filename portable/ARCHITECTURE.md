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
input. Flow is an external vocabulary/convention consumer; it owns mapper and
execution behavior. The Editor consumes the authoring bridge and generates
ordinary source, while the compiler remains authoritative.

The staged implementation path is [PLAN.md](PLAN.md).
