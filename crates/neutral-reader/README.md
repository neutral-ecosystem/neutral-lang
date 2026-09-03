<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-reader

`neutral-reader` validates external Neutral artifacts and exposes immutable,
typed reader views over accepted logical data.

It depends on core, IR, and vocabulary contracts. It does not acquire artifacts
or perform vocabulary lookup: callers supply the bytes and the exact captured
vocabulary contract. The CLI and standalone probe use this boundary to inspect
artifacts without depending on compiler internals.

The active reader exposes validated in-process compiler artifacts through
immutable scalar, nominal-record, and qualified vocabulary traversal; recursive
type/value/default-provenance validation; exact vocabulary-contract validation;
and indexed source lookup. Hostile external decoding is introduced by its later
owning stage.
