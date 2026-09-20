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

Vocabulary contracts can add closed nominal data types, fields, constants,
public-type declarations, and structural capabilities. They cannot add
callbacks, scripts, native modules, validators, acquisition, hidden imports,
runtime execution, or Flow behavior.

`url` and `path` are distinct inert scalar values. Neutral validates bounded
literal representation and carries them as data; it never fetches, opens,
normalizes, resolves, or authorizes them. A vocabulary may define a later
data-level interpretation.

Vocabulary authoring metadata is a separate versioned, data-only input. It
only influences descriptors/presentation and cannot alter core semantics,
capture, source parsing, IR, identity, or execution.
