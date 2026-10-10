<!-- SPDX-License-Identifier: Apache-2.0 -->

# Cross-package feature regressions

`mod.rs` owns shared imports, fixture references and test helpers. Feature modules
exercise numbers, nullable values, records/defaults, lists, reuse, references,
syntax, scalars, vocabulary, determinism, formatting and encoded artifacts.
`hostile_decoder.rs` retains bounded hostile-input and deterministic mutation cases.

Keep fixture inputs in the owning conformance corpus rather than duplicating
them here. Test function category prefixes (`conformance_`, `security_`, etc.)
remain stable because repository test selection uses them; module names describe
feature ownership, not implementation stages.

Run `cargo test --package neutral-test-suite` for this crate, or
`cargo xtask test all` for the complete workspace and doctests.
