# Section 3: document shape

## SYN-DOC-001 — Language version

The first non-trivia construct is exactly:

```neu
neu "0.1"
```

The version uses canonical decimal `major.minor` with no leading zero except the
single digit `0`. Escapes, ranges, prefixes, suffixes, and alternate spellings
are invalid. v0 accepts only `0.1`.

## SYN-DOC-002 — One source unit and module

The next construct is exactly one module header:

```neu
module example
```

One `CapturedCompilation` contains exactly one source unit and one logical
module. The module name is `snake_case`. `::` is not permitted in a module name.
There is no source-module import or multi-unit merge.

After the header, the root contains an optional vocabulary `use`, then record
and binding declarations.

## SYN-DOC-003 — Optional vocabulary use

Zero or one declaration may appear:

```neu
use Fixture
```

`Fixture` introduces one uppercase-leading vocabulary namespace. It is a logical
requirement, not a path or acquisition command. Host-supplied captured lock data
must map it to one exact permitted bundle identity, content digest, schema
version, and feature set. Missing or mismatched capture fails closed.

The compiler never selects `latest`, searches a registry, or downloads a bundle
because of source syntax.

## SYN-DOC-004 — Exported root

Every valid root record and binding is part of the exported Neutral document.
v0 has no `pub`, `private`, namespace declaration, nested declaration scope, or
public/private projection.

All names share one module scope and must be unique. Declaration order has no
semantic precedence.
