# Section 4: declarations and names

## SYN-DEC-001 — Immutable type-first bindings

Bindings use:

```neu
num retry_count = 3
string label = "sample"
```

There is no `let`. Every binding is immutable.

Each declaration receives module-symbol identity, a separate declaration
revision/fingerprint, graph-local IR element identity, and source provenance.
Identity is not derived from array position or parser traversal.

## SYN-DEC-002 — Explicit types

Every binding declaration spells its complete type. v0 does not infer a
declaration type. Context may type an inner list or record literal only after the
declaration or field supplies the expected type.

Vocabulary-owned values use the same binding production:

```neu
Fixture::Metadata metadata = {
    label: "sample",
}
```

This is an ordinary typed binding, not a special declaration kind.

## SYN-DEC-003 — No mutation

`mut`, reassignment, compound assignment, override, mutation methods, and
assignment statements are invalid. `=` occurs only in binding initialization
and field-default declarations.

## SYN-DEC-004 — Duplicates and protected names

All records and bindings occupy one module scope. Duplicate declarations are
invalid. Core lexical/type names are protected and cannot be redeclared.
Vocabulary type names remain inside the imported vocabulary namespace.

## SYN-DEC-005 — Forward resolution

The compiler collects declarations before resolving binding values and
`ref(...)` targets. Both may point to later bindings.

Ordinary value uses form a static dependency graph. Every cycle in that graph is
invalid. `ref(...)` identity edges do not evaluate the target and are excluded
from value-cycle detection.

```neu
string second = first
string first = "value"
```

The example is valid. Two bindings that use each other's values are invalid.
