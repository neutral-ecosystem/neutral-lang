# Proposed Neutral language v0 authoring guide

Status: editable syntax proposal

This guide shows the complete v0 source language. If syntax is not described
here, it is unsupported in v0.

## 1. Complete example

```neu
neu "0.1"
module example

use Fixture

record Config {
    string image,
    string? note = null,
    List<string> labels = [],
}

record Selection {
    Ref<Config> config,
}

string image = "example.invalid/tool:1"
string image_copy = image

Config config = {
    image: image_copy,
    labels: ["portable", "deterministic"],
}

Selection selection = {
    config: ref(config),
}

Fixture::Metadata metadata = {
    label: "v0 probe",
}
```

The captured vocabulary defines only the schema of `Fixture::Metadata`. The
compiler and generic probe assign it no runtime meaning.

## 2. Source-file structure

Every file has this order:

1. `neu "0.1"`;
2. `module snake_case`;
3. optional `use Vocabulary`; and
4. record and binding declarations.

One compilation contains exactly one source file and one module. v0 has no
source import, namespace declaration, nested declaration scope, or multi-file
merge.

## 3. Headers

```neu
neu "0.1"
module example
```

The version spelling is exact. Escapes, ranges, and leading-zero alternatives
are invalid. The module name uses `snake_case`.

## 4. Vocabulary use

```neu
use Fixture
```

At most one `use` is allowed. It introduces an uppercase-leading namespace for
qualified vocabulary types:

```neu
Fixture::Metadata metadata = {
    label: "sample",
}
```

`use` does not download or select a bundle. The host supplies an exact captured
bundle and lock record. v0 supports no `.` selection or vocabulary static value.

## 5. Comments and line endings

```neu
// one-line comment

/* non-nesting
   block comment */

num count = 10 // trailing comment
```

Declarations end at a logical newline. Semicolons are not tokens and are never
needed. Newlines inside `()`, `[]`, and record/value braces do not terminate an
incomplete value.

## 6. Names

Use `snake_case` for modules, bindings, and fields:

```neu
string image_name = "tool"
```

Use an uppercase-leading name for record and vocabulary types:

```neu
record ToolConfig {
    string image,
}
```

`UpperCamelCase` is the style, while the compiler enforces the simpler
uppercase-leading identifier rule. Identifiers are ASCII. Core names cannot be
redeclared.

`::` means vocabulary qualification. `.` has no v0 meaning.

## 7. Immutable bindings

Bindings are type-first and do not use `let`:

```neu
num attempts = 3
string label = "sample"
bool enabled = true
```

Every binding is immutable and explicitly typed. v0 rejects `mut`, reassignment,
compound assignment, override, and inferred declaration types.

Declaration order is non-semantic. A value may refer to a binding declared
later, provided the resulting value-dependency graph is acyclic.

## 8. Core types

| Type | Meaning |
| --- | --- |
| `num` | Exact base-10 rational |
| `string` | Unicode scalar sequence |
| `bool` | `true` or `false` |
| `T?` | Nullable `T` |
| `List<T>` | Ordered homogeneous list |
| `Ref<T>` | Document-local identity reference |

There are no separate source `int`, `uint`, or `float` types. There is no
standalone null type, `none`, or `absent`.

## 9. Numbers

```neu
num whole = 10
num fraction = 10.5
num small = 1e-3
num readable = 16_777_216
```

The compiler stores the exact normalized decimal rational. It does not convert
through a host integer or floating-point type. v0 defines no target numeric
lowering; `0.1` remains exactly one tenth in source and Neutral IR.

## 10. Nullability and field presence

`?` attaches to the type:

```neu
string? note = null
List<string>? labels = null
```

For record fields, presence and nullability are separate:

```neu
record Example {
    string required_text,
    string? required_nullable,
    string defaulted_text = "x",
    string? defaulted_nullable = null,
}
```

An omitted field must have a default. `null` is valid only where the expected
type is nullable.

## 11. Lists

```neu
List<string> labels = ["one", "two"]
List<string> empty = []
```

Lists are ordered and homogeneous. An empty list needs an expected `List<T>`
from its declaration or field. A final trailing comma is allowed.

## 12. Record declarations

```neu
record Config {
    string image,
    string? note = null,
    List<string> labels = [],
}
```

Records are nominal. Fields are explicitly typed, unique, and followed by a
comma. Fields have no visibility modifiers.

Recursive embedding is invalid unless every cycle crosses `Ref<T>`:

```neu
record Node {
    Ref<Node>? next,
}
```

`Node? next` and `List<Node> children` do not break embedded recursion.

## 13. Contextual record values

```neu
Config config = {
    image: "example.invalid/tool:1",
    labels: ["portable"],
}
```

