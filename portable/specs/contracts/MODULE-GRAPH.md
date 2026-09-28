<!-- SPDX-License-Identifier: Apache-2.0 -->

# Module and import graph contract

Status: accepted Stage 3 contract; core graph implemented, public integration pending

This contract refines [SOURCE](SOURCE.md) and [PROJECT](PROJECT.md) for Stage 3.
It uses the already frozen `neutral.capture/v1` request and its processing
controls; it adds no host resolver or new request field. Stage 2 capture keeps
all supplied units before Stage 3 parses imports and constructs the graph.

## Source grammar and identity

The first significant source line is exactly `neu "1.0"`. After permitted
trivia, exactly one `module` header precedes any `use`, `import`, or declaration.
Its qualified module name has one or more segments separated by exact `::`:

```text
snake_name = [a-z][a-z0-9]*("_"[a-z][a-z0-9]*)*
module_id  = snake_name ("::" snake_name)*
import     = "import" module_id "as" snake_name
```

Spaces or comments may separate tokens under the inherited source layout
rules, but they cannot split `::` or an identifier. Case folding, Unicode
normalization, empty segments, leading/trailing/repeated underscores, dots,
slashes, and URL syntax are never accepted as module identity. Segments must
also be valid non-keyword identifiers under the inherited source lexer. The
source header must agree exactly with the requested module ID. Duplicate module IDs
and source IDs remain Stage 2 `NEU-CAP` failures; exactly one captured source
unit owns each module. A second `module` header in one unit is invalid.

After the module header, zero or more `use` requirements precede zero or more
imports. All imports precede declarations. An import is the exact form above;
the alias is required even if it matches the target's final segment. Source
text cannot select an import version, path, URL, wildcard, relative name,
implicit import, re-export, or additional source unit for a module. An import
references only another module in the complete supplied captured set.

## Alias and edge rules

An alias names one local imported module and shares one namespace with
vocabulary requirement aliases. It may not duplicate another import alias or
vocabulary alias in the same unit, or shadow the unit's own final module-name
segment. An identical target imported twice is invalid even under different
aliases. Alias spelling affects local source resolution and diagnostics, but
does not change the target module's identity or the canonical graph edge.

Every import contributes one directed edge from the importing module to its
target. A self import is invalid. A target absent from the supplied set is
invalid; capture never asks the host to fetch it. Disconnected modules remain
members of the graph, each in its own component unless connected to another
member.

## Deterministic graph and SCCs

Graph construction first validates all module/import syntax and all edges,
then builds an immutable graph. Modules are ordered by exact UTF-8 module ID;
imports are ordered by `(target module ID, alias, original byte offset)` for
source accounting. The graph retains source spans for both module headers and
import occurrences. Input order, map iteration order, thread scheduling,
source IDs, and host locations cannot change graph meaning.

Strongly connected components (SCCs) are computed from the complete graph.
Members of each SCC are ordered by module ID. The condensation graph is
processed dependency-first: an imported component precedes its importer; when
several components are ready, the component with the lexicographically least
member is first. A two-or-more-module import cycle is valid graph structure.
Semantic dependency cycles through values, declarations, or embedded records
are checked separately in Stage 4; a valid import SCC is never rejected solely
because it is cyclic.

## Bounds and failure behavior

The frozen request controls independently bound `imports_per_module`,
`import_edges`, and `scc_units`. Exact limits pass; one over fails. Every
count uses checked arithmetic. Import counts and edges are checked before
proportional edge allocation; SCC size is checked before publishing a graph.
Condensation depth is at most the already bounded `source_units` count, so it
needs no second independent request field. Traversal and SCC computation must
use bounded work without recursive call-stack growth. Cancellation is observed
between bounded graph phases and before publication. On any error there is no
authoritative partial graph or project IR.

## Diagnostic ordering

Graph diagnostics have source locations in the original captured bytes.
Validation visits modules by module ID and imports by original byte offset.
When more than one graph diagnostic is retained, order by `(module ID, source
byte start, diagnostic code, target module ID, alias)`; missing fields compare
as empty strings. A configured diagnostics limit truncates only after this
canonical ordering, and truncation is reported as a limit failure rather than
silent partial success. Parameters may contain bounded logical names but never
host paths, URLs, source text, or credentials.

| Code | Stable Stage 3 failure |
| --- | --- |
| `NEU-MOD-001` | Invalid or repeated module/import syntax or placement. |
| `NEU-MOD-002` | Imported module is absent from the supplied set. |
| `NEU-MOD-003` | Module imports itself. |
| `NEU-MOD-004` | Same target module is imported more than once. |
| `NEU-MOD-005` | Local alias collides with another import, vocabulary alias, or own module name. |
| `NEU-MOD-006` | Forbidden wildcard, relative, implicit, path, URL, or re-export import form. |
| `NEU-MOD-007` | Graph import-count, edge-count, or SCC-size limit is exceeded. |
| `NEU-MOD-008` | Cancellation observed during graph work. |

Stage 2 request/header/duplicate failures retain their `NEU-CAP` codes. Stage 4
reserves `NEU-XMOD-001` for an illegal semantic dependency cycle. The Stage 3
fixture for that case proves the import graph is valid; it is not a Stage 3
semantic rejection claim.
