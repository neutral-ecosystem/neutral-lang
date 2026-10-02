<!-- SPDX-License-Identifier: Apache-2.0 -->

# Vocabulary contract

Status: accepted portable baseline

v1 supports zero or more exact captured data-only vocabularies:

```neu
use ExampleDomain as domain
```

Each source alias is local. Logical IR records canonical vocabulary identity
and exact semantic revision, never the spelling of the alias. The supplied
locks must be the exact cover of identities required by source: missing, extra,
unused, duplicate, and conflicting locks fail capture. A canonical vocabulary
identity has one semantic revision throughout a project.

## Source requirements and exact cover

After the module header, zero or more `use Identity as alias` lines precede
imports and declarations. `Identity` is an uppercase-leading ASCII identifier;
`alias` is a lowercase `snake_case` name. The same identity may be required by
several modules, each with its own alias, but may occur only once in a module.
An alias may not duplicate another vocabulary/import alias or shadow the final
segment of its own module name. Different aliases never create different
semantic vocabulary identities.

For the complete captured source closure, form the set of required canonical
identities. It must equal the set of supplied locks exactly. Each lock contains
one identity, exact semantic release, encoding/schema versions, required
features, and captured-byte digest. Duplicate identities are rejected even
when their revisions or bytes agree; different revisions of one identity are
therefore also rejected. Missing/extra locks and byte-digest mismatches reject
the entire capture, with no partial project. Lock order and source alias
spelling do not change canonical identities.

## Closed data and visibility

Vocabulary contracts can add closed nominal data types, fields, constants,
public-type declarations, and structural capabilities. They cannot add
callbacks, scripts, native modules, validators, acquisition, hidden imports,
runtime execution, or Flow behavior.

Only explicitly public, source-authorable nominal types may appear in source
signatures or public exports. A public vocabulary type must not transitively
expose a private vocabulary type through its fields. Private types may support
internal closed defaults but cannot be named by source. Type identity is
`(canonical vocabulary identity, locked semantic release, nominal type name)`;
source aliases, bundle JSON order, authoring metadata, and host locations are
excluded. Unknown members and executable-shaped members fail closed before
publishing a validated contract.

The v1 project bundle is a closed JSON envelope with exact `format`,
`encoding_version`, `schema_version`, `identity`, `version`,
`required_features`, and `types` members. Encoding and schema version are
`1.0`. Each type has exactly `name`, `public`, and `fields`; each field has
exactly `name` and `type`. `public` is a JSON Boolean. Field `type` is one of
`num`, `string`, `bool`, `url`, `path`, or a nominal name in the same bundle.
Duplicate type/field names and unknown targets fail. Public types may refer
only to public types. This initial v1 shape intentionally has no executable
extensions, defaults, or authoring metadata; adding those requires a new
reviewed schema revision, not silent interpretation of an unknown member.

## Inert location values

`url` and `path` are distinct inert scalar values. Neutral validates bounded
literal representation and carries them as data; it never fetches, opens,
normalizes, resolves, or authorizes them. A vocabulary may define a later
data-level interpretation.

Source uses the exact type names `url` and `path`, with ordinary quoted string
literals as initializers. The decoded text is retained byte-for-byte as a
Unicode scalar sequence; no URI parsing, filesystem interpretation, separator
rewriting, case folding, or percent decoding occurs. Their IR variants remain
distinct from each other and from `string`, even when their text is equal.

Vocabulary authoring metadata is a separate versioned, data-only input. It
only influences descriptors/presentation and cannot alter core semantics,
capture, source parsing, IR, identity, or execution.
It is not a member of a captured vocabulary bundle, lock, or project request;
an in-bundle `metadata` member is unknown and rejected. The separate authoring
input schema and validation belong to Stage 8. No Stage 5 producer may invent
semantic defaults from that later presentation channel.
