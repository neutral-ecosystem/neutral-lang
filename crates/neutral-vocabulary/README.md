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
`materialize_composition_value` reuses the same validator for supplied closed
data and keeps safe supplied/null/absent/default origin facts separate from
materialized meaning. It cannot introduce source-reference targets or effects.

`validate_composition_scope` checks already resolved source record/variant
contracts against that exact catalogue, using the same closure, default,
restriction and materialization rules for both nominal origins. Its immutable
scope applies occurrence-based depth limits and rejects private/dangling types
and embedded cycles. It is not source parsing, import checking or complete
project compilation; the explicitly selected successor compiler uses this scope
instead of maintaining a second default/restriction engine.

`validate_composition_bindings` checks a complete resolved binding request over
that scope. It materializes constraints/defaults with the same value engine,
checks invariant reference targets (including forward and non-embedding cyclic
references), enforces public target closure and retains actual reference paths
separately from meaning. Budgets span the whole request, including unused
defaults. This API does not parse source, establish imports or publish encoded IR.

`validate_composition_model` independently checks canonical catalogue contracts
restored from complete IR: owners, exact dependencies, schema/features, public
closure, normalized choices/defaults and bounds. The reader uses it before
accepting decoded project data. It does not trust a producer's validation claims,
acquire locks or reinterpret old-schema bundles as new ones.

## Command

Use `cargo xtask test all` for complete repository validation, including doctests.
The focused Cargo command below runs this crate's test binaries only.

This is a library crate. Verify bundle validation with:

```sh
cargo test --package neutral-vocabulary
```
