<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v1 specification

Status: accepted portable baseline

This is the v1 delta specification. It inherits the released v0.1 language
profile by reference; it does not duplicate v0 source rules. A conforming v1
implementation must retain the v0.1 corpus under `neu "0.1"` and implement the
v1 rules only for explicit `neu "1.0"` projects.

- [Requirements](REQUIREMENTS.md) is the delta obligation index.
- [Contracts](contracts/README.md) define source, capture/IR, vocabulary, and
  authoring behavior.
- [Decisions](decisions/README.md) records the closed architecture choices.
- [Fixtures](fixtures/README.md) and [examples](examples/README.md) own the
  eventual executable and explanatory corpus.
- [Traceability](TRACEABILITY.md) maps each delta group to its stage and
  conformance evidence.
