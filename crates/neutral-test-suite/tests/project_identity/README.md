<!-- SPDX-License-Identifier: Apache-2.0 -->

# Project identity conformance

This suite owns executable literal transcript/SHA-256 vectors, complete logical
meaning, layer exclusions, bounded construction, cancellation, and artifact
selection/format separation. `vectors.json` is the execution copy of the portable
release asset; it remains usable after the portable plan is archived.
It does not claim reader/probe identity integration, independent full-vector
review, or actual incremental caching. Run `cargo test --package
neutral-test-suite project_identity`.
