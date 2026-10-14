<!-- SPDX-License-Identifier: Apache-2.0 -->

# Explicit pure evaluation

`mod.rs` owns closed request/results and shared request state. `bindings.rs`
validates the complete supplied envelope before demands; `selectors.rs` checks
actual output-root availability; `keys.rs` orders missing keys. `walk.rs` visits
selected paths, using the private decimal/value operations and shared budget.

`normalization.rs` reuses the kernel without external bindings for producer
folding. `folding.rs` normalizes unconditional operands while preserving deferred
guards and closing eliminated match frames without capturing local binders.
Residual results are owned, bounded data; compiler integration must recompute
facts before independently validating and publishing a normalized project.

No callback, ambient acquisition, scheduler or backend execution lives here.
Semantic model authority does not replace reader companion/identity validation.
