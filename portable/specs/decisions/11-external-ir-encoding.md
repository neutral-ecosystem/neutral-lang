<!-- SPDX-License-Identifier: Apache-2.0 -->

# Section 9: external Neutral IR encoding

Status: accepted for Neutral v0 on 2026-09-03

This decision defines the only v0 external representation of a successful
Neutral artifact. It is called **Neutral IR Framed CBOR 0.1** (`NIR-CBOR/0.1`)
and has media type `application/vnd.neutral.ir+cbor;version=0.1`.

The encoding is an interchange and storage boundary. It does not change the
logical IR, fingerprint transcripts, captured vocabulary encoding, or public
reader contract.

## Contract coverage

| Frozen contract | Encoding ownership |
| --- | --- |
| `NL-IR-001`–`NL-IR-003` | Separate logical-payload and envelope sections plus complete companion sections |
| `NL-IR-004`–`NL-IR-005` | Distinct digest/fingerprint fields, graph-local labels, and structural alpha-equivalence |
| `NL-IR-006` | Encode validated memory only; decode external bytes as hostile input |
| `NL-PRO-001` | Exact source digest, byte length, and half-open byte spans in the source-map section |
| `NL-PRO-002`–`NL-PRO-003` | Closed value/field/reuse/reference provenance tied to logical IDs and source maps |
| `NL-PRO-004`–`NL-PRO-005` | Separate derivation partitions and companions excluded from logical equality |
| `NL-API-001`–`NL-API-005` | Complete validation before immutable reader construction; no rewrite or acquisition surface |
| `NL-DIA-004`–`NL-DIA-006` | Checked framing, hard structural limits, closed capabilities/schema, and safe bounded errors |

Successful compiler artifacts carry no diagnostic list: under `NL-API-002`, a
diagnostic failure has no authoritative IR to encode. The derivation section
still records the diagnostic policy and successful zero-diagnostic resource
fact. Failure diagnostics remain an out-of-band API result, not a sixth artifact
section.

## SYN-ENC-001 — Candidate decision

| Candidate | Exact numbers | Duplicate/unknown fields | Bounded hostile decoding | Tooling and bindings | Decision |
| --- | --- | --- | --- | --- | --- |
| JSON | Requires tagged decimal strings | Duplicate retention needs a nonstandard parser path; closed maps are possible | Text and nesting can be bounded | Excellent | Rejected because ordinary JSON libraries commonly collapse duplicates and expose numbers through floating point |
| Protocol Buffers | Requires a custom decimal message | Unknown-field preservation and merged duplicate scalar behavior conflict with fail-closed v0 rules | Length framing is useful but still needs an outer limit and semantic pass | Excellent | Rejected because generated schema evolution semantics are not Neutral's closed-schema semantics |
| MessagePack | Requires a custom decimal shape | Duplicate map keys and extension behavior vary by implementation | Definite lengths help, but profiles and schema tooling are inconsistent | Good | Rejected because the required closed profile would have weaker cross-language agreement |
| FlatBuffers | Requires a custom decimal table | Generated readers can verify structure, but unknown-field evolution is schema-driven | Strong verifier model | Good | Rejected because recursive logical values and graph validation still need a separate semantic model, with substantial generated-code coupling |
| Restricted CBOR in a fixed Neutral frame | Decimal components remain text plus signed integer | The Neutral profile rejects duplicate and unknown keys before construction | Fixed framing, definite lengths, checked integer widths, and hard limits permit pre-allocation checks | Broad, with RFC 8949 implementations in major languages | **Selected** |

The selected profile uses only a small RFC 8949 subset. A conforming decoder
may use a general CBOR library only if it can retain duplicate map keys, reject
disallowed forms, and enforce the limits before proportional allocation.

## SYN-ENC-002 — Fixed outer frame

All multibyte frame integers are unsigned big-endian. The fixed header is 48
bytes:

