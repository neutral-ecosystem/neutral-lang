<!-- SPDX-License-Identifier: Apache-2.0 -->

# Capture, project IR, identity, reader, and view contract

Status: accepted portable baseline

## Capture

The host acquires/resolves all inputs before calling Neutral and submits a
versioned `CapturedProjectRequest` containing an optional non-semantic project
key, one exact core profile, source units with logical module/source identities
and bytes, and exact vocabulary semantic locks. It contains no roots, host
paths/URLs, credentials, package state, compiler options, output policy, or
authoring metadata.

Capture validates request/header agreement, uniqueness, complete imports,
exact vocabulary-lock coverage, and resource limits, then freezes an immutable
captured project. Every supplied source is a project member, even disconnected
ones. Capture and semantic compilation perform no external I/O and accept no
resolver/acquisition callback.

## IR and reader

Successful compilation produces complete project IR: modules/import graph,
types, symbols, values, identity edges, public export index, vocabulary facts,
source-map/provenance companions, derivation/resource facts, and diagnostics.
Private content needed for validation remains in complete IR; public readers
receive only valid public dependency closures. A reader validates public
artifacts without parsing source or using compiler-private models.

`ViewRequest` is a post-compilation operation. Its selected roots and format
options produce an artifact-specific view; they do not alter capture, complete
IR, logical equality, or logical project identity.

## Identity chain

```text
supplied source closure -> captured closure identity
canonical logical project -> logical project identity
logical project + derivation context -> derivation identity
derivation + artifact kind/format -> artifact identity
```

Logical identity uses bounded canonical logical form and versioned SHA-256
transcripts. Host paths/mappings, source evidence, capture order, aliases,
roots, and derivation options are not logical meaning. Conflicting mappings for
one logical module fail; equivalent canonical logical content may yield the
same logical project identity from different hosts.
