# Section 12: evolution and conformance

## SYN-EVO-001 — Normative grammar

v0 must publish lexical grammar, raw-newline tokens, layout normalization,
context-free grammar, and static validation rules. The contract cannot depend on
one parser generator or recovery implementation.

## SYN-EVO-002 — Conformance corpus and reader proof

The corpus covers:

- every accepted syntax form and explicit exclusion;
- invalid UTF-8, escapes, comments, layout, names, and headers;
- records, lists, nullability, omission, defaults, and ordinary reuse;
- forward values, value cycles, references, and recursive records;
- exact numeric parsing, normalization, equality, and limits;
- captured vocabulary success and failure;
- resource limits and bounded diagnostics;
- malformed/adversarial encoded IR; and
- deterministic repeated/concurrent compilation.

### Source-to-IR and reader proof

Golden tests compare the logical IR graph modulo `ElementId` renaming, plus
source map, provenance, derivation, and diagnostics. They do not freeze map
iteration, pretty printing, or noncanonical serialization.

One generic effect-free probe must consume declarations, values, references,
and vocabulary-owned data exclusively through the public reader API and map a
probe diagnostic back to source.

## Exit rule

v0 is not complete until every master checklist item has normative prose,
grammar, positive/negative fixtures, stable diagnostics, lowering/provenance,
resource treatment, and reader evidence.

Later syntax requires a separate proposal and cannot be added by silently
extending the v0 grammar or vocabulary bundle.