| Offset | Width | Field | Required v0 value |
| ---: | ---: | --- | --- |
| 0 | 8 | magic | byte sequence `4e 45 55 49 52 0d 0a 1a` (`NEUIR`, CR, LF, SUB) |
| 8 | 2 | framing revision | `1` |
| 10 | 2 | header length | `48` |
| 12 | 4 | flags | `0` |
| 16 | 8 | total artifact length | exact byte length of the complete frame |
| 24 | 8 | directory offset | `48` |
| 32 | 4 | section count | `5` |
| 36 | 2 | directory-entry length | `24` |
| 38 | 2 | reserved | `0` |
| 40 | 8 | required-capability mask | content-derived bit mask defined below |

The header is immediately followed by five 24-byte directory entries. Each
entry is `(kind:u16, schema_revision:u16, flags:u32, offset:u64, length:u64)`.
Every v0 entry has schema revision `1` and flags `0`.

| Kind | Name | Presence | Meaning |
| ---: | --- | --- | --- |
| 1 | envelope | required, first | Encoding/version/capability, producer, and integrity facts |
| 2 | logical payload | required | Complete `LogicalDocument` |
| 3 | source map | required | Complete original-byte `SourceMap` |
| 4 | provenance | required | Value, field, reuse, and reference provenance arrays |
| 5 | derivation | required, last | Complete `DerivationManifest`, including resource facts |

Entries are unique and ordered by kind. The first section begins immediately
after the directory; sections are contiguous, ordered by kind, nonempty, do not
overlap the header or directory, and end exactly at `total artifact length`.
Gaps, padding, trailing bytes, arithmetic overflow, zero-length sections, and
offsets not representable by the host are invalid.

## SYN-ENC-003 — Capabilities

The header mask declares every optional logical behavior actually present. No
bit may be set speculatively, and every behavior present must set its bit.

| Bit | Stable name | Required when |
| ---: | --- | --- |
| 0 | `exact-decimal` | any numeric type or value is present |
| 1 | `nullable` | any nullable type or null value is present |
| 2 | `nominal-record` | any user record type or value is present |
| 3 | `ordered-list` | any list type or value is present |
| 4 | `identity-reference` | any `Ref<T>` type, reference value, or reference provenance exists |
| 5 | `captured-vocabulary` | a captured vocabulary contract or vocabulary-owned type/value exists |
| 6 | `ordinary-reuse-provenance` | ordinary reuse provenance exists |
| 7 | `default-provenance` | user-record or vocabulary-default provenance exists |

Bits 8 through 63 are reserved and unsupported by v0. The envelope repeats the
set bits as stable names in ascending-bit order. Header and envelope declarations
must agree exactly. An unknown bit, unknown name, missing required bit, unused
declared bit, duplicate name, or order mismatch returns `unsupported-capability`.
Capabilities do not enable acquisition, code loading, syntax, or semantics.

## SYN-ENC-004 — Restricted CBOR profile

Each section contains exactly one CBOR item. The profile permits:

- unsigned and negative integers whose decoded values fit their declared field;
- definite-length byte strings and valid UTF-8 text strings;
- definite-length arrays and maps; and
- the simple values `false`, `true`, and `null`.

Tags, floating-point values, `undefined`, unassigned simple values, indefinite
lengths, break markers, invalid UTF-8, and trailing section data are invalid.
Maps have text keys only. Every map below is closed: missing required keys,
duplicate keys, and unknown keys are errors detected before map construction.
Integer and length arguments need not use shortest-width CBOR forms; map key
order has no meaning. Therefore valid byte encodings are deliberately
**noncanonical**.

Schema key spelling is exact ASCII. Text values are compared as decoded Unicode
scalar sequences without normalization. Arrays retain order only where the
logical or companion contract defines order. Canonically ordered logical
collections must already be in their frozen order; the decoder validates rather
than repairs them.

## SYN-ENC-005 — Common schema notation

