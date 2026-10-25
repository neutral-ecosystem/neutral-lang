<!-- SPDX-License-Identifier: Apache-2.0 -->

# Nominal contract helpers

The parent `nominal.rs` owns private nominal resolver state. This directory
separates declaration/default parsing (`syntax.rs`), bounded type/value resolution
(`resolver.rs`), original capture facts (`companions.rs`), import/SCC validation
(`graph.rs`) and diagnostic codes (`codes.rs`). Complete expression compilation
is owned by the sibling `expressions` module, including cache, expansion and
formatting. Nominal helpers do not own or reexport the compiler entry points.

All modules remain private to the compiler. Preserve checks before allocation,
cooperative cancellation, fail-closed errors and transactional cache publication
when changing these phases. Do not introduce host acquisition or a second
record/variant semantic model.
