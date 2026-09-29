<!-- SPDX-License-Identifier: Apache-2.0 -->

# Private source frontend

This directory owns compiler-private source decoding, raw tokens, physical
newline retention, semantic line-end normalization, and the recovery-free
parser. It handles exact trivia, identifiers, bounded string escape decoding,
Unicode scalar validation, exact numbers, Boolean literals, nullability,
record declarations and defaults, invariant lists, immutable-value reuse, and
typed identity references. Executable expressions remain inactive.

Nothing here is a public Neutral contract. Public consumers receive only
bounded diagnostics and validated public IR. A partial or
recovered private syntax model can never become authoritative output.
