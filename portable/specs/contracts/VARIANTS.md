<!-- SPDX-License-Identifier: Apache-2.0 -->

# Closed tagged variants

Status: frozen shared contract, 06-10-2026; not active source syntax

V1-VOC-009 supports **both** source-declared and vocabulary-declared nominal
variants. They share one resolved type/value model and validation algorithm;
JSON transport must not introduce a second type system. This document refines
[consumer readiness](CONSUMER-READINESS.md). It does not amend an existing
bundle, wire, source, or identity profile in place.

## Source declarations

Frozen declaration grammar (explicit composition-capable capture /2 only):

```text
variant_declaration = ["public"] "variant" UpperName "{" alternatives "}"
alternatives = alternative {"," alternative} [","]
alternative = type snake_name
```

The existing type grammar supplies scalar, nominal, qualified nominal,
`List<T>`, `Ref<T>`, and nullable payload types. A variant has at least one
alternative. Tags are unique, case-sensitive, non-protected `snake_case` names;
each selects exactly one payload type. Alternatives cannot declare defaults,
inherit another variant, add wildcard tags, or omit their payload type.
The declaration's trailing comma is optional, consistent with record declarations.

```neu
neu "1.0"
module example

public variant Outcome {
    string success,
    num failure
}

public Outcome answer = {
    tag: "success",
    payload: "complete",
}
```

`variant` is contextual at a declaration start under explicitly selected
`tagged-variants-v1` from [COMPOSITION-PROFILE](COMPOSITION-PROFILE.md).
Do not add it to the frozen v0 protected-name set or reinterpret existing
bindings named `variant`. `neu "0.1"` never accepts variant declarations.
Public/private visibility, imported type qualification, alias resolution,
source attribution, and public signature closure follow the existing nominal
type rules. Naming a private imported variant remains forbidden.

## One contextual value form

Values use existing record-literal syntax, with exactly `tag` and `payload`:

```neu
Outcome failure = { tag: "failure", payload: 3, }
List<Outcome> outcomes = [answer, failure]
Ref<Outcome> chosen = ref(answer)
```

The expected variant type gives this literal its meaning. `tag` must be an
exact string literal naming a declared alternative; it is not an expression,
implicit discriminator, or inferred value. `payload` is checked against that
alternative's complete resolved type, including restrictions and public closure.
Field order does not change meaning. Ordered list elements retain their order.

Missing/duplicate/unknown fields, unknown tags, wrong payload types, missing
payloads, and two simultaneous alternatives fail. Explicit `payload: null` is
valid only for a nullable payload type. A defaulted record payload may be `{}`
only if that record's own field rules allow it. Omission is never a unit value
or an implicit null. No extra unit/void scalar is introduced.

Ordinary immutable reuse and typed references work for variant values. Reuse
cannot switch the selected tag or reinterpret the payload. References identify
a binding whose **whole variant type** matches, not just a tag or payload field.
Member selectors remain the separately scheduled Stage 8 boundary.

## Vocabulary declarations

The frozen composition schema has a closed variant definition alongside
record definitions. Its variant member set is exactly `kind`, `name`,
`public`, and `alternatives`; each alternative has exactly `tag` and `type`:

```json
{
  "kind": "variant",
  "name": "Outcome",
  "public": true,
  "alternatives": [
    { "tag": "success", "type": { "kind": "string" } },
    { "tag": "failure", "type": { "kind": "num" } }
  ]
}
```

This is a **type entry**, not a complete bundle envelope.
The [composition contract](VOCABULARY-COMPOSITION.md) defines its complete
envelope and the shared recursive type object shapes.
The composition schema/version and exact feature selection are frozen separately
from implementation and activation. The existing project schema
`1.0` still accepts only its closed record definition; neither a `kind` nor an
`alternatives` member may become valid there. v0 bundle behavior is unchanged.

A vocabulary variant is used like any qualified vocabulary nominal type:

```neu
use ExampleDomain as domain
public domain::Outcome answer = { tag: "success", payload: "complete", }
```

Public variants close over every alternative's transitive payload types, not
only the alternative selected by one value. Cross-vocabulary alternatives use
canonical identity/revision/type dependencies with the same exact supplied
transitive-lock rules as records; source alias spellings cannot enter a bundle.
No acquisition or executable dispatch occurs when a tag is validated.

## Nominality, cycles, and identity

Source variant identity is its stable module-symbol identity. Vocabulary variant
identity is `(canonical vocabulary identity, exact semantic revision, type name)`.
An alternative's identity additionally includes its tag. Different owners or
tags never become interchangeable because their payload types are equal.
Source and vocabulary variants remain distinct unless they name the same
resolved nominal owner; matching declaration text is not an implicit conversion.

Embedded-type cycle analysis traverses every alternative, including branches
not selected by a binding. Nullable/list wrappers do not hide embedded cycles.
Typed-reference edges are distinguished from embedding edges. Reference identity
cycles are not evaluation cycles; ordinary reuse/non-reference value cycles still
reject. A reference never embeds, evaluates or expands its target.

The new identity profile includes the complete sorted alternative catalogue,
public visibility, resolved payload types/defaults/restrictions, selected tag,
and typed payload value. Alternative declaration order is non-semantic;
renaming a tag, changing a payload contract, or selecting another tag is semantic.
Aliases, roots, host mappings, source spans, and presentation metadata remain
excluded from complete logical meaning. Old identity profiles and literal
vectors remain unchanged; adding variant transcript tags to them is forbidden.

## Public boundaries and limits

The complete IR/wire/reader contract must expose the variant owner, all typed
alternatives, selected tag, payload, and reference dependencies without compiler
linkage or source reparsing. Public views retain that interpretive closure and
redact private implementation provenance. The probe reports both declaration
origins without treating any alternative as executable.

Require independent bounds for alternatives per variant, total alternatives,
type/value depth, nodes, strings, references, and output bytes. Validate before
proportional allocation; arithmetic/allocation failures and cancellation must
not publish partial variants. Exact ceilings, diagnostic codes, version selectors
and transcript grammar are specified in [COMPOSITION-PROFILE](COMPOSITION-PROFILE.md),
not inferred from examples.

## Fixture and activation gate

The [original design examples](../decisions/variants/README.md) cover
both origins, imported public source variants, mixed collections, wrong tags,
wrong payloads, duplicate tags, and private signature closure. They are not
active conformance or passing compiler evidence yet. Their literal source bytes
are now captured in the registered `composition-contract` request corpus with
fixed positive/error oracles and immutable identity vectors.

Before activating that frozen suite, implement the shared
model through compiler, IR, wire, independent reader/probe, views, and cache
equivalence. Parser-only support does not close V1-VOC-009 or promotion.
