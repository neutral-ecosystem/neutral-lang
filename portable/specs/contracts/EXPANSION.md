<!-- SPDX-License-Identifier: Apache-2.0 -->

# Finite description expansion — review candidate

Profile: `neutral.expansion/1`, covering V1-CONS-015..016 and requiring
[expression/1](EXPRESSIONS.md) and [evaluator/1](EVALUATOR.md).
Status: review candidate, not frozen or activated. Source/profile, wire, identity
and authoring compatibility/vector activation is a separate gate.

## Source and typing

```neu
template Label(static string prefix, deferred string suffix) -> string {
    prefix ++ suffix
}
public string label = instantiate Label(prefix: "build-", suffix: name)
public expand labels = Label(prefix: platform ++ "-", suffix: architecture)
    for (platform in ["linux", "windows"], architecture in ["arm64", "x64"])
    where true
    exclude [{ platform: "windows", architecture: "arm64" }]
```

A template is `template Name(parameters) -> Type { expression }`, optionally
public. Parameters have explicit `static` or `deferred` phase and exact types;
optional `= expression` defaults are definition-scoped and may refer to earlier
parameters only. A required parameter cannot follow a defaulted one. No ambient
inputs or private caller names are implicitly captured. The body can refer to
parameters and explicitly named public constants/types resolved at definition,
using the defining module's normal local/import-alias resolution. These named
references cannot be rebound to caller names. A public template exposes only
public transitive parameter/result/default/body contracts.

`instantiate Name(name: expression, ...)` binds named arguments exactly once,
rejecting missing/extra/duplicate arguments. Defaults apply to omitted arguments
only, never explicit null. Static parameters require fully static values; deferred
parameters accept static values or typed residual expressions. Result restrictions
remain obligations when deferred. Template names are not values and cannot be
returned, stored, passed as parameters or invoked during consumer evaluation.

Resolve explicit argument expressions in caller scope and defaults in definition
scope, then bind in declared parameter order. Evaluate/check static arguments in
that order even when unused; they are part of the expansion request. Keep deferred
argument expressions beneath their selected body guards rather than eagerly
evaluating them at substitution. All argument types are checked regardless of use.

Substitution uses resolved identities, never text replacement. Parameters/binders
cannot capture caller symbols; generated refs retain their stable resolved targets.
Reject recursion in the template application graph, including cycles in unselected
branches. Type-check all template bodies before expansion. Arithmetic in unused
definitions is not evaluated merely to type-check them. Invoked selected bodies
obey expression guard/evaluation rules and independent aggregate budgets.

## Matrix expansion

`[public] expand Name = Template(arguments) for (axis in expression, ...)`
has optional `where expression` and `exclude [record, ...]`, in that order.
Each axis is a fully static homogeneous list, nonempty or empty; axis names are
unique lexical binders. Each exclusion has exactly all axes, with exact element
types. Duplicate exclusion tuples reject; unknown axes/values reject. `where`
is a static Boolean expression over axis values/static constants; no deferred
value may influence shape. There is no general statement-level `for` or `while`.

Enumerate the Cartesian product in declared-axis order, first axis slowest.
Evaluate `where`, then exact tuple exclusions, then the retained template body.
Empty axes produce zero instances. Duplicate axis values are allowed: positions,
not values, determine generated names. Preflight the full Cartesian cardinality
and charge visited/filter work even when all tuples are excluded. Deferred inputs
may populate resulting descriptions but cannot determine generated declaration
cardinality, names, refs/topology, axes, filter or exclusion decisions. This is
expansion structure, not magic interpretation of consumer-specific graph fields.

Each retained instance is a binding `Name__i0__i1...` using original zero-based
decimal axis positions, never a runtime counter or renumbered filtered position.
All generated and explicit declarations share the module collision check; reject
collisions rather than rename them. A one-axis expansion uses `Name__i0`.
Plain instantiation has its explicitly declared binding name. No synthetic matrix
binding named `Name` is published. References to generated bindings use those
stable names and normal visibility/type rules. Generated names must fit existing
identifier byte limits, checked before string construction.

## Meaning, attribution and limits

Semantic IR contains expanded typed values/residual expressions, not runtime
template definitions. The source-authoring model retains template definitions,
applications, axes, filters/exclusions and mappings separately; do not reconstruct
original source from expanded values. Unused private template definitions affect
captured evidence, not logical meaning. Public template contracts remain exported
semantic facts; their meaning cannot be omitted from the new identity profile.

Equivalent materialized expansions and explicit declarations have equal logical
meaning only with identical public symbols and exported contracts. Parameter and
axis changes affect logical identity when they alter expanded meaning/public
contracts; captured/derivation facts retain source/expansion changes separately.
Use new identity/wire/authoring profiles and vectors, not edits to frozen /1 or /2.

Attribution links each expanded value to the application, definition and relevant
argument/axis occurrence. Report checked public locations or explicit redaction;
never fabricate byte spans for generated source. Expanded and source-preserving
views have distinct round-trip tests. Consumer evaluation cannot instantiate a
template or dynamically fan out operations.

Independent positive hard ceilings: 1,000,000 template definitions/applications
each, 64 parameters and 16 axes per application, 1,000,000 items per axis,
1,000,000 full Cartesian tuples per application and aggregate visited tuples,
1,000,000 aggregate expanded bindings/nodes each, 64 expansion depth,
100,000,000 work visits, 67,108,864 retained/output bytes each. Intersect existing
expression/reader/capture limits. Checked multiplication precedes product
allocation; nested expansions share aggregate budgets and cannot reset them.
Filters do not bypass product/work limits. Cancellation/fallible retention applies
to binding, substitution, naming, mappings and expansion publication.

Diagnostics: NEU-TPL-001 unsupported profile; 002 grammar; 003 argument/name/scope
error; 004 type/static-phase mismatch; 005 recursive application; 006 generated
name collision; 007 invalid axis/filter/exclusion; 008 resource/allocation failure;
009 cancellation. Sort by stable module/application identity, original byte range,
code and original axis-index tuple. No failure returns partial expanded IR or
replaces a successful cache generation. Literal vectors must cover hygiene,
defaults/null, ordering, duplicates/collisions, filtered one-over products,
deferred-shape rejection, attribution and explicit-expansion identity equivalence.
