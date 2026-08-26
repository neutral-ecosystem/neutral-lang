# Neutral language v0 showcase

This document demonstrates every v0 source feature in one domain-neutral model.
The language describes typed immutable data and identity links; it performs no
runtime action.

## Complete example

```neu
neu "0.1"
module showcase

use Fixture

record Config {
    string image,
    string? note = null,
    List<string> labels = [],
}

record Selection {
    Ref<Config> config,
}

record Pair {
    num left,
    num right,
}

string image = "example.invalid/tool:1"
string copied_image = image
bool enabled = true
num ratio = 0.5

Config config = {
    image: copied_image,
    labels: ["portable", "typed"],
}

Selection selection = {
    config: ref(config),
}

Pair pair = {
    left: 10,
    right: ratio,
}

Fixture::Metadata metadata = {
    label: "generic probe",
}
```

## Headers

```neu
neu "0.1"
module showcase
```

Both are mandatory and canonical. One compilation contains one source unit and
one logical module.

## Optional captured vocabulary

```neu
use Fixture
```

At most one vocabulary is used. The host captures its exact bundle; source does
not download or select it. The bundle is closed data and may supply nominal
schemas such as `Fixture::Metadata`.

## Immutable explicit bindings

```neu
num count = 10
string label = "sample"
bool enabled = true
```

Bindings need no `let`, never need semicolons, and cannot be reassigned. Every
declaration spells its type.

## Scalars

The scalar types are `num`, `string`, and `bool`.

```neu
num exact_decimal = 0.1
string text = "hello\nworld"
bool active = false
```

`num` is exact in Neutral IR. v0 does not convert it to a target integer or
floating-point representation.

## Nullability

```neu
string? note = null
List<string>? names = null
```

`?` belongs to the type. `null` is the only null literal. It is invalid for a
non-nullable expected type.

## Lists

```neu
List<string> labels = ["one", "two"]
List<string> empty = []
```

Lists are ordered and homogeneous. Context supplies the element type of `[]`.

## Records and defaults

```neu
record Config {
    string image,
    string? note = null,
    List<string> labels = [],
}
```

Records are nominal. A field without a default is required. A default makes the
field omittable but does not change its nullability.

Defaults are closed constants and cannot read another binding or create an
identity reference.

## Contextual record construction

```neu
Config config = {
    image: "example.invalid/tool:1",
}
```

The binding supplies the expected type. `note` and `labels` use their defaults.
Logical IR contains the final values; provenance says which values came from
defaults.

Field shorthand and anonymous record types are absent.

## Ordinary value reuse

```neu
string original = "value"
string copy = original
```

`copy` has the logical string value `"value"`. Provenance records the reuse
edge. Forward reuse is allowed:

```neu
string copy = original
string original = "value"
```

A cycle is invalid:

```neu
string first = second
string second = first
```

## Identity references

```neu
record Selection {
    Ref<Config> config,
}

Selection selection = {
    config: ref(config),
}
```

`ref(config)` links the declaration identity. It does not copy the record value
and implies no containment, ownership, dependency, or order. Forward identity
references are allowed and do not participate in value-cycle detection.

## Reference-only recursion

```neu
record Node {
    Ref<Node>? next,
}
```

This is valid because the recursion crosses `Ref<Node>`. Embedded recursion is
invalid:

```neu
record Node {
    Node? next,
}
```

## Vocabulary-owned typed data

```neu
Fixture::Metadata metadata = {
    label: "sample",
}
```

The exact captured vocabulary schema defines the fields. The compiler validates
them and emits qualified typed data. No vocabulary code runs, and the generic
probe does not assign application meaning.

## Qualification

`::` is used only for `Vocabulary::Type` in v0. These are invalid:

```neu
module package::module
Fixture::Metadata.value
config.image
```

There are no local namespaces, module paths, static values, or member access.

## Comments

```neu
// line comment

/* non-nesting
   block comment */
```

Comments are non-semantic trivia and do not attach documentation.

## Type compatibility

Exact type identity is required except that `T` may widen to outer `T?`:

```neu
string name = "sample"
string? maybe_name = name
```

Generic arguments are invariant. `List<string>` cannot initialize
`List<string?>`.

## What reaches IR

The public logical payload contains:

- logical module and optional vocabulary identities;
- declaration identity plus declaration fingerprint;
- resolved types and final immutable values;
- document-local reference edges; and
- required structural features.

Source maps record where elements came from. Provenance records explicit source,
value reuse, and defaults. Derivation records the captured compilation inputs.

`ElementId` is a graph-local label. Logical equality and deterministic output are
defined modulo consistent `ElementId` renaming.

## What v0 deliberately omits

- namespaces and visibility modifiers;
- multiple source units and imports;
- secret references;
- static/member selection;
- maps, sets, tuples, unions, and enums;
- operators, expressions, functions, and control structures;
- mutation, override, composition, templates, and macros;
- executable plugins or external evaluation; and
- application-specific syntax or runtime behavior.
