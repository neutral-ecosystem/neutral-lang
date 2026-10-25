<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-ir

`neutral-ir` owns shared scalar, nominal, source and raw `/3` expression IR
contracts. The complete current project model is
`expression::project::ExpressionProjectIr`; its fields keep source and
processing companions separate from logical meaning. Construction does not
validate an artifact. The independent reader validates complete artifacts
before exposing authority.

The crate also provides shared closed composition values, source/vocabulary
record and variant contracts, project limits, resource facts, and bounded raw
retention inspection. These helpers support the current expression producer
and reader; they do not construct a complete composition-project model. The
`project_interface` module owns exact public type contracts, while
`project_identity` owns bounded `/3` captured, logical, interface, derivation
and artifact transcripts. Frozen identity vectors are kept with the identity
tests and remain byte-for-byte stable.

The former `LogicalDocument`, `CompilationArtifacts`, `project::ProjectIr`,
and `composition::project::CompositionProjectIr` complete models and their
`/1` and `/2` identity entry points have been removed. Shared scalar, type,
value, nominal, source, provenance, location, framing and composition-contract
primitives remain available.

Run crate tests with:

```sh
cargo test -p neutral-ir
```
