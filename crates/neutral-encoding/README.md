<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-encoding

The crate exposes the complete expression-project `/3` codec. The decoder
accepts only its exact magic and tuple profile. Encoding and decoding share
bounded CBOR parsing, caller limits, safe error classes, cancellation handling,
and fallible allocation behavior.

The codec does not compile source, evaluate expressions, acquire vocabularies,
or perform host I/O. Decoding returns authority only after the independent
reader validates the complete reconstructed project.

`encode_expression_project` and `decode_expression_project` transport
expression trees, templates, matrices, vocabularies, and source evidence.
Frozen literal vectors and hostile-input checks live in `tests/expression/`.

Run `cargo test -p neutral-encoding` and
`cargo clippy -p neutral-encoding --all-targets -- -D warnings` to verify the
codec.
