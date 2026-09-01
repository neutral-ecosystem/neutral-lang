<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 5 conformance oracles

This directory owns exact expected results for reuse and reference semantics.
Slice 5.1 entries freeze final logical values, ordinary reuse edges, stable
failure classes/codes/spans, and absence of IR on cycles. Slice 5.2 adds typed
identity-reference evidence. Slice 5.3 has no additional source spelling: its
alpha-equivalence evidence is generated directly from logical graph fixtures in
the IR property suite, including ID permutations and hostile graph mutations.
