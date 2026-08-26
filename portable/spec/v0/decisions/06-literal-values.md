# Section 6: literal and constructed values

## SYN-VAL-001 — Scalars and null

`true`/`false`, numeric literals, quoted strings, and `null` construct the core
scalar/null values. `null` requires a nullable expected type. Source numeric
spelling lowers to the exact rational specified in the lexical decision.

## SYN-VAL-002 — Contextual records

```neu
Config config = {
    image: "example.invalid/tool:1",
    labels: ["portable"],
}
```

`{ ... }` requires exactly one expected nominal type from the binding, field, or
list element context. Fields use `name: value,`; unknown, duplicate, missing
required, and type-incompatible fields are errors. There are no untyped
anonymous records and no field shorthand.

The same contextual form constructs a captured vocabulary-owned nominal type.

## SYN-VAL-003 — Lists

`[a, b]` is ordered and homogeneous. Each item must satisfy the expected element
type. `[]` is valid only when an expected `List<T>` is available. A trailing
comma is allowed.

## SYN-VAL-004 — Ordinary value reuse

An unqualified binding name in a value position reuses that binding's immutable
logical value:

```neu
string image = "example.invalid/tool:1"
string image_copy = image
```

The logical value of `image_copy` is the string itself; provenance records the
reuse edge. Forward reuse is allowed after full symbol collection. Cycles are
invalid.

## SYN-VAL-005 — Logical values and provenance

When a field is omitted and a default exists, the final logical record contains
the resulting value. Provenance separately classifies it as a user-record or
captured-vocabulary default and records the exact default contract.

Ordinary reuse likewise produces the final reused logical value while
provenance records the source binding. `ref(name)` is not value reuse; its
identity-only rules are specified in the reference decision.

## SYN-VAL-006 — Excluded shorthand and selection

v0 has no static value selection, computed properties, member access,
interpolation, arithmetic, Boolean operators, or comparison expressions.
