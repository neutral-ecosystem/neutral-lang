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
The separate v1 project-bundle validator checks exact multi-vocabulary locks,
closed public/private nominal type declarations, transitive public type
closure, inert location field types, and embedded cycles. It publishes
canonical contracts without source-local aliases or authoring metadata.

The separate `composition` API validates explicitly selected composite bundles
and their complete captured dependency closure. It checks variants, lists,
nullable/nominal references, omission policies, closed defaults and declarative
restrictions against shared `neutral-ir` contracts. It does not acquire inputs
or activate these forms in the existing compiler, project wire, or identity
profiles. Successful catalogue validation alone is not project acceptance.

## Command

Use `cargo xtask test all` for complete repository validation, including doctests.
The focused Cargo command below runs this crate's test binaries only.

This is a library crate. Verify bundle validation with:

```sh
cargo test --package neutral-vocabulary
```
