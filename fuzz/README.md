<!-- SPDX-License-Identifier: Apache-2.0 -->

# Coverage-guided fuzzing

This isolated `cargo-fuzz` package owns coverage-guided source, vocabulary,
external IR, formatter, and probe campaigns. It is deliberately outside the
stable production workspace: fuzzing may use a temporary nightly toolchain,
while all release code continues to build with the repository’s selected
stable compiler.

Confirmed failures must be minimized and promoted into deterministic fixtures
or regression tests before they are considered resolved.
