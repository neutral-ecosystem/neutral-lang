<!-- SPDX-License-Identifier: Apache-2.0 -->

# Project semantic unit tests

These tests exercise the compiler-owned project analyzer directly: visibility,
nominal compatibility, references, semantic cycles, and stable identity edges.
The criterion also covers contextual project lowering, exact-number bounds, and
private lowering guards in `lowering.rs`. Public inputs stay captured and effect-free;
private guards use in-memory tokens.
