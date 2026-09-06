<!-- SPDX-License-Identifier: Apache-2.0 -->

# Standalone probe tests

This directory proves the `neutral-probe` executable can inspect encoded
Neutral artifacts through the public encoding and reader contracts. The tests
deliberately construct the artifact without `neutral-compiler`, execute the
installed-style binary boundary, and verify its stable categorized output.
The public-IR fixture also proves exact library/decoder/binary summary parity,
original-source diagnostic mapping, and enforcement of hostile traversal limits.

Within the ecosystem, these are consumer-boundary tests: they protect the
independent inspection path and prevent compiler linkage from becoming a hidden
runtime requirement.
