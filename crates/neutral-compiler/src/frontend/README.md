<!-- SPDX-License-Identifier: Apache-2.0 -->

# Private minimal frontend

This directory owns compiler-private source decoding, raw tokens, physical
newline retention, semantic line-end normalization, and the recovery-free
minimal parser used by Stage 2, Step 3.

Nothing here is a public Neutral contract. Public consumers receive only
bounded diagnostics and, in later stages, validated public IR. A partial or
recovered private syntax model can never become authoritative output.
