<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v0 conformance execution assets

This directory owns the executable conformance manifest, reviewed fixture
inventory, immutable expected-result oracles, and runner-facing assets.
Normative source and vocabulary inputs remain under
[`../specs/fixtures/`](../specs/fixtures/README.md) and must not be duplicated
here.

Within the ecosystem, `manifest.toml` is the sole fixture-discovery and oracle
index. `fixture-oracle-review.toml` records review state and fixture hashes,
while `oracles/` freezes accepted public IR observations or exact rejection
contracts for each active vertical slice.

Generated reports do not belong in this directory. Automation writes them to
the ignored stage/profile hierarchy under the repository's `test-results/`.
