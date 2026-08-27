# Neutral v0 conformance fixtures

These fixtures cover the reduced, domain-neutral v0 language.

The corpus is grouped first by outcome, then by the primary language feature
under test. This keeps feature growth localized and makes it easy to discover
the matching fixture and oracle.

```text
fixtures/
├── positive/
│   ├── syntax/       # headers, comments, identifiers, and core source shape
│   ├── strings/      # valid string decoding
│   ├── booleans/     # valid Boolean literals
│   ├── numbers/      # valid exact-number spellings and normalization
│   ├── nullability/  # nullable scalars, explicit null, and outer widening
│   ├── values/       # value reuse and defaults
│   └── vocabulary/   # data-only vocabulary integration
└── negative/
    ├── syntax/       # rejected syntax, boundaries, and versions
    ├── identifiers/  # invalid and protected names
    ├── strings/      # rejected string forms and limits
    ├── booleans/     # rejected Boolean forms
    ├── numbers/      # rejected exact-number spellings and limits
    ├── nullability/  # rejected null placement and nullable type shapes
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
- [immutable-value-reuse.neu](positive/values/immutable-value-reuse.neu) distinguishes
  ordinary value reuse from identity references and proves forward resolution.
- [defaults-compatibility.neu](positive/values/defaults-compatibility.neu) covers
  defaults, nullability, lists, and outer nullable widening.
- [minimal-vocabulary.neu](positive/vocabulary/minimal-vocabulary.neu) proves one captured
  data-only vocabulary through the generic source-to-IR boundary.

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
- [generic-covariance.neu](negative/vocabulary/generic-covariance.neu): invariant generic
  argument violation.
- [module-path.neu](negative/vocabulary/module-path.neu): module qualification is absent.
- [mut-modifier.neu](negative/vocabulary/mut-modifier.neu): mutation is absent.
- [namespace-declaration.neu](negative/vocabulary/namespace-declaration.neu): namespaces
  are absent.
- [nonconstant-default.neu](negative/values/nonconstant-default.neu): defaults cannot
  read bindings.
- [reassignment.neu](negative/values/reassignment.neu): reassignment is absent.
- [value-cycle.neu](negative/values/value-cycle.neu): immutable value cycle.
- [version-escape.neu](negative/syntax/version-escape.neu): escaped version spelling.
- [version-leading-zero.neu](negative/syntax/version-leading-zero.neu): noncanonical
  version spelling.
- [visibility-modifier.neu](negative/vocabulary/visibility-modifier.neu): visibility syntax
  is absent.
- [vocabulary-name-collision.neu](negative/vocabulary/vocabulary-name-collision.neu): the
  imported vocabulary namespace cannot be redeclared.

## Contract matrix

- [vocabulary-contract-cases.md](vocabulary-contract-cases.md) specifies captured
  vocabulary resolution, closed-schema validation, and external-reader cases.
- [Fixture/oracle review candidate](../../../../conformance/fixture-oracle-review.toml)
  locks the current fixture bytes and identifies the required oracle shape for
  each case. It is not approved or immutable until contract-freeze review.

## Required additions

The corpus still needs fixtures for malformed UTF-8, layout recovery, record recursion,
wrong-kind references, missing/duplicate fields, resource
boundaries, invalid encoded IR, logical alpha-equivalence, source maps,
provenance, and deterministic concurrent compilation.
