# Neutral language v0: needed features

Status: proposed v0 requirements

This file is the requirements source for the Neutral v0 checklists. It contains
only features needed to prove the first source-to-IR boundary. It does not plan
later language versions or any application runtime.

## 1. Product outcome

v0 must prove this complete path:

```text
one captured .neu source unit
    -> parser and semantic analysis
    -> immutable Neutral IR
    -> public reader API
    -> generic effect-free probe
```

- **NL-V0-001:** The probe must enumerate declarations, types, values,
  `Ref<T>` edges, vocabulary-owned values, and provenance without parsing
  source or using compiler-private models.
- **NL-V0-002:** A probe diagnostic must map from an IR element to the correct
  captured source span.
- **NL-V0-003:** v0 must assign no application-specific meaning or perform an
  external effect.

## 2. Representation boundaries

- **NL-BND-001:** Concrete `.neu` syntax, compiler-private frontend/semantic
  models, public Neutral IR, and consumer-private data are distinct layers.
- **NL-BND-002:** Consumers receive only documented public IR/reader contracts.
- **NL-BND-003:** Every accepted source construct must have documented lowering
  or explicitly non-semantic source-map treatment.
- **NL-BND-004:** Successful compilation means structural validity, not
  authorization, execution, external compatibility, or proof of an effect.
- **NL-BND-005:** v0 has no public IR transformation API.

## 3. Captured compilation

- **NL-CAP-001:** A v0 request contains exactly one source unit and one logical
  module.
- **NL-CAP-002:** The host supplies the only resolver used by compilation.
- **NL-CAP-003:** `capture(request)` produces an immutable
  `CapturedCompilation`; `compileCaptured(...)` performs no external I/O.
- **NL-CAP-004:** The captured form includes exact source bytes, logical and
  content identities, optional exact vocabulary bytes/identity, behavior
  versions, options, and deterministic budgets.
- **NL-CAP-005:** Missing captured inputs fail closed. Source triggers no
  ambient filesystem, environment, or network lookup.
- **NL-CAP-006:** Resolver credentials and host paths do not enter source, IR,
  ordinary diagnostics, or derivation.

## 4. Source text and grammar

- **NL-SRC-001:** Source is UTF-8. Invalid UTF-8 and unescaped NUL are errors.
- **NL-SRC-002:** Original bytes define source identity and byte spans; line
  endings normalize logically without changing original offsets.
- **NL-SRC-003:** `//` starts a line comment and `/* ... */` is a non-nesting
  block comment.
- **NL-SRC-004:** Declarations end by logical newline or a closing declaration
  block; v0 has no semicolon token.
- **NL-SRC-005:** The private frontend is raw lexer → layout normalizer →
  parser, with deterministic recovery boundaries.
- **NL-SRC-006:** Identifiers are ASCII. Bindings, fields, and modules use
  `snake_case`; type and vocabulary names are uppercase-leading, with
  `UpperCamelCase` as style.
- **NL-SRC-007:** `::` is reserved for vocabulary qualification in v0. General
  `.` member access and vocabulary static selection are absent.

## 5. Document shape

- **NL-DOC-001:** Every unit begins with canonical `neu "0.1"` followed by one
  canonical `module snake_case` header.
- **NL-DOC-002:** Zero or one `use Vocabulary` declaration appears after the
  headers and before ordinary declarations.
- **NL-DOC-003:** `use` names a logical vocabulary requirement only; captured
  lock data selects one exact permitted bundle.
- **NL-DOC-004:** The root contains record and binding declarations only.
- **NL-DOC-005:** All valid declarations are exported. v0 has no visibility
  modifier, namespace declaration, module import, or multi-unit merge.

## 6. Declarations and names

- **NL-DEC-001:** Bindings use immutable type-first syntax without `let`:
  `num count = 10`.
- **NL-DEC-002:** Every binding has an explicit type. Context may determine the
  type of an inner record/list literal, but not a declaration.
