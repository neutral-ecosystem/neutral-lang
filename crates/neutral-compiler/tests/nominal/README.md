<!-- SPDX-License-Identifier: Apache-2.0 -->

# Current nominal helper tests

These focused tests exercise nominal declaration parsing,
type/default resolution and source companion projection against captures
created through the current expression request. Expression cache generation
behavior is covered in `tests/expression_source/cache.rs` and the current test
suite. Import/SCC boundaries belong to the sibling `tests/graph/` directory.
