<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-vocabulary

`neutral-vocabulary` defines the closed logical schema for captured Neutral
vocabularies and validates their bundles strictly.

It depends only on `neutral-core` and `neutral-ir`. During compilation and
artifact reading it turns already-captured vocabulary facts into validated,
immutable contracts; it never resolves names from the filesystem or network.
It cannot extend Neutral core syntax or semantics.
