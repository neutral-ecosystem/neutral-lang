<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v0 conformance fixtures

These fixtures cover the reduced, domain-neutral v0 language.

The corpus is grouped first by outcome, then by the primary language feature
under test. This keeps feature growth localized and makes it easy to discover
the matching fixture and oracle.

```text
fixtures/
├── vocabulary/
│   └── bundles/      # exact captured JSON bytes, grouped by outcome
├── positive/
│   ├── syntax/       # headers, comments, identifiers, and core source shape
│   ├── strings/      # valid string decoding
│   ├── booleans/     # valid Boolean literals
│   ├── numbers/      # valid exact-number spellings and normalization
│   ├── nullability/  # nullable scalars, explicit null, and outer widening
│   ├── records/      # valid nominal schemas and contextual values
│   ├── defaults/     # valid closed defaults and omission
│   ├── lists/        # valid ordered, nested, and defaulted lists
│   ├── values/       # value reuse and combined accepted cases
│   └── vocabulary/   # data-only vocabulary integration
└── negative/
    ├── syntax/       # rejected syntax, boundaries, and versions
    ├── identifiers/  # invalid and protected names
    ├── strings/      # rejected string forms and limits
    ├── booleans/     # rejected Boolean forms
    ├── numbers/      # rejected exact-number spellings and limits
    ├── nullability/  # rejected null placement and nullable type shapes
    ├── records/      # rejected nominal and contextual record behavior
    ├── defaults/     # rejected closed-default behavior
    ├── lists/        # rejected list values and resource limits
    ├── values/       # rejected value semantics
    └── vocabulary/   # rejected vocabulary/module features
```

## Positive source

- [string-escapes-unicode.neu](positive/strings/string-escapes-unicode.neu),
  [boolean-true.neu](positive/booleans/boolean-true.neu), and
  [boolean-false.neu](positive/booleans/boolean-false.neu) freeze Stage 3 Slice 3.2
  string decoding and exact Boolean values.
- [comments-equivalent.neu](positive/syntax/comments-equivalent.neu) proves line and
  block comments remain nonsemantic while their source regions stay observable
  only through source facts.
- [identifier-boundaries.neu](positive/syntax/identifier-boundaries.neu) freezes valid
  ASCII `snake_case` segments containing digits.
- [minimal-core.neu](positive/syntax/minimal-core.neu) is the Stage 2 atomic source
  path: one module with one exact `num` binding and a complete frozen oracle.
- The [positive number fixtures](positive/numbers/) freeze signs, separators,
  fractions, exponents, zero canonicalization, and exact normalization.
- The [positive nullability fixtures](positive/nullability/) freeze nullable
  scalar types, explicit typed null, and outer scalar widening.
- The [positive record fixtures](positive/records/) freeze nominal schemas,
  contextual values, forward collection, nesting, and canonical field order.
- The [positive default fixtures](positive/defaults/) freeze closed scalar,
  null, and record defaults, omission materialization, and field provenance.
- The [positive list fixtures](positive/lists/) freeze ordered, empty, nested,
  nullable-element, and defaulted lists.
- The [positive immutable-value reuse fixtures](positive/reuse/) freeze
  forward/transitive resolution, nested reuse, and outer-nullable widening.
- The [positive typed-reference fixtures](positive/references/) freeze forward
  targets, nested edges, field-name neutrality, and reference-only recursion.
- The [accepted captured vocabulary bundles](vocabulary/bundles/positive/)
  freeze strict JSON, all v0 type/default forms, exact byte identity, normalized
  logical equality, and reference-only vocabulary recursion.
- [immutable-value-reuse.neu](positive/values/immutable-value-reuse.neu) freezes
  the combined ordinary-reuse/reference boundary.
- [defaults-compatibility.neu](positive/values/defaults-compatibility.neu) covers
  defaults, nullability, lists, and outer nullable widening.
- [minimal-vocabulary.neu](positive/vocabulary/minimal-vocabulary.neu) is the
  Slice 6.2 source-to-IR integration fixture.

## Negative source

