# Section 8: minimal vocabulary boundary

## SYN-VOC-001 — Source surface

v0 permits zero or one vocabulary requirement:

```neu
use Fixture

Fixture::Metadata metadata = {
    label: "sample",
}
```

`use` introduces one namespace. `Vocabulary::Type` is a qualified nominal type;
vocabulary-owned bindings otherwise use the ordinary binding/value grammar.

## SYN-VOC-002 — Exact captured identity

The host's captured lock input maps the source name to one exact vocabulary
identity, content digest, schema version, and required structural feature set.
Source cannot select a version range, request `latest`, name a URL/path, or
trigger acquisition.

The IR records the exact resolved identity/version and the structural features
required by used vocabulary data.

## SYN-VOC-003 — Closed data-only schema

The vocabulary bundle conforms to a fixed versioned Neutral-owned schema. v0
allows nominal type definitions, typed fields, closed constant defaults, and
required structural feature IDs.

It rejects scripts, callbacks, arbitrary expressions, custom validators,
bytecode, native/Wasm modules, and executable entry points. Unknown fields or
required structural semantics fail closed.

Published schema/feature IDs have immutable meaning. A semantic change requires
a new qualified ID or schema version.

## SYN-VOC-004 — Validation boundary

Neutral validates bundle structure and each contextual vocabulary payload. It
does not interpret application behavior. The generic probe enumerates the
qualified type identity and typed fields only.

An external reader receives the exact captured vocabulary contract from its
host. It never performs hidden I/O to obtain a schema.

## SYN-VOC-005 — Diagnostics

Distinct diagnostics cover unknown `use` name, missing/mismatched captured lock,
invalid bundle schema, unknown required feature, unknown qualified type, invalid
payload field, missing required field, duplicate field, and field type mismatch.
