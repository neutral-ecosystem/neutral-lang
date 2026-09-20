<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-reader

`neutral-reader` validates external Neutral artifacts and exposes immutable,
typed reader views over accepted logical data.

It depends on core, IR, and vocabulary contracts. It does not acquire artifacts
or perform vocabulary lookup: callers supply the bytes and the exact captured
vocabulary contract. The CLI and standalone probe use this boundary to inspect
artifacts without depending on compiler internals.

It also exposes the shared deterministic language-profile catalogue used by
hosts for compatibility discovery. This catalogue comes from `neutral-core`,
so reader and CLI reporting cannot drift into separate version tables.

The active reader exposes validated in-process compiler artifacts through
immutable scalar, nominal-record, and qualified vocabulary traversal; recursive
type/value/default-provenance validation; exact vocabulary-contract validation;
and indexed source lookup. `neutral-encoding` owns hostile framed-byte decoding
and exposes a reader view only after the complete external artifact validates.

## Command

This is a library crate. Verify its public artifact-reading contracts with:

```sh
cargo test --package neutral-reader
```
