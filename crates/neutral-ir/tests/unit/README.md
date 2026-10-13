<!-- SPDX-License-Identifier: Apache-2.0 -->

# Unit test ownership

[`mod.rs`](mod.rs) covers logical equality, source accounting, names, exact values
and fingerprints through the library test module. [`project_identity_framing.rs`](project_identity_framing.rs)
tests private bounded framing. [Expression tests](expression/README.md) use the
public raw-tree API through their integration-test entry point.

Use shared production selectors to construct ordinary test data. Keep literal
expected transcript/wire bytes and historical compatibility oracles independent
of the code being tested; otherwise a protocol regression can change both sides.
Run `cargo nextest run --config-file config/nextest.toml --package neutral-ir`.
