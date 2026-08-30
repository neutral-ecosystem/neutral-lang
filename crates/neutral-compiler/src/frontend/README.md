<!-- SPDX-License-Identifier: Apache-2.0 -->

# Private source frontend

This directory owns compiler-private source decoding, raw tokens, physical
newline retention, semantic line-end normalization, and the recovery-free
parser. Stage 3 adds exact trivia, identifiers, bounded string escape
decoding, Unicode scalar validation, exact numbers, Boolean literals, postfix
scalar nullability, and explicit null. Stage 4 Slice 4.1 adds bounded multiline
record declarations, nominal types, required fields, and recursively contextual
record values. Slice 4.2 adds field defaults and closed scalar/null/record
constant syntax. Slice 4.3 adds invariant list types and bounded ordered list
values while keeping reuse, references, and expressions inactive.

Nothing here is a public Neutral contract. Public consumers receive only
bounded diagnostics and validated public IR. A partial or
recovered private syntax model can never become authoritative output.
