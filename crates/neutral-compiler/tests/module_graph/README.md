<!-- SPDX-License-Identifier: Apache-2.0 -->

# Module graph tests

This directory owns compiler-local tests for logical import parsing and graph
construction from complete in-memory captured projects. It verifies the pure
graph boundary and bounded SCC algorithm without host acquisition. Cross-crate
public-result integration and executable portable conformance belong to the
cross-package test suite.
