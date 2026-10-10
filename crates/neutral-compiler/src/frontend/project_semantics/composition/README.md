<!-- SPDX-License-Identifier: Apache-2.0 -->

# Composition compilation internals

The parent `composition.rs` owns phase ordering, private request state and atomic
publication. This directory separates captured-byte parsing (`syntax.rs`),
bounded type/value resolution and expansion accounting (`resolver.rs`), and
complete IR/companion assembly (`assembly.rs`). Existing sibling cache, graph
and attribution modules retain their specialized responsibilities.

All modules remain private to the compiler. Preserve checks before allocation,
cooperative cancellation, fail-closed errors and transactional cache publication
when changing these phases. Do not introduce host acquisition or a second
record/variant semantic model.
