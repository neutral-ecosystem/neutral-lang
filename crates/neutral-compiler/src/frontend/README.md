<!-- SPDX-License-Identifier: Apache-2.0 -->

# Private scalar frontend

This directory owns compiler-private source decoding, raw tokens, physical
newline retention, semantic line-end normalization, and the recovery-free
scalar parser. Stage 3 adds exact trivia, identifiers, bounded string escape
decoding, Unicode scalar validation, exact numbers, Boolean literals, postfix
scalar nullability, and explicit null.

Nothing here is a public Neutral contract. Public consumers receive only
bounded diagnostics and validated public IR. A partial or
recovered private syntax model can never become authoritative output.