The following notation defines the closed section schemas. `text` is valid UTF-8,
`u64` and `u32` are bounded nonnegative integers, `i64` is a bounded signed
integer, `digest` is a 32-byte CBOR byte string, and `fingerprint` is a 32-byte
CBOR byte string. Digests and fingerprints retain their existing SHA-256 and NHT
meanings; encoding them as bytes creates no new identity.

```text
module = {
  "language_behavior_version": text,
  "name": text
}
symbol = {
  "module": module,
  "name": text
}
nominal = {
  "module": module,
  "name": text
}
vocabulary_type = {
  "vocabulary": text,
  "name": text
}
span = { "start": u64, "end": u64 }
```

All language, schema, and API version text must equal the exact frozen version,
not merely a compatible range.

### Resolved types and exact values

Every sum value is a closed map with a mandatory `kind` discriminator and only
the keys shown for that kind:

```text
type = { "kind": "num" | "string" | "bool" }
     | { "kind": "record", "identity": nominal }
     | { "kind": "vocabulary-record", "identity": vocabulary_type }
     | { "kind": "list", "inner": type }
     | { "kind": "ref", "inner": type }
     | { "kind": "nullable", "inner": type }

value = {
          "kind": "num",
          "negative": bool,
          "coefficient": text,
          "scale": i64
        }
      | { "kind": "string", "value": text }
      | { "kind": "bool", "value": bool }
      | { "kind": "null" }
      | { "kind": "record", "identity": nominal, "fields": [field_value...] }
      | {
          "kind": "vocabulary-record",
          "identity": vocabulary_type,
          "fields": [field_value...]
        }
      | { "kind": "list", "items": [value...] }
      | {
          "kind": "ref",
          "target_element_id": u64,
          "target_symbol": symbol,
          "target_type": type
        }

field_value = { "name": text, "value": value }
```

An exact number never passes through host floating point. Its coefficient is
`"0"` or a nonempty ASCII digit string beginning with `1` through `9`; it has no
sign, separator, decimal point, or exponent. Zero requires `negative=false` and
`scale=0`. A nonzero coefficient must not end in `0`. The signed scale fits
`i64`, and coefficient length and absolute scale fit the derivation acceptance
limits and the reader's effective limits.

### Logical payload section

```text
logical_payload = {
  "logical_ir_schema_version": text,
  "module": module,
  "record_types": [record_type...],
  "vocabulary": null | vocabulary_contract,
  "declarations": [declaration...]
}

record_type = {
  "element_id": u64,
  "symbol": symbol,
  "fingerprint": fingerprint,
  "identity": nominal,
  "fields": [field_schema...]
}
field_schema = { "name": text, "type": type, "default": null | value }
declaration = {
  "element_id": u64,
  "symbol": symbol,
  "fingerprint": fingerprint,
  "name": text,
  "type": type,
  "value": value
}

vocabulary_contract = {
  "identity": vocabulary_identity,
  "types": [vocabulary_type_contract...]
}
vocabulary_identity = {
  "identity": text,
  "version": text,
  "schema_version": text,
  "encoding_version": text,
  "content_digest": digest,
  "required_features": [text...]
}
vocabulary_type_contract = {
  "identity": vocabulary_type,
  "fields": [vocabulary_field...]
}
vocabulary_field = { "name": text, "type": type, "default": null | value }
```

This projection contains every logical module/vocabulary identity, record and
binding node, resolved type, final value, fingerprint, and identity-reference
edge. Element IDs are document-local labels. Unique IDs, symbol consistency,
closed/canonical fields, exact types, reference targets, graph invariants,
fingerprints, captured vocabulary contracts, and all existing invalid logical
IR states are revalidated after decoding.

### Source-map section

```text
source_map = {
  "source_map_version": text,
  "source_digest": digest,
  "source_byte_length": u64,
  "module_span": span,
  "entries": [source_map_entry...]
}
source_map_entry = {
  "element_id": u64,
  "declaration_span": span,
  "type_span": span,
  "name_span": span,
  "value_span": span
}
```