- **NL-DEC-003:** v0 has no `mut`, reassignment, assignment statement, override,
  or mutation method.
- **NL-DEC-004:** Declarations are collected before value/reference resolution,
  so declaration order is non-semantic.
- **NL-DEC-005:** Ordinary value uses may refer forward. Static value-dependency
  cycles are errors.
- **NL-DEC-006:** `ref(name)` may refer forward and does not participate in the
  value-dependency cycle graph.
- **NL-DEC-007:** Duplicate names are invalid. Core names cannot be redeclared.

## 7. Core types

- **NL-TYP-001:** The scalar types are exactly `num`, `string`, and `bool`.
- **NL-TYP-002:** `num` is an exact normalized base-10 rational in Neutral IR;
  it has no host-dependent integer/float split or v0 target conversion.
- **NL-TYP-003:** `T?` adds `null` to a type. `null` is the only explicit null
  literal and there is no `absent`/`none` value.
- **NL-TYP-004:** `List<T>` is ordered and homogeneous.
- **NL-TYP-005:** `Ref<T>` is a typed document-local identity reference.
- **NL-TYP-006:** User records are nominal. Every recursive record cycle is
  invalid unless the cycle crosses `Ref<T>`.
- **NL-TYP-007:** Compatibility is exact type identity plus outer `T` to `T?`
  widening. Generic arguments are invariant.
- **NL-TYP-008:** v0 has no maps, sets, tuples, unions, enums, user-defined
  generics, or implicit conversions other than the stated nullability widening.

## 8. Records, lists, defaults, and values

- **NL-VAL-001:** A record declaration contains typed fields with unique
  `snake_case` names.
- **NL-VAL-002:** Contextual `{ ... }` construction requires exactly one
  expected nominal type and explicit `field: value` entries.
- **NL-VAL-003:** Field shorthand and untyped anonymous records are absent.
- **NL-VAL-004:** `[...]` constructs an ordered homogeneous list; an empty list
  requires an expected `List<T>` type.
- **NL-VAL-005:** A field without a default is required. A field with a closed
  constant default may be omitted.
- **NL-VAL-006:** Required/defaulted and nullable/non-nullable are independent:
  `string name`, `string? note`, `string name = "x"`, and
  `string? note = null` have distinct contracts.
- **NL-VAL-007:** Defaults may contain only literals, `null` where nullable,
  lists, and contextual records composed from closed constants.
- **NL-VAL-008:** An ordinary binding name reuses its immutable logical value.
  Logical IR stores the resulting value; provenance stores the reuse edge.

## 9. Identity references

- **NL-REF-001:** Only `ref(name)` constructs `Ref<T>`.
- **NL-REF-002:** The target must be a value binding of compatible type; types,
  modules, and vocabulary namespaces are wrong-kind targets.
- **NL-REF-003:** `Ref<T>` means identity only. It implies no containment,
  ownership, ordering, dependency, readiness, or runtime behavior.
- **NL-REF-004:** IR targets use graph-local `ElementId`. Logical equality and
  determinism are defined modulo consistent `ElementId` renaming.
- **NL-REF-005:** Consumers cannot treat `ElementId` as durable cross-document
  identity.

## 10. Minimal vocabulary boundary

- **NL-VOC-001:** v0 supports zero or one exact captured data-only vocabulary.
- **NL-VOC-002:** A vocabulary may define nominal types, fields, closed constant
  defaults, and required structural feature IDs.
- **NL-VOC-003:** Vocabulary types use `Vocabulary::Type`; this does not add a
  special declaration kind to core grammar.
- **NL-VOC-004:** Bundles conform to one closed, versioned, Neutral-owned schema
  and contain no scripts, callbacks, arbitrary expressions, custom validators,
  bytecode, or native modules.
- **NL-VOC-005:** Published schema/feature IDs have immutable meaning. Unknown
  required structural semantics fail closed.
- **NL-VOC-006:** The compiler validates payload structure. A generic probe only
  enumerates vocabulary-owned data and does not interpret application behavior.
