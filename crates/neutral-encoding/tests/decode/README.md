<!-- SPDX-License-Identifier: Apache-2.0 -->

# Decoder boundary tests

This directory owns crate-local tests for private fixed-frame validation and
logical schema budgets. It isolates outer framing and resource policy from CBOR
and logical reconstruction so every exact boundary is observable.
