# Neutral language v0 syntax checklist

Status: proposed master checklist

This is the complete syntax scope for v0. There is no v1 or v2 syntax plan in
the current specification. Unlisted syntax is unsupported.

## 1. Governing boundaries

- [ ] **SYN-GOV-001** — Separate `.neu`, private compiler models, public Neutral
  IR, and consumer-private models.
- [ ] **SYN-GOV-002** — Define lowering and source provenance for every accepted
  construct.
- [ ] **SYN-GOV-003** — Keep syntax acceptance separate from authority,
  execution, and external effects.
- [ ] **SYN-GOV-004** — Require normative prose, grammar, valid/invalid fixtures,
  diagnostics, lowering, and reader evidence for completion.

## 2. Source text and layout

- [ ] **SYN-LEX-001** — Define UTF-8, BOM, invalid bytes, NUL, and original-byte
  spans.
- [ ] **SYN-LEX-002** — Define whitespace and newline termination through raw
  lexer, layout normalizer, and parser stages; do not add semicolons.
- [ ] **SYN-LEX-003** — Define `//` and non-nesting `/* ... */` comments.
- [ ] **SYN-LEX-004** — Define ASCII `snake_case` value/field/module names and
  uppercase-leading type/vocabulary names.
- [ ] **SYN-LEX-005** — Reserve `::` for vocabulary qualification; reject `.`.
- [ ] **SYN-LEX-006** — Define delimiters, commas, trailing commas, strings,
  scalar literals, and `null`.

## 3. Document

- [ ] **SYN-DOC-001** — Require exact `neu "0.1"` syntax.
- [ ] **SYN-DOC-002** — Require one `module snake_case` header and one source
  unit per compilation.
- [ ] **SYN-DOC-003** — Allow zero or one `use Vocabulary` before declarations,
  resolved only by captured host input.
- [ ] **SYN-DOC-004** — Allow record and binding declarations at root; export all
  declarations and reject namespaces/visibility/imports.

## 4. Declarations and names

- [ ] **SYN-DEC-001** — Define immutable type-first bindings without `let`.
- [ ] **SYN-DEC-002** — Require explicit declaration types.
- [ ] **SYN-DEC-003** — Reject mutation, reassignment, and override.
- [ ] **SYN-DEC-004** — Reject duplicate names and protect core names.
- [ ] **SYN-DEC-005** — Collect declarations before resolving forward values and
  references; reject value-dependency cycles.

## 5. Types and records

- [ ] **SYN-TYP-001** — Define `num`, `string`, and `bool`.
- [ ] **SYN-TYP-002** — Define exact host-independent source/IR `num` semantics
  without v0 target conversion.
- [ ] **SYN-TYP-003** — Define nominal records, typed fields, and recursion rules.
- [ ] **SYN-TYP-004** — Define ordered homogeneous `List<T>`.
- [ ] **SYN-TYP-005** — Define postfix nullability `T?`, `null`, required fields,
  and closed-constant defaults as independent axes.
- [ ] **SYN-TYP-006** — Define invariant `Ref<T>` and exact type compatibility
  with only outer `T` to `T?` widening.
- [ ] **SYN-TYP-007** — Define qualified vocabulary-owned nominal types.

## 6. Values

- [ ] **SYN-VAL-001** — Define Boolean, numeric, string, and null literals.
- [ ] **SYN-VAL-002** — Define contextual record values with explicit fields and
  one expected nominal type.
- [ ] **SYN-VAL-003** — Define homogeneous list values and empty-list context.
- [ ] **SYN-VAL-004** — Define ordinary binding names as immutable value reuse.
- [ ] **SYN-VAL-005** — Keep final logical values separate from reuse/default
  provenance.
- [ ] **SYN-VAL-006** — Reject field shorthand, anonymous record types, static
  member selection, and general member access.

## 7. Identity references

- [ ] **SYN-REF-001** — Define `ref(name)` as the only constructor for `Ref<T>`.
- [ ] **SYN-REF-002** — Restrict targets to compatible value bindings and define
  wrong-kind errors.
- [ ] **SYN-REF-003** — Specify identity-only meaning with no inferred
  containment, ownership, dependency, or order.
- [ ] **SYN-REF-004** — Define document-local targets and logical equality modulo
  `ElementId` renaming.

## 8. Minimal vocabulary surface

- [ ] **SYN-VOC-001** — Define `use Vocabulary` and `Vocabulary::Type`.
- [ ] **SYN-VOC-002** — Define the closed, versioned, data-only bundle schema.
- [ ] **SYN-VOC-003** — Pin exact identity/version through captured lock input.
- [ ] **SYN-VOC-004** — Validate contextual payloads and required structural
  features without executing bundle code.
- [ ] **SYN-VOC-005** — Define vocabulary-resolution and payload diagnostics.

## 9. Diagnostics, formatting, and conformance

- [ ] **SYN-DIA-001** — Define stable diagnostic classes, source spans,
  deterministic ordering, caps, and recovery boundaries.
- [ ] **SYN-DIA-002** — Require valid, invalid, boundary, and misleading-lookalike
  examples for every syntax form.
- [ ] **SYN-TOL-001** — Define one stable formatter; comments remain
  non-semantic.
- [ ] **SYN-EVO-001** — Publish normative grammar and token boundaries.
- [ ] **SYN-EVO-002** — Require source-to-IR, source-map, provenance, and public
  reader conformance.

## Explicit v0 syntax exclusions

- namespaces and visibility modifiers;
- multiple source units and imports;
- secret-reference types and constructors;
- `.` selection;
- maps, sets, tuples, unions, enums, and user generics;
- operators and symbolic expressions;
- functions and control structures;
- mutation, composition, override, templates, and macros; and
- application-specific declaration forms.
