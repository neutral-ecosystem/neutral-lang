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

## Command

This is a library crate. Verify its public logical-model contracts with:

```sh
cargo test --package neutral-ir
```
