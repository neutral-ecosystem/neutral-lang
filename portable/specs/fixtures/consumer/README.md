<!-- SPDX-License-Identifier: Apache-2.0 -->

# Consumer protocol boundary fixtures

This directory owns literal facts and requests for the frozen
[consumer protocol](../../contracts/CONSUMER-PROFILE.md). Its 32 cases cover
typed field/list/variant selection, future-result obligations, exact entry
selection, public attribution/privacy, incomplete contracts, compatibility,
limits and cancellation. The [graph corpus](graphs/README.md) separately registers
31 source/vocabulary consumer cases; neither suite claims implemented consumers.

`boundary.json` supplies an exact profile tuple, complete controls, public/private
type and binding facts, original source bytes, and case requests. Artifact labels
are fixture references, not paths or accepted serialized artifacts. The future
public harness must compile/read the literal source and compare retained facts,
or construct a separately validated equivalent artifact. Negative artifact
records contain `base` and one explicit `fault`: mutate that companion/type in
an otherwise validated copy and run the real reader boundary. Never trust a
fixture's `public` flag as a substitute for independent artifact validation.

Fact values use contextual JSON with exact numeric text; it is a fixture notation,
not a replacement IR/wire format. Type IDs and selectors use the frozen protocol.
Case `controls` replace only named default controls; omitted `cancelled` is false.
Entry and selector requests contain only the protocol members; controls and
cancellation belong to the outer request policy. `compatibility` compares the
literal producer tuple and exact reader capability set. `project-one`,
`project-two` and `legacy-document` are fixture capability labels for the
corresponding exact table rows, not negotiated protocol version strings.

[Literal oracles](../../../conformance/oracles/consumer/boundary.json) require
core/reader-contract acceptance separately from any consumer execution semantics.
No fixture performs an operation, resolves credentials or contacts a backend.
Public use-site byte locations are checked against exact source digests; private
reuse origins must remain explicitly redacted. Production comparison and
required-active status remain pending.
