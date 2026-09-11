<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral language v0 syntax checklist

Status: active implementation checklist

Authoritative master: [syntax.md](syntax.md)

Checking an item requires normative prose, grammar, valid and invalid fixtures,
stable diagnostics, source-to-IR lowering, and public-reader evidence.

## 1. Boundaries

- [x] SYN-GOV-001 — Representation boundaries
- [x] SYN-GOV-002 — Lowering and provenance
- [x] SYN-GOV-003 — Acceptance is not authority or execution
- [x] SYN-GOV-004 — Completion evidence

Decision: [governing boundaries](../decisions/01-governing-boundaries.md)

## 2. Source text

- [x] SYN-LEX-001 — UTF-8 and byte spans
- [x] SYN-LEX-002 — Newline layout and no semicolons
- [x] SYN-LEX-003 — Line and block comments
- [x] SYN-LEX-004 — Identifier categories
- [x] SYN-LEX-005 — `::` vocabulary qualification and rejected `.`
- [x] SYN-LEX-006 — Delimiters, strings, scalars, and `null`

Decision: [lexical source text](../decisions/02-lexical-source-text.md)

## 3. Document

- [x] SYN-DOC-001 — Exact language header
- [x] SYN-DOC-002 — One source unit and module
- [x] SYN-DOC-003 — Optional captured vocabulary use
- [x] SYN-DOC-004 — Root records/bindings and universal export

Decision: [document shape](../decisions/03-documents-modules-profiles.md)

## 4. Declarations

- [x] SYN-DEC-001 — Immutable type-first bindings
- [x] SYN-DEC-002 — Explicit declaration types
- [x] SYN-DEC-003 — No mutation/reassignment/override
- [x] SYN-DEC-004 — Duplicates and protected names
- [x] SYN-DEC-005 — Forward resolution and value cycles

Decision: [declarations and names](../decisions/04-declarations-bindings-names.md)

## 5. Types

- [x] SYN-TYP-001 — `num`, `string`, and `bool`
- [x] SYN-TYP-002 — Exact source/IR numeric semantics
- [x] SYN-TYP-003 — Nominal records and recursion
- [x] SYN-TYP-004 — `List<T>`
- [x] SYN-TYP-005 — `T?`, `null`, required/defaulted fields
- [x] SYN-TYP-006 — `Ref<T>` and exact compatibility
- [x] SYN-TYP-007 — Vocabulary-owned nominal types

Decision: [type notation](../decisions/05-type-schema-notation.md)

## 6. Values

- [x] SYN-VAL-001 — Scalar and null literals
- [x] SYN-VAL-002 — Contextual record values
- [x] SYN-VAL-003 — Homogeneous lists
- [x] SYN-VAL-004 — Immutable value reuse
- [x] SYN-VAL-005 — Logical values versus provenance
- [x] SYN-VAL-006 — Rejected shorthand/member/static access

Decision: [literal values](../decisions/06-literal-values.md)

## 7. References

- [x] SYN-REF-001 — `ref(name)`
- [x] SYN-REF-002 — Target kind and type checks
- [x] SYN-REF-003 — Identity-only semantics
- [x] SYN-REF-004 — Document-local IDs and alpha-equivalence

Decision: [references](../decisions/07-references-relationships.md)

## 8. Vocabulary

- [x] SYN-VOC-001 — `use Vocabulary` and `Vocabulary::Type`
- [x] SYN-VOC-002 — Closed data-only bundle schema
- [x] SYN-VOC-003 — Exact captured resolution
- [x] SYN-VOC-004 — Payload/feature validation without execution
- [x] SYN-VOC-005 — Vocabulary diagnostics

Decision: [vocabulary boundary](../decisions/10-domain-vocabulary.md)

## 9. Safety and conformance

- [x] SYN-DIA-001 — Stable diagnostics and bounded recovery
- [x] SYN-DIA-002 — Complete example obligations
- [x] SYN-TOL-001 — Stable formatter and non-semantic comments
- [x] SYN-EVO-001 — Normative grammar/token boundaries
- [x] SYN-EVO-002 — Source-to-IR/reader conformance

Decisions:

- [security and limits](../decisions/12-security-sensitive-syntax.md)
- [diagnostics](../decisions/13-diagnostics-invalid-syntax.md)
- [formatting](../decisions/14-documentation-formatting-tools.md)
- [conformance](../decisions/15-evolution-conformance.md)

## Completion rule

v0 is complete only when every item is checked and the generic effect-free probe
passes using only the public reader API.
