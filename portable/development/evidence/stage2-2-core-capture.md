<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 2.2 core capture validation

Status: core implementation gate complete; public integration pending

Date: 2026-09-24  
Toolchain: `rustc 1.98.1 (48a229cea 2026-09-01)`  
Implementation baseline: `7f08782c98c99cdf0615d77858581c360bf58d04`

## Bounded immutable capture

`neutral-compiler` now owns a closed `neutral.capture/v1`
`CapturedProjectRequest` and an effect-free `capture_project` boundary. The
request carries only an exact v1 profile, optional bounded correlation key,
complete source and vocabulary values, explicit independent limits, and a
cooperative cancellation token.

Capture validates every nonzero limit, collection and byte ceilings with
checked arithmetic, unique source/module identities, exact source digests,
profile/module header agreement, and all cancellation checkpoints before
publishing. Successful output freezes bytes behind immutable shared slices,
omits the mutable cancellation token and non-semantic project key, and orders
sources by `(module ID, source ID)`.

## Supplied closure and vocabulary locks

The core retains every supplied source, including disconnected units; it has
no capture root and performs no reachability pruning. Tests retain an orphan
module and prove canonical ordering.

Header requirements are collected across the complete supplied set. The
supplied vocabulary identities must be exactly equal to that requirement set.
Missing, extra, duplicate/conflicting-identity, and content-digest-mismatched
inputs fail without a partial project. Captured vocabularies are ordered by
canonical identity.

## Resolver and effect boundary

The request type contains no resolver, callback, path, URL, root, credential,
or acquisition field. A compile-fail documentation test proves a resolver
callback cannot be attached to the request API. `cargo xtask check` audits the
normal compiler dependency closure and rejects filesystem, environment,
network, process, command, and clock APIs from core, IR, vocabulary, and
compiler source.

## Retained validation

The following commands passed:

```text
cargo test -p neutral-compiler
cargo clippy -p neutral-compiler --all-targets --all-features -- -D warnings
cargo xtask check
cargo test --workspace
```

The compiler suite includes exact/one-over source-unit limits, all-limit
nonzero enforcement, the complete `NEU-CAP-001` through `NEU-CAP-013` code
catalogue, request/header mismatches, duplicate identities, integrity failures,
closure retention, exact lock coverage, cancellation, immutability, and the
resolver compile-fail case.

## Deferred work

CLI and Editor-host construction, fixture-schema execution through a shared
host adapter, shuffled-order equivalence at the public integration layer, and
full Stage 2 qualification remain assigned to `v0.2.3` and `v0.2.4`.
