<!-- SPDX-License-Identifier: Apache-2.0 -->

# Project codec tests

Owns hostile project tuple/schema, companion, resource, depth, cancellation, and
byte-limit tests. Private writers forge adversarial inputs only here; production
encoding requires an independently validated project. Cross-package compiler
round-trip and view equivalence live in `neutral-test-suite`.

Run `cargo test --package neutral-encoding project`.
