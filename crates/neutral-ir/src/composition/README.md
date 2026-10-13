<!-- SPDX-License-Identifier: Apache-2.0 -->

# Composition contracts

[`mod.rs`](mod.rs) owns the common record/variant and structural value model.
`ClosedValue` defaults cannot contain binding references; `BindingValue` uses
the same representation with exact module-symbol references. Scalar comparison
is shared by restrictions and validators, with no floating-point conversion.

[`profile.rs`](profile.rs) owns frozen selectors, feature IDs and transport/domain
constants. Package versions never select language behavior. The [project
module](project/README.md) adds complete raw data, policies, origins and preflight.
Construction or profile recognition does not imply semantic validation or activation.

Vocabulary validates contracts and defaults; semantics checks expression typing;
reader validates complete artifacts. Reuse these contracts instead of creating
consumer-specific record or variant models. Run `cargo xtask test all` for the
cross-crate pipeline, or `cargo test --package neutral-ir` for local tests/doctests.