The declaration supplies the expected nominal type. Fields use `name: value,`.
Unknown, duplicate, missing required, and incompatible fields are errors.

v0 has no anonymous record types and no shorthand such as `{ image }`.

## 14. Closed defaults

Field defaults may contain only closed constants: scalar/null literals, lists,
and contextual records recursively made from closed constants.

```neu
record Config {
    string image = "example.invalid/tool:1",
    List<string> labels = [],
}
```

A default cannot read a module binding or contain `ref(...)`.

## 15. Ordinary value reuse

Use a binding name directly to reuse its immutable value:

```neu
string image = "example.invalid/tool:1"
string image_copy = image

Config config = {
    image: image_copy,
}
```

The reused logical value is copied into the resulting value model. Provenance
records that it came from the named binding. A cycle such as `a = b` and `b = a`
is invalid.

## 16. Identity references

Use `ref(name)` when identity—not the value—is required:

```neu
record Selection {
    Ref<Config> config,
}

Selection selected = {
    config: ref(config),
}

Config config = {
    image: "example.invalid/tool:1",
}
```

Forward `ref(...)` is valid. The target must be a value binding of the exact
expected type. `ref(...)` cannot target a record declaration, module, or
vocabulary namespace.

`Ref<T>` does not imply containment, dependency, ownership, order, or runtime
behavior.

## 17. Vocabulary-owned values

The captured bundle supplies the expected fields:

```neu
Fixture::Metadata metadata = {
    label: "sample",
}
```

The compiler validates the payload with the exact captured schema. The public
IR preserves the qualified type identity and typed fields. No vocabulary code is
executed.

## 18. Type compatibility

Compatibility is exact type identity plus one widening rule: non-nullable `T`
may initialize outer nullable `T?`.

```neu
string image = "tool"
string? maybe_image = image
List<string>? maybe_labels = ["one"]
```

`List<string>` cannot initialize `List<string?>`. `List<T>` and `Ref<T>` are
invariant. Different nominal record types are incompatible even with identical
fields.

## 19. Quick reference

| Goal | Syntax |
| --- | --- |
| Version | `neu "0.1"` |
| Module | `module example` |
| Optional vocabulary | `use Fixture` |
| Immutable binding | `num count = 10` |
| Value reuse | `string copy = original` |
| Nullable type | `string? note = null` |
| List | `List<string> names = ["one"]` |
| Record | `record Config { string image, }` |
| Record value | `Config config = { image: "tool", }` |
| Identity reference | `Ref<Config> chosen = ref(config)` |
| Vocabulary type | `Fixture::Metadata metadata = { ... }` |
| Line comment | `// comment` |
| Block comment | `/* comment */` |

## 20. Compact grammar sketch

This sketch omits trivia and static validation details.

```ebnf
source =
    language_header, module_header,
    [ use_declaration ],
    { declaration }, end_of_file

language_header = "neu", '"0.1"', LINE_END
module_header = "module", snake_name, LINE_END
use_declaration = "use", upper_name, LINE_END

declaration = record_declaration | binding_declaration

record_declaration =
    "record", upper_name, "{", { record_field }, "}", LINE_END

record_field = type, snake_name, [ "=", constant_value ], ","

binding_declaration = type, snake_name, "=", value, LINE_END

type = primary_type, [ "?" ]

primary_type =
      "num" | "string" | "bool"
    | upper_name
    | upper_name, "::", upper_name
    | "List", "<", type, ">"
    | "Ref", "<", type, ">"

value =
      boolean_literal | numeric_literal | string_literal | "null"
    | list_value | contextual_record_value | snake_name | reference_value

list_value = "[", [ value, { ",", value }, [ "," ] ], "]"

contextual_record_value = "{", { value_field }, "}"
value_field = snake_name, ":", value, ","

reference_value = "ref", "(", snake_name, ")"

constant_value =
      boolean_literal | numeric_literal | string_literal | "null"
    | constant_list | constant_record

constant_list =
    "[", [ constant_value, { ",", constant_value }, [ "," ] ], "]"

constant_record = "{", { constant_field }, "}"
constant_field = snake_name, ":", constant_value, ","
```

Static validation resolves record and vocabulary types, requires one expected
type for contextual values, validates list homogeneity and nullability, applies
closed defaults, checks exact compatibility, rejects value cycles and embedded
record cycles, validates `ref(...)` target kind/type, and validates the optional
captured vocabulary without executing code.

## 21. Explicitly unsupported

v0 has no namespaces, visibility modifiers, multiple source units, imports,
secret references, `.` selection, extra collection/algebraic types, operators,
functions, control structures, mutation, override, composition, templates, macros,
plugins, external evaluation, or application-specific syntax.
