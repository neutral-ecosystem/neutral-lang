# Neutral v0 conformance fixtures

These fixtures cover the reduced, domain-neutral v0 language.

## Positive source

- [immutable-value-reuse.neu](positive/immutable-value-reuse.neu) distinguishes
  ordinary value reuse from identity references and proves forward resolution.
- [defaults-compatibility.neu](positive/defaults-compatibility.neu) covers
  defaults, nullability, lists, and outer nullable widening.
- [minimal-vocabulary.neu](positive/minimal-vocabulary.neu) proves one captured
  data-only vocabulary through the generic source-to-IR boundary.

## Negative source

- [generic-covariance.neu](negative/generic-covariance.neu): invariant generic
  argument violation.
- [module-path.neu](negative/module-path.neu): module qualification is absent.
- [mut-modifier.neu](negative/mut-modifier.neu): mutation is absent.
- [namespace-declaration.neu](negative/namespace-declaration.neu): namespaces
  are absent.
- [nonconstant-default.neu](negative/nonconstant-default.neu): defaults cannot
  read bindings.
- [reassignment.neu](negative/reassignment.neu): reassignment is absent.
- [value-cycle.neu](negative/value-cycle.neu): immutable value cycle.
- [version-escape.neu](negative/version-escape.neu): escaped version spelling.
- [version-leading-zero.neu](negative/version-leading-zero.neu): noncanonical
  version spelling.
- [visibility-modifier.neu](negative/visibility-modifier.neu): visibility syntax
  is absent.
- [vocabulary-name-collision.neu](negative/vocabulary-name-collision.neu): the
  imported vocabulary namespace cannot be redeclared.

## Contract matrix

- [vocabulary-contract-cases.md](vocabulary-contract-cases.md) specifies captured
  vocabulary resolution, closed-schema validation, and external-reader cases.

## Required additions

The corpus still needs fixtures for malformed UTF-8, layout recovery, strings,
exact numeric normalization/limits, record recursion, wrong-kind references, missing/duplicate fields, resource
boundaries, invalid encoded IR, logical alpha-equivalence, source maps,
provenance, and deterministic concurrent compilation.
