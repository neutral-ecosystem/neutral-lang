<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-encoding

`neutral-encoding` owns the external Neutral IR Framed CBOR 0.1 byte boundary.
Its encoder accepts only a `neutral-reader::ValidatedDocument`, projects all
five frozen sections, derives capability declarations from actual content,
adds envelope-only producer and integrity facts, and applies the immutable
encoding ceilings before returning bytes.

Within the ecosystem, the compiler produces in-memory artifacts, the reader
validates them, and this crate serializes or reconstructs that validated view.
The decoder treats the frame and restricted CBOR as hostile: it checks lengths,
versions, capabilities, integrity, closed schemas, limits, logical data, source
maps, provenance, derivation, and captured vocabulary identity before exposing
an immutable reader view. It performs no vocabulary lookup or host I/O.

The crate owns encoding mechanics, stable decode failure classes, and
bounds-before-allocation enforcement. It does not own language meaning, mutate
validated documents, or make emitted byte ordering part of logical identity.

## Command

This is a library crate. Verify its encoder and hostile-input decoder with:

```sh
cargo test --package neutral-encoding
```