Spans remain half-open offsets into exact original source bytes. They are
ordered, bounded by `source_byte_length`, attached to the correct unique logical
node, and checked using the existing source-map invariants.

### Provenance section

```text
provenance = {
  "provenance_version": text,
  "values": [value_provenance...],
  "fields": [field_provenance...],
  "reuses": [reuse_provenance...],
  "references": [reference_provenance...]
}
value_provenance = {
  "element_id": u64,
  "origin": origin,
  "normalization": normalization
}
field_provenance = {
  "element_id": u64,
  "field_path": [text...],
  "origin": origin
}
reuse_provenance = {
  "element_id": u64,
  "value_path": [text...],
  "source_element_id": u64
}
reference_provenance = {
  "element_id": u64,
  "value_path": [text...],
  "target_element_id": u64
}
```

`origin` is exactly one of `explicit-source`, `ordinary-reuse`,
`identity-reference`, `explicit-record-field`, `user-record-default`, or
`vocabulary-default`. `normalization` is exactly one of `exact-number`,
`string-escape`, `boolean-identity`, `null-identity`, `record-context`,
`list-context`, `immutable-reuse`, or `identity-reference`. Paths are arrays of
the existing canonical field-name or decimal list-index components. All owners,
sources, targets, paths, origins, and normalizations must describe the decoded
logical graph exactly; orphan, duplicate, contradictory, or impossible records
are invalid.

### Derivation section

```text
derivation = {
  "derivation_schema_version": text,
  "language_behavior_version": text,
  "logical_ir_schema_version": text,
  "source_map_version": text,
  "provenance_version": text,
  "meaning": { "source_digest": digest },
  "acceptance": {
    "source_bytes": u64,
    "diagnostics": u32,
    "string_bytes": u64,
    "numeric_digits": u64,
    "numeric_scale": u64,
    "declarations": u64,
    "record_fields": u64,
    "nesting_depth": u64,
    "list_items": u64,
    "traversal_nodes": u64
  },
  "diagnostic_policy": { "safe_bounded_output": bool },
  "resource_facts": {
    "source_bytes": u64,
    "declarations": u64,
    "diagnostics": u64,
    "decoded_string_bytes": u64
  },
  "vocabulary": null | vocabulary_identity
}
```

Version, source digest, vocabulary identity, acceptance facts, and observed
resource facts must agree with the other sections. Successful artifacts require
`safe_bounded_output=true` and zero retained diagnostics.

### Envelope section

```text
envelope = {
  "encoding": "NIR-CBOR/0.1",
  "framing_revision": 1,
  "required_capabilities": [text...],
  "versions": {
    "language_behavior": text,
    "logical_ir": text,
    "source_map": text,
    "provenance": text,
    "derivation": text,
    "vocabulary_schema": text
  },
  "producer": { "name": text, "version": text, "build": null | text },
  "integrity": [integrity_record...]
}
integrity_record = { "section": u64, "sha256": digest }
```

Integrity contains exactly one record for each section kind 2 through 5 in
ascending order and hashes the exact section bytes. It is corruption evidence,
not authenticity. The envelope is not self-hashed; an artifact-byte digest may
be computed by a container or caller. Producer/build, encoding, integrity, and
exact byte identity are excluded from logical payload equality.

## SYN-ENC-006 — Limits and validation order

The immutable hard ceilings are centralized in
[`config/ir-encoding.toml`](../../../config/ir-encoding.toml):

- 64 MiB complete artifact and per-section encoded size;
- exactly five sections;
- 128 nested CBOR containers or logical type/value layers;
- 1,000,000 CBOR array/map entries and logical traversal nodes;
- 16 MiB for any one text or byte string; and
- 1,000,000 exact-number coefficient digits.

