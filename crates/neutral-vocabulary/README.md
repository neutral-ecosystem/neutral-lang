<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-vocabulary

`neutral-vocabulary` defines the closed logical schema for captured Neutral
vocabularies and validates their bundles strictly.

It depends only on `neutral-core` and `neutral-ir`. During compilation and
artifact reading it turns already-captured vocabulary facts into validated,
immutable contracts; it never resolves names from the filesystem or network.
It cannot extend Neutral core syntax or semantics.

Its API verifies the exact typed SHA-256 lock before parsing, decodes
strict bounded UTF-8 JSON without map collapse, validates the closed envelope,
features, names, type graph, and contextual defaults, then exposes separate
captured-byte and normalized logical projections. Bundle content is always data:
scripts, callbacks, validators, bytecode, native modules, and unknown shapes are
rejected rather than interpreted.
The compiler maps this validated contract into public IR; source syntax never
calls this crate to search a registry, path, cache, or network.

## Command

This is a library crate. Verify bundle validation with:

```sh
cargo test --package neutral-vocabulary
```