- The `string-*` scalar fixtures in [negative/strings](negative/strings/) plus
  [invalid-boolean-literal.neu](negative/booleans/invalid-boolean-literal.neu) freeze
  invalid escapes, Unicode scalars, controls, termination, types, and limits.
- Identifier failures are in [negative/identifiers](negative/identifiers/).
- Syntax failures are in [negative/syntax](negative/syntax/) and freeze the
  Stage 3 Slice 3.1 name, comment, punctuation, and token-boundary failures.
- The [negative number fixtures](negative/numbers/) freeze malformed
  separators/fractions/exponents, base prefixes, and digit/scale limits.
- The [negative nullability fixtures](negative/nullability/) freeze null in a
  non-nullable context, duplicate postfix `?`, and forbidden inner widening.
- The [negative record fixtures](negative/records/) freeze duplicate roots and
  fields, missing/unknown fields, type/kind mismatches, shorthand, and embedded
  recursion.
- The [negative default fixtures](negative/defaults/) freeze non-constant,
  reference, expression, inactive-list, wrong-type, and incomplete-record
  default failures.
- The [negative list fixtures](negative/lists/) freeze item typing, delimiters,
  and item/depth/traversal resource limits.
- The [negative immutable-value reuse fixtures](negative/reuse/) freeze unknown
  and wrong-kind names, direct/indirect cycles, and invariant list arguments.
- The [negative typed-reference fixtures](negative/references/) freeze unknown,
  wrong-kind, wrong-type, invariant-target, and missing-constructor failures.
- The [hostile captured vocabulary bundles](vocabulary/bundles/negative/) freeze
  duplicate, unknown, executable, raw-number, malformed, target, recursion,
  default, and feature rejection.
- [module-path.neu](negative/vocabulary/module-path.neu): module qualification is absent.
- [mut-modifier.neu](negative/vocabulary/mut-modifier.neu): mutation is absent.
- [namespace-declaration.neu](negative/vocabulary/namespace-declaration.neu): namespaces
  are absent.
- [nonconstant-default.neu](negative/defaults/nonconstant-default.neu): defaults cannot
  read bindings.
- [reassignment.neu](negative/values/reassignment.neu): reassignment is absent.
- [version-escape.neu](negative/syntax/version-escape.neu): escaped version spelling.
- [version-leading-zero.neu](negative/syntax/version-leading-zero.neu): noncanonical
  version spelling.
- [visibility-modifier.neu](negative/vocabulary/visibility-modifier.neu): visibility syntax
  is absent.
- [vocabulary-name-collision.neu](negative/vocabulary/vocabulary-name-collision.neu): the
  imported vocabulary namespace cannot be redeclared.
- [missing-capture.neu](negative/vocabulary/missing-capture.neu) and
  [unknown-type.neu](negative/vocabulary/unknown-type.neu) freeze exact captured
  resolution and qualified-type failures.
- [missing-payload-field.neu](negative/vocabulary/missing-payload-field.neu),
  [duplicate-payload-field.neu](negative/vocabulary/duplicate-payload-field.neu),
  [unknown-payload-field.neu](negative/vocabulary/unknown-payload-field.neu), and
  [wrong-payload-type.neu](negative/vocabulary/wrong-payload-type.neu) freeze the
  closed vocabulary payload diagnostics.

## Contract matrix

- [vocabulary-contract-cases.md](vocabulary-contract-cases.md) specifies captured
  vocabulary resolution, closed-schema validation, and external-reader cases.
- [Fixture/oracle review candidate](../../conformance/fixture-oracle-review.toml)
  locks the current fixture bytes and identifies the required oracle shape for
  each case. It is not approved or immutable until contract-freeze review.

## Generated evidence outside source fixtures

Malformed UTF-8/NUL vectors, exhaustive layout boundaries, concurrent
compilation schedules, graph alpha-equivalence, invalid source-map/provenance
states, and hostile encoded IR are generated directly by their owning tests.
They are not `.neu` fixtures because some cannot be represented as valid UTF-8
source and others exercise public in-memory or encoded-artifact contracts rather
than source syntax. Resource cases with stable source spellings remain registered
fixtures; boundary permutations are generated from centralized limit values.
