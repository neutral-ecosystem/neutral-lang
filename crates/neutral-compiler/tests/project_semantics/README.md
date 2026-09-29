<!-- SPDX-License-Identifier: Apache-2.0 -->

# Project semantic unit tests

These tests exercise the compiler-owned project analyzer directly: visibility,
nominal compatibility, references, semantic cycles, and stable identity edges.
The analyzer retains no host I/O authority and publishes no project IR yet.
