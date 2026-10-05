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
The [identity writer](src/project_identity/framing.rs) explains bounded in-place
framing and why malformed canonical order is rejected instead of silently sorted.

## Command

This is a library crate. Verify its public logical-model contracts with:

```sh
cargo test --package neutral-ir
```
