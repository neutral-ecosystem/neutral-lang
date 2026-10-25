<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-core

`neutral-core` is the foundation shared by every production-facing Neutral
component. It owns source identity, spans, diagnostics, structural limits,
cancellation, and language-profile discovery contracts.

The active structural limits bound source and diagnostics, decoded strings,
exact-number digits and scale, root declarations, record fields, and contextual
record nesting before semantic work expands those structures.

The profile catalogue reports the sole current development source profile,
stable capabilities and centralized default limits; unsupported profiles reject.

It sits at the bottom of the dependency graph. Its reviewed `sha2` dependency
implements the frozen exact-byte SHA-256 digest contract. The allocation boundary
uses reviewed `trybox` and std-only `triomphe` backends for fallible boxes and
shared ownership; it otherwise must not
depend on compiler, reader, CLI, host, automation, or test packages, and it
must not perform host I/O. Higher layers use these stable value contracts to
communicate without coupling to a particular source parser or artifact encoding.

For code orientation, [lib.rs](src/lib.rs) preserves the root exports.
[digest.rs](src/digest.rs) owns exact-byte hashes and transcript framing;
[source.rs](src/source.rs) owns original-byte coordinates;
[diagnostics.rs](src/diagnostics.rs) owns diagnostic ordering;
[limits.rs](src/limits.rs) owns structural ceilings; and
[cancellation.rs](src/cancellation.rs) owns cooperative cancellation;
[error.rs](src/error.rs) owns constructor failures and result classes.
[profile.rs](src/profile.rs) owns discovery and defaults. These are
primitives; they do not establish that a whole program or external artifact is valid.

[allocation.rs](src/allocation.rs) provides `Shared::try_new`, fallible owned
copies and boxes. Successor APIs use `Shared<T>` rather than `std::sync::Arc<T>`;
cloning a shared owner retains the existing allocation. Recursive copies require
caller depth/work preflight. [ordered.rs](src/ordered.rs) provides fallibly growing
ordered indexes; callers must budget insertion/removal shifts, not just key lookup.
These primitives do not make an entire pipeline allocation-safe automatically.

`RetainCapacity` checks vector/string reservations through the same boundary.
The non-default `allocation-testing` feature exposes thread-local failure and
cancellation scopes for cross-package tests. It is not a global allocator
replacement and is disabled in ordinary production dependency builds.

## Command

Use `cargo xtask test all` for complete repository validation, including doctests.
The focused Cargo command below runs this crate's test binaries only.

This is a library crate. Verify its foundational contracts with:

```sh
cargo test --package neutral-core
```
