# Neutral language v0 syntax checklist

Status: proposed implementation checklist

Authoritative master: [syntax.md](syntax.md)

Checking an item requires normative prose, grammar, valid and invalid fixtures,
stable diagnostics, source-to-IR lowering, and public-reader evidence.

## 1. Boundaries

- [ ] SYN-GOV-001 — Representation boundaries
- [ ] SYN-GOV-002 — Lowering and provenance
- [ ] SYN-GOV-003 — Acceptance is not authority or execution
- [ ] SYN-GOV-004 — Completion evidence

Decision: [governing boundaries](decisions/01-governing-boundaries.md)

## 2. Source text

- [ ] SYN-LEX-001 — UTF-8 and byte spans
- [ ] SYN-LEX-002 — Newline layout and no semicolons
- [ ] SYN-LEX-003 — Line and block comments
- [ ] SYN-LEX-004 — Identifier categories
- [ ] SYN-LEX-005 — `::` vocabulary qualification and rejected `.`
- [ ] SYN-LEX-006 — Delimiters, strings, scalars, and `null`

Decision: [lexical source text](decisions/02-lexical-source-text.md)

## 3. Document

- [ ] SYN-DOC-001 — Exact language header
- [ ] SYN-DOC-002 — One source unit and module
- [ ] SYN-DOC-003 — Optional captured vocabulary use
- [ ] SYN-DOC-004 — Root records/bindings and universal export

Decision: [document shape](decisions/03-documents-modules-profiles.md)

## 4. Declarations

- [ ] SYN-DEC-001 — Immutable type-first bindings
- [ ] SYN-DEC-002 — Explicit declaration types
- [ ] SYN-DEC-003 — No mutation/reassignment/override
- [ ] SYN-DEC-004 — Duplicates and protected names
- [ ] SYN-DEC-005 — Forward resolution and value cycles

Decision: [declarations and names](decisions/04-declarations-bindings-names.md)

## 5. Types

- [ ] SYN-TYP-001 — `num`, `string`, and `bool`
- [ ] SYN-TYP-002 — Exact source/IR numeric semantics
- [ ] SYN-TYP-003 — Nominal records and recursion
- [ ] SYN-TYP-004 — `List<T>`
- [ ] SYN-TYP-005 — `T?`, `null`, required/defaulted fields
- [ ] SYN-TYP-006 — `Ref<T>` and exact compatibility
- [ ] SYN-TYP-007 — Vocabulary-owned nominal types

Decision: [type notation](decisions/05-type-schema-notation.md)

## 6. Values

- [ ] SYN-VAL-001 — Scalar and null literals
- [ ] SYN-VAL-002 — Contextual record values
- [ ] SYN-VAL-003 — Homogeneous lists
- [ ] SYN-VAL-004 — Immutable value reuse
- [ ] SYN-VAL-005 — Logical values versus provenance
- [ ] SYN-VAL-006 — Rejected shorthand/member/static access

Decision: [literal values](decisions/06-literal-values.md)

## 7. References

- [ ] SYN-REF-001 — `ref(name)`
- [ ] SYN-REF-002 — Target kind and type checks
- [ ] SYN-REF-003 — Identity-only semantics
- [ ] SYN-REF-004 — Document-local IDs and alpha-equivalence

Decision: [references](decisions/07-references-relationships.md)

## 8. Vocabulary

- [ ] SYN-VOC-001 — `use Vocabulary` and `Vocabulary::Type`
- [ ] SYN-VOC-002 — Closed data-only bundle schema
- [ ] SYN-VOC-003 — Exact captured resolution
- [ ] SYN-VOC-004 — Payload/feature validation without execution
- [ ] SYN-VOC-005 — Vocabulary diagnostics

Decision: [vocabulary boundary](decisions/10-domain-vocabulary.md)

## 9. Safety and conformance

- [ ] SYN-DIA-001 — Stable diagnostics and bounded recovery
- [ ] SYN-DIA-002 — Complete example obligations
- [ ] SYN-TOL-001 — Stable formatter and non-semantic comments
- [ ] SYN-EVO-001 — Normative grammar/token boundaries
- [ ] SYN-EVO-002 — Source-to-IR/reader conformance

Decisions:

- [security and limits](decisions/12-security-sensitive-syntax.md)
- [diagnostics](decisions/13-diagnostics-invalid-syntax.md)
- [formatting](decisions/14-documentation-formatting-tools.md)
- [conformance](decisions/15-evolution-conformance.md)

## Completion rule

v0 is complete only when every item is checked and the generic effect-free probe
passes using only the public reader API.
