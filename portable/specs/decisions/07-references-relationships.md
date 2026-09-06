# Section 7: identity references

## SYN-REF-001 — Reference form

Only this form constructs an identity reference:

```neu
Ref<Config> selected = ref(config)
```

The expected `Ref<T>` supplies the target type. `ref(...)` accepts one
unqualified binding name and performs no value evaluation.

## SYN-REF-002 — Target validation

The target must resolve uniquely to a value binding whose type is exactly `T`.
Records, modules, vocabulary namespaces, and unknown names are wrong-kind or
unresolved errors. Forward targets are valid.

## SYN-REF-003 — Identity-only meaning

`Ref<T>` stores document-local target identity and provenance only. It does not
mean containment, ownership, copied value, dependency, order, readiness, or any
runtime relationship. Source position and field names cannot add such meaning.

Identity edges are excluded from ordinary value-cycle detection and break
embedded record recursion.

## SYN-REF-004 — IR identity

The target `ElementId` is authoritative only inside one IR document. Target
declaration kind/type is authoritative; redundant reference constraints must
agree.

Logical IR equality is graph alpha-equivalence under a consistent one-to-one
mapping of all `ElementId` values. Consumers cannot persist an `ElementId` as a
durable or cross-document identity.