A host may configure lower limits. The effective limit is the minimum of the
hard ceiling, the host policy, and every applicable captured derivation limit.
No declared length controls reserve/allocation until it fits the remaining
bounded section and the effective item/string limit.

Validation is fail-closed in this order:

1. available byte count, magic, fixed header fields, total length, capability
   mask, and checked directory-size arithmetic;
2. unique ordered directory entries, exact section kinds/revisions/flags,
   contiguous ranges, and per-section size ceilings;
3. envelope restricted-CBOR shape, exact encoding/versions/capabilities, and
   bounded producer/integrity records;
4. exact SHA-256 verification of sections 2 through 5;
5. restricted-CBOR lexical/container validation for every remaining section;
6. closed schema, scalar widths, enum spellings, collection order, and local
   structural limits;
7. logical IDs, types, values, references, fingerprints, and vocabulary;
8. source-map, provenance, derivation, resource, and cross-section consistency;
9. construction of the immutable validated reader document.

Cancellation may stop any phase and exposes no partial typed document.

## SYN-ENC-007 — Complete malformed and unsupported result classes

Every rejected artifact maps to exactly one bounded reader result class. Stable
diagnostic codes are assigned with the Stage 7 decoder implementation; the
classifications are frozen here.

| Class | Includes |
| --- | --- |
| `encoded-size-limit` | artifact, section, string, byte string, item, depth, coefficient, or traversal ceiling exceeded |
| `malformed-frame` | short header/directory, bad magic, wrong fixed size, reserved/flag bits, length mismatch, overflow, gap, overlap, reordering, duplicate/missing/extra/empty section, or trailing bytes |
| `unsupported-version` | unknown framing, section schema, encoding, language, logical, source-map, provenance, derivation, or vocabulary schema version |
| `unsupported-capability` | unknown/missing/unused capability or mask/name disagreement |
| `integrity-mismatch` | missing/duplicate/reordered digest record or unequal section digest |
| `malformed-cbor` | truncation, invalid UTF-8, forbidden type/tag/float/simple/indefinite form, invalid additional information, or trailing section item |
| `invalid-encoded-schema` | duplicate/unknown/missing/wrong-type key, integer overflow, invalid enum/text/digest/fingerprint/number spelling, or noncanonical required collection order |
| `invalid-logical-ir` | any frozen invalid module, vocabulary, ID, type, value, default, reference, graph, or fingerprint state |
| `invalid-source-map` | invalid source identity, span, ordering, ownership, or coverage state |
| `invalid-provenance` | invalid owner/path/origin/normalization/reuse/reference or logical mismatch |
| `invalid-derivation` | invalid policy/resource/limit fact or cross-section identity/version/vocabulary mismatch |
| `cancelled` | caller cancellation before validation completes |
| `internal-defect` | implementation invariant failure not attributable to hostile bytes |

Errors expose only a class, stable code, bounded safe parameters, and an encoded
byte offset when available. They never echo arbitrary hostile text, attempt a
registry/path/network lookup, load code, recover a partial artifact, accept an
unknown field, or reinterpret an unsupported version.

## SYN-ENC-008 — Equality, security, and change control

Two valid artifacts may encode identical logical meaning with different map
orders, legal CBOR integer widths, envelope producer/build facts, integrity
bytes, companion data, or graph-local `ElementId` spellings. Their bytes are
not canonical and byte equality is not logical equality. Logical equality
remains whole-graph structural alpha-equivalence under the existing v0 contract.

The framing and allocation review approves this design because fixed-width
checked framing precedes CBOR decoding, every length is bounded before
proportional allocation, the schema is closed, exact numbers avoid floating
point, integrity is verified before semantic construction, and no unsupported
feature can be ignored. Encoder convenience must not weaken any decoder check.

Any future framing, section, schema, capability, or semantic change requires a
new version identifier or capability assignment and an explicit compatibility
decision. Reserved values have no implied meaning.
