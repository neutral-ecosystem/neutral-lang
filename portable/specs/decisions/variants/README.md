<!-- SPDX-License-Identifier: Apache-2.0 -->

# Planned tagged-variant fixtures

This directory owns design inputs for the [shared variant proposal](../../contracts/VARIANTS.md).
Both source and vocabulary declarations resolve to the same nominal type/value
model. These inputs are **not active conformance cases or passing evidence**.
They must gain reviewed request envelopes, literal oracles, and manifest hashes
before activation; runtime tests will own an independent execution copy.

| Group | Intended responsibility |
| --- | --- |
| [Positive](positive/README.md) | Local, imported, and vocabulary-owned types; selected payloads, lists, and references |
| [Negative](negative/README.md) | Closed tags, exact value shape, payload typing, and public type closure |

The vocabulary JSON is a proposed type entry, not a complete accepted bundle.
Activation requires a new bundle schema, complete captured bundle/lock inputs,
reviewed limits and diagnostic codes, and independent identity vectors. Existing
bundle schemas and identity profiles remain unchanged.
