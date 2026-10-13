<!-- SPDX-License-Identifier: Apache-2.0 -->

# Bounded expression retention

[`mod.rs`](mod.rs) implements `Expression::try_clone_bounded`: preflight the entire
tree before retention, then copy every branch with fallible storage and cooperative
cancellation. [`value.rs`](value.rs) copies deep literals with per-child cancellation
checks instead of an opaque whole-value clone.

Private copy helpers require successful preflight. Keep them private: exporting
an unguarded recursive `Clone`/`TryClone` would bypass depth/storage checks. A failed
or cancelled copy returns no partial tree and does not modify the original.
Copying preserves syntax-level structure; it never folds or chooses branches.

Run `cargo nextest run --config-file config/nextest.toml --package neutral-ir
--test expression -E 'test(copy)'` for retention and allocation-failure tests.
