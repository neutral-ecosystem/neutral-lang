<!-- SPDX-License-Identifier: Apache-2.0 -->

# Core unit module

This path-based module exercises foundational value invariants while retaining
private access without embedding test bodies in `src/`.

`allocation.rs` tests fallible nested copies, constituent failure and cross-thread
single-drop shared ownership. `ordered.rs` checks sorted replacement, entry,
membership and set semantics. These do not simulate global allocator exhaustion.
