<!-- SPDX-License-Identifier: Apache-2.0 -->

# Identity comparisons

`mod.rs` compares complete captured/logical/interface/derivation/artifact identity
partitions, exclusions and rejection boundaries with frozen independent vectors.
`facts.rs` assembles test models from literal facts without normalizing expectations;
it is test scaffolding, not production reader authority or an expected-value generator.

Run `cargo test -p neutral-test-suite composition_contract::pipeline::identity`.
