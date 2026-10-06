<!-- SPDX-License-Identifier: Apache-2.0 -->

# Composition contract oracle

Runtime-owned copies of reviewed inputs live here so tests do not depend on the
portable plan. `reference.py` is a test-only Python encoder of the frozen /2
transcript grammar. It reuses only unchanged /1 framing primitives from the
independent test oracle, never compiler, codec or production identity code.
`mod.rs` verifies literal bytes/digests, input integrity and adversarial partitions.
This checks a frozen design, not implementation availability. Ordinary tests run
with `cargo xtask test all`; focused runs use `cargo test -p neutral-test-suite
composition_contract`. Python 3 is required; missing tools fail, not skip.
