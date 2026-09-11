# Section 5: types and records

## SYN-TYP-001 — Core types

v0 predeclares:

- `num`: exact base-10 rational;
- `string`: Unicode scalar sequence;
- `bool`: `true` or `false`;
- `List<T>`: ordered homogeneous list;
- `Ref<T>`: document-local identity link; and
- postfix `T?`: nullable type.

There is no standalone null type. `List` and `Ref` take exactly one invariant
type argument. No other generic type is core.

## SYN-TYP-002 — Exact numeric semantics

`num` uses the host-independent exact rational rules in the lexical decision.
v0 has no target numeric lowering and numeric spelling introduces no source
subtyping.

## SYN-TYP-003 — Nominal records

```neu
record Config {
    string image,
    string? note = null,
    List<string> labels = [],
}
```

Record names are uppercase-leading. Field names are `snake_case`, unique within
the record, and followed by commas. Records are nominal: matching fields do not
make two differently named records compatible.

Every nominal recursive record cycle is invalid unless the cycle crosses
`Ref<T>`. `Node?` and `List<Node>` still embed `Node`; `Ref<Node>` links identity
without embedding and is allowed.

## SYN-TYP-004 — Lists

`List<T>` is ordered and homogeneous. List order is logical. `T` is invariant,
and an empty list requires an expected element type.

## SYN-TYP-005 — Field state

Required/defaulted and non-nullable/nullable are independent:

| Form | Presence | Nullability |
| --- | --- | --- |
| `string name,` | required | non-null |
| `string? note,` | required | nullable |
| `string name = "x",` | omittable | non-null |
| `string? note = null,` | omittable | nullable |

Omission is structural and is not a source value. v0 has no `optional`,
`absent`, or `none` construct.

A user-record default is a closed constant: scalar/null literal, list, or
contextual record recursively composed from closed constants. It cannot contain
an ordinary binding name or `ref(...)`.

## SYN-TYP-006 — References and compatibility

Source assignment compatibility has only:

1. exact resolved type identity; or
2. widening non-nullable `T` to outer nullable `T?`.

`List<string>` may initialize `List<string>?`, but not `List<string?>`.
`List<T>` and `Ref<T>` are invariant. Numeric target lowering is outside v0.

## SYN-TYP-007 — Vocabulary-owned types

An imported vocabulary may contribute nominal types referenced as
`Vocabulary::Type`. The exact captured bundle owns their field schemas and
closed defaults. They remain inspectable typed data in IR.

v0 has no vocabulary static values, enum-case selection, or general `.` access.
