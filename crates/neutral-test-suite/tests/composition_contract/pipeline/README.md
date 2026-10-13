<!-- SPDX-License-Identifier: Apache-2.0 -->

# Complete composition pipeline tests

`mod.rs` owns shared capture/compile/reader helpers and source-to-artifact cases.
Sibling modules cover allocation failures, consumer envelopes, graph assembly,
cache/boundary hardening and independent reader/probe inspection. `identity/`
compares production identity layers against literal independent expectations.
Source inputs come from the parent suite's `sources` module; fixture paths are
defined once there. Rust module names and existing test filters are unchanged.

Run `cargo test -p neutral-test-suite composition_contract::pipeline` or the full
gate with `cargo xtask test all`.
