# Neutral language v0 decisions

Status: proposed coherent baseline

These decisions specify only the minimum v0 language described by
[architecture.md](../architecture.md). They do not plan later versions.
Implementation order and release gates are defined in
[the v0 roadmap](../../../ROADMAP.md).

## Decision files

1. [Governing boundaries](01-governing-boundaries.md)
2. [Lexical source text](02-lexical-source-text.md)
3. [Document shape](03-documents-modules-profiles.md)
4. [Declarations and names](04-declarations-bindings-names.md)
5. [Types and records](05-type-schema-notation.md)
6. [Literal and constructed values](06-literal-values.md)
7. [Identity references](07-references-relationships.md)
8. [Minimal vocabulary boundary](10-domain-vocabulary.md)
9. [Security and limits](12-security-sensitive-syntax.md)
10. [Diagnostics](13-diagnostics-invalid-syntax.md)
11. [Formatting](14-documentation-formatting-tools.md)
12. [Evolution and conformance](15-evolution-conformance.md)

## Complete v0 surface

```text
neu "0.1"
module example
[use Vocabulary]

record Name { ... }
Type binding = value
```

Core types are `num`, `string`, `bool`, `T?`, `List<T>`, `Ref<T>`, user
records, and qualified vocabulary types. Values are scalar/null literals,
contextual records, lists, ordinary binding reuse, and `ref(name)`.

Every binding is immutable and explicitly typed. One source unit defines one
module; all declarations are exported.

## Excluded from v0

v0 has no namespace or visibility syntax, multiple units, imports, secrets,
static/member selection, operators, functions, control structures, mutation,
composition, templates, macros, executable plugins, external effects, or
application-specific declarations.

An excluded feature requires a later independent proposal; these files make no
promise that it will be added.