- **NL-VOC-007:** The external reader receives exact captured vocabulary
  contracts from its host and performs no hidden acquisition.

## 11. IR and identity

- **NL-IR-001:** Logical payload and artifact envelope are separate projections.
- **NL-IR-002:** The payload contains versions, logical module/vocabulary
  identities, required features, declarations, types, final values, and
  reference edges.
- **NL-IR-003:** Producer/build, encoding, integrity, source-map, provenance, and
  derivation facts live in the envelope or companion artifacts and do not alter
  logical equality.
- **NL-IR-004:** Logical module identity, captured source/bundle content
  identity, module-symbol identity, declaration fingerprint, graph-local element
  ID, derivation identity, and byte identity are distinct.
- **NL-IR-005:** Logical equality is graph alpha-equivalence under one-to-one
  element-ID mapping.
- **NL-IR-006:** Successful compiler output is immutable and already validated.
  External encoded IR is always decoded and validated as untrusted input.

## 12. Source maps, provenance, and derivation

- **NL-PRO-001:** Source maps associate IR element IDs with half-open original
  byte spans.
- **NL-PRO-002:** Provenance records why a value exists: explicit source,
  ordinary reuse, record default, or vocabulary default.
- **NL-PRO-003:** Provenance references source-map entries rather than copying
  spans.
- **NL-PRO-004:** Derivation separates meaning-affecting inputs,
  acceptance/resource inputs, and diagnostic/output-policy inputs.
- **NL-PRO-005:** Formatting-only changes may preserve logical IR while changing
  source maps. A semantic cache cannot reuse stale locations.

## 13. APIs

- **NL-API-001:** Public operations include `capture`, `compileCaptured`, the
  convenience `compile`, and `decodeAndValidate`.
- **NL-API-002:** Compilation returns authoritative IR only on success, plus
  diagnostics, source map, provenance, derivation, and resource facts.
- **NL-API-003:** The reader exposes immutable typed traversal and indexed lookup
  without exposing storage layout as meaning.
- **NL-API-004:** APIs are reentrant, concurrency-safe, cancellable, and bounded.
- **NL-API-005:** v0 exposes no public AST and no IR rewrite/transformation API.

## 14. Diagnostics and robustness

- **NL-DIA-001:** Diagnostics have stable codes, layer, severity, safe
  parameters, primary/related spans, optional remedy, and truncation state.
- **NL-DIA-002:** Canonical ordering uses logical source-unit identity, byte
  range, code, then safe parameters.
- **NL-DIA-003:** Recovery never converts invalid source into authoritative IR.
- **NL-DIA-004:** Compiler and reader enforce explicit bounds on bytes, nesting,
  declarations, references, list size, numeric digits/scale, diagnostics, and IR
  size before proportional work.
- **NL-DIA-005:** Operational time/memory ceilings are deployment controls, not
  reproducible language semantics.
- **NL-DIA-006:** Diagnostics escape control text and never expose accidental
  host paths or acquisition credentials.

## 15. Required v0 documentation and evidence

v0 requires:

1. normative lexical, layout, grammar, name, type, value, reference, and
   vocabulary specifications;
2. a logical IR and one noncanonical encoding specification;
3. identity, source-map, provenance, and derivation specifications;
4. compiler, resolver, reader, and probe API specifications;
5. positive, negative, ambiguity, exact-numeric, vocabulary, resource, and adversarial
   fixtures;
6. deterministic repeated/concurrent compilation tests; and
7. one generic effect-free public-API probe.

## 16. Explicit v0 non-features

The following are outside v0 and are not implied future commitments:

- namespaces and visibility modifiers;
- multiple source units and imports;
- secret references;
- static/member selection;
- symbolic operators and expressions;
- functions and control structures;
- mutation and override;
- composition, templates, macros, and expansion;
- general-purpose collections beyond `List<T>`;
- executable plugins;
- ambient or runtime effects;
- IR transformation and migration APIs;
- canonical encoding; and
- application-specific syntax, types, or behavior.
