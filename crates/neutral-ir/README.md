<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-ir

`neutral-ir` owns Neutral's public logical intermediate-representation and
source-accounting contracts. It is where accepted program meaning is described
independently of source spelling, storage, or external encoding. Its shared
`language` namespace is the single source for frozen core spellings and
identifier predicates needed by producers and hostile readers.

It depends only on `neutral-core`. The compiler produces this logical model;
the reader validates and exposes it; the vocabulary layer refers to it; and the
probe observes it through public reader contracts. It must not acquire input or
perform host effects. Its active scalar model carries exact numbers, decoded
Unicode strings, Booleans, recursive outer-nullable type identity, and explicit
typed null, module-owned nominal record schemas, and recursively typed
contextual record values; display is deterministic and escapes hostile controls.
It also carries exact captured vocabulary identity/version/schema/encoding/
digest/feature facts, qualified type contracts and values, and distinct
vocabulary-default provenance without introducing an executable value kind.
For project development, `project_interface` is the shared public contract
used by the compiler producer, independent reader validator, and probe
consumer. It carries only exported signatures and public-to-public dependency
edges. Its source-profile version comes from `neutral-core`; the Cargo package
version comes from the workspace manifest. Neither is duplicated in this
interface. The domain-separated fingerprint covers that signature surface,
not private values or complete project identity.
Project signatures now distinguish canonical locked vocabulary nominals and
inert `url`/`path` types. `ProjectLocationValue` retains decoded source text in
separate URL and path variants without interpretation or host authority.

The `project` module owns complete project IR separately from that narrow
signature snapshot. It retains every public/private declaration and supplied
module, canonical locked vocabulary schemas, typed contextual values and closed
defaults, plus separate original-byte source maps, dependency provenance, exact
captured input evidence, explicit limits, and resource facts. `logical_eq`
excludes host/source and processing evidence. `ViewRequest` is a distinct
post-compilation selection contract; it cannot change complete project meaning.
The `project_identity` module owns separately typed captured-closure, complete
logical-project, derivation, and artifact identities. Its versioned NHT/SHA-256
transcripts include private/disconnected meaning while excluding host mappings,
source evidence, aliases, roots, and processing context from the logical layer.
Construction uses independent byte/node/depth bounds and cancellation. Semantic
reader validation remains mandatory: an identity is not an attestation.
Complete encoded
transport is owned by `neutral-encoding`, which retains explicit artifact-byte
limits separately from logical content and materialized-value work.

Start with [lib.rs](src/lib.rs) for value/identity distinctions and exact-number
normalization, then [project.rs](src/project.rs) for meaning versus companions.
The [composition model](src/composition.rs) shares raw composite vocabulary
contracts and closed defaults with validators and future readers. It preserves
absence/null distinctions and exact numeric comparison; construction does not
establish validity or extend frozen project schemas.
`SourceCompositionDefinition` associates that same record/variant body with an
exact module-symbol owner; it does not introduce a second variant model or
confer source/compiler validity merely through construction.
`CompositionValue<R>` shares scalar/list/record/variant structure between
`ClosedValue` defaults and `BindingValue` identity references. A closed default's
reference parameter is uninhabited; the type system prevents a binding target
from being assigned as a default. `CompositionBinding` and its occurrence paths
are raw resolved-model contracts, not a new complete project or wire schema.
Its [profile selectors](src/composition/profile.rs) are shared across capture,
vocabulary and consumers. `captured_composition_closure` constructs a separately
typed successor capture identity using the same bounded framing machinery;
existing identity transcripts stay unchanged.
`composition::project::CompositionProjectIr` retains complete resolved source
and vocabulary declarations, materialized bindings, origin paths, companions
and independently recomputable resource facts. Iterative structural inspection
bounds raw data before recursive consumers clone or interpret it; construction
still confers no validation authority.
`canonical_composition_project` and `composition_interface` use separately
selected bounded logical/interface framing. Complete logical bytes and digest
match the frozen independent vector. The independent reader checks occurrence
attribution and derives redacted public closure; the standalone probe consumes
that projection. `CompositionDerivationIdentity` binds complete logical/captured
meaning, explicit producer facts and every capture/project/composition control;
`CompositionArtifactIdentity` additionally binds kind, format, options and roots.
Both use separately typed /2 transcripts; /1 profiles and vectors are unchanged.
The [identity writer](src/project_identity/framing.rs) explains bounded in-place
framing and why malformed canonical order is rejected instead of silently sorted.

## Command

Use `cargo xtask test all` for complete repository validation, including doctests.
The focused Cargo command below runs this crate's test binaries only.

This is a library crate. Verify its public logical-model contracts with:

```sh
cargo test --package neutral-ir
```
