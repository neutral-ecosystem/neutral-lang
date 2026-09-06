# Section 10: diagnostics and invalid syntax

## SYN-DIA-001 — Stable diagnostics, spans, recovery, and bounds

Codes use `NL-<CLASS>-<ID>` and distinguish at least encoding, lexical/layout,
syntax, name, kind, type, value cycle, reference, vocabulary, feature, limit,
invalid IR, and internal defect classes. Codes and safe parameter schemas are
stable API; rendered messages may improve.

### Source spans

Primary and related locations contain logical source-unit identity plus
half-open original-byte ranges. Optional display line/column is derived from the
captured bytes and cannot replace byte offsets.

### Recovery

Recovery synchronizes only at documented declaration/field/list boundaries.
Recovered private models may support an editor but never produce authoritative
IR. A truncation or fatal error cannot look successful.

### Ordering and bounds

Canonical ordering is logical source-unit identity, start byte, end byte, stable
code, then deterministic safe parameters. It is independent of hash order,
threads, resolver delivery, and message text.

The initial diagnostic cap is a versioned implementation profile. On reaching
it, the compiler stops optional recovery and emits one bounded too-many-errors
finding.

## SYN-DIA-002 — Example obligation

Every syntax feature includes valid, invalid, boundary, and misleading-lookalike
fixtures. The ambiguity corpus includes newline boundaries, comments, `::`
qualification, forbidden `.`, negative numbers, contextual braces, `T?`,
ordinary `name` reuse versus `ref(name)`, and missing commas/terminators.

The generic probe may attach its own diagnostic to an IR element and map it
through the public source map. That diagnostic remains consumer-owned rather
than being relabeled as a source syntax error.
