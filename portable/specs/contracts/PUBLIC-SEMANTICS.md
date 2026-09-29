<!-- SPDX-License-Identifier: Apache-2.0 -->

# Public project semantics contract

Status: accepted Stage 4 contract; complete project IR and authoritative reader publication remain Stage 6 work

This contract refines [SOURCE](SOURCE.md) and [MODULE-GRAPH](MODULE-GRAPH.md).
It changes neither the captured request nor the v0.1 profile. All resolution
uses only the complete, immutable captured source closure and its validated
module graph. A graph SCC is legal even when its members import each other;
only a semantic dependency cycle is rejected.
The Stage 4 source analyzer caps recursive type and contextual-value nesting at
64 levels, within the independently bounded captured source bytes; deeper
forms fail closed before semantic publication.

## Declaration syntax and visibility

After imports, a root declaration is either `record Name { ... }` or an
explicitly typed immutable binding. A single leading `public` token makes that
root public; absence makes it private. `public` cannot prefix headers, uses,
imports, fields, or values, and cannot appear twice. Record fields inherit
their containing type's visibility. Within its own module, a declaration may
name private or public roots. Import aliases occupy the module namespace and
cannot be root declaration names. No re-export syntax exists.

An imported root is addressed only as `alias::name`, where `alias` is an
explicit import in the current unit and `name` is public in the target module.
Unqualified names resolve only in the current module, never by searching
imports or by using the imported module's final segment. The same rule holds
for nominal types, value reuse, and `ref(...)` targets. A bare `module::name`
without an import alias is not a fallback. A private imported name behaves as
inaccessible, including in diagnostics; public diagnostics do not reveal its
type, value, or source text.

## Types and values

Core scalar types, `List<T>`, `Ref<T>`, and one outer nullable `?` retain the
v0.1 type rules. Nominal types are identified by `(profile, full module ID,
root name)`, never by an import alias, capture order, or source ID. Type
compatibility is exact nominal identity plus the inherited structural rules
for constructors; structurally identical records from different modules are
not interchangeable. A public root's declared type and every transitively
reachable field type of a public nominal record must name only public nominal
types. A public value may depend on a private local value, because private
implementation dependencies do not become exported names. Its public
signature still cannot mention a private type.

Qualified value reuse reads a public immutable binding in the imported module.
An ordinary local reuse may name a private local binding. Resolution checks
the declared type against the target value's resolved type and records a
source-accounted edge from the consuming root to the target's stable
module-symbol identity. A `ref(name)` or `ref(alias::name)` is an identity
edge, not a value copy; its target must be a binding of the exact `Ref<T>`
target type. A public binding's exposed reference cannot target a private
binding, even in its own module. Any reference reachable through a public
value must obey the same rule. Private bindings may reference private local
bindings. An imported reference target must always be public.

Edges are keyed by `(source profile, full module ID, root name)` for each
endpoint; aliases and source locations are retained only as provenance. This
key is stable under source ordering, capture order, and import-alias renaming.
The project semantic model retains each occurrence's original-byte source
location separately from the canonical edge identity. Stage 6 is responsible
for contextual value validation/lowering, and for publishing the export index,
source maps, and redacted public reader view. A successful Stage 4 resolution
model is not an authoritative compiled project and does not activate `neu
"1.0"` in the standalone compiler.

## Stage 4 public interface snapshot

Before the complete Stage 6 project IR, the compiler may produce a narrow
in-process public-interface snapshot. It contains only explicitly public
module-symbol identities, canonical binding type signatures or record field
name/type signatures, and public-to-public type, value-reuse, and identity-ref
edges. A dependency on a private local value is absent from this snapshot;
source IDs, byte spans, captured bytes, private roots, private edge endpoints,
and raw provenance are never members of the public snapshot. The reader
rejects unordered/duplicate exports or edges, dangling or non-public nominal
type targets, type edges inconsistent with public signatures, structurally
incompatible edge categories, excess type nesting, and a stale
fingerprint before exposing a view. An independent consumer can enumerate
cross-module value and reference edges using only the reader contract.
The reader checks internal consistency of the supplied public-only snapshot;
without the complete project IR it cannot independently prove that a producer
did not omit or misclassify a declaration. The compiler owns that projection,
and Stage 6 will validate the authoritative complete-project boundary.

The snapshot fingerprint is an NHT-v1 SHA-256 transcript in the
`neutral/project-interface/v1` domain over the exact profile, canonically
ordered public symbols, their type signatures, and public-to-public edges.
Alias spelling, captured source order, source locations, and private
implementation values do not enter it. It is a *public signature fingerprint*,
not a full logical-project identity or a guarantee about record default values
or contextual value lowering. Stage 6 owns complete validated project IR and
the authoritative public view; Stage 7 owns the complete project identity
chain.

## Semantic dependency and diagnostic rules

Construct the declaration dependency graph across all modules before value
evaluation. Ordinary reuse, reference targets, and nominal field-type edges
are distinct typed edges. A cycle through ordinary immutable value reuse or
embedded nominal records is invalid, including one crossing an import SCC.
Identity-only `ref` edges do not evaluate target values and do not by
themselves make a cycle. Acyclic immutable roots are ordered dependency-first,
with stable module-symbol identity as the tie-breaker; Stage 6 performs
contextual value validation/lowering. No authoritative project result is
published on failure, cancellation, or a structural-limit breach.

Stage 4 diagnostics use the source occurrence as primary location and are
ordered by `(full module ID, original byte start, diagnostic code, target
module ID, target name)`. An inaccessible name uses the same code whether the
target is private or absent. Multiple errors are truncated only after this
ordering, with an explicit limit failure rather than partial success.

| Code | Stable failure |
| --- | --- |
| `NEU-XMOD-001` | Illegal cross-module semantic value or embedded-type cycle. |
| `NEU-XMOD-002` | Invalid `public` placement or duplicate modifier. |
| `NEU-XMOD-003` | Unqualified imported access, absent alias/name, or private imported name. |
| `NEU-XMOD-004` | Public signature transitively exposes a private nominal type. |
| `NEU-XMOD-005` | Exposed reference targets a private binding. |
| `NEU-XMOD-006` | Incompatible cross-module nominal or value type. |
| `NEU-XMOD-007` | Invalid reference target or unbound root. |
| `NEU-XMOD-008` | Semantic traversal or declaration bound exceeded. |
| `NEU-XMOD-009` | Cancellation observed before semantic publication. |
| `NEU-XMOD-010` | Caller supplied a graph from a different captured closure. |
| `NEU-XMOD-011` | Declaration or value violates inherited source grammar. |

The inherited v0.1 diagnostic catalogue remains unchanged. Stage 4 errors
never authorize host acquisition, path lookup, or a fallback profile.
