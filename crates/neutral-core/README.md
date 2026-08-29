<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-core

`neutral-core` is the foundation shared by every production-facing Neutral
component. It owns source identity, spans, diagnostics, structural limits, and
cancellation contracts.

The active structural limits bound source and diagnostics, decoded strings,
exact-number digits and scale, root declarations, record fields, and contextual
record nesting before semantic work expands those structures.

It sits at the bottom of the dependency graph. Its reviewed `sha2` dependency
implements the frozen exact-byte SHA-256 digest contract; it otherwise must not
depend on compiler, reader, CLI, host, automation, or test packages, and it
must not perform host I/O. Higher layers use these stable value contracts to
communicate without coupling to a particular source parser or artifact encoding.
