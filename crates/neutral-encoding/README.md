<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-encoding

`neutral-encoding` is the one-way external artifact producer for Neutral IR
Framed CBOR 0.1. It accepts only a `neutral-reader::ValidatedDocument`, projects
all five frozen sections, derives capability declarations from actual content,
adds envelope-only producer and integrity facts, and applies the immutable
encoding ceilings before returning bytes.

Within the ecosystem, the compiler produces in-memory artifacts, the reader
validates them, and this crate serializes that validated view. It owns neither
language meaning nor hostile-byte decoding. Stage 7 Step 3 adds decoding at the
reader boundary; byte ordering emitted here is deterministic implementation
behavior and is not logical identity.
