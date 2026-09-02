<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral

Neutral is a portable declarative language under development. The current
repository contains the approved v0 specification and a Rust implementation
complete through Stage 6 Slice 6.1. The active compiler supports source/scalar
behavior, records, defaults, invariant lists, and ordinary immutable-value
reuse and typed identity references through validated IR, reader, and probe
boundaries. Logical payload comparison uses whole-graph alpha-equivalence under
one consistent document-local `ElementId` mapping.

The captured-vocabulary boundary also validates exact locked JSON bundle bytes
into immutable closed logical contracts without filesystem or network lookup.
Vocabulary source syntax remains scheduled for Slice 6.2.

Start with [the development plan](portable/DEVELOPMENT.md). The mandatory
contract-freeze gate and current implementation progress are recorded there.
