<!-- SPDX-License-Identifier: Apache-2.0 -->

# Vocabulary seeds

The v1 seed mirrors the closed project-bundle shape from the active portable
Stage 5 fixtures. The fuzz harness computes an exact lock for each mutated
input, so mutations reach schema validation rather than stopping at a stale
digest.

`composition.json` covers the explicitly selected composition schema, including
restricted defaults, nullable/optional fields, variant-list payloads and nominal
reference types. It is a reviewed seed, not mutable corpus or full project
integration evidence. Its inert identity/revision match the harness lock.
