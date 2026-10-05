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

Use `cargo xtask test all` for complete repository validation, including doctests.
The focused Cargo command below runs this crate's test binaries only.

This is a library crate. Verify its encoder and hostile-input decoder with:

```sh
cargo test --package neutral-encoding
```

## Complete projects

`project::encode_project` accepts only a `ValidatedProject` and explicit
cancellation. `project::decode_project` independently validates complete IR,
source/provenance companions, resources, and the derived public export index
under explicit wire and project bounds. The separate `NIR-PROJECT-CBOR/1`
transport retains private validation content but never root-prunes a project.
It enforces captured artifact-byte limits independently of value work.
Package versions, byte order, and public-interface fingerprints are not complete
project identity. Neither codec links compiler-private models or performs I/O.
