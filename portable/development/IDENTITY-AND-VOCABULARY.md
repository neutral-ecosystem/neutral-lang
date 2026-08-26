# Neutral v0 identity, digest, fingerprint, and vocabulary encoding contract

Status: proposed normative contract; must be accepted by the contract-freeze gate

This document fixes the implementation-independent identity and captured
vocabulary byte contracts needed before production code. It does not define the
external Neutral IR encoding selected later.

## Principles

- Exact captured bytes, logical identity, semantic revision, derivation, and
  serialized artifact identity are different facts.
- A digest is an integrity identifier, not proof of producer authority or
  semantic validity.
- Every semantic fingerprint uses a versioned, domain-separated transcript.
- Graph-local `ElementId` spelling never enters durable identity.
- No host serializer, locale, map order, pointer address, or Rust type layout may
  enter a digest preimage.
- Hash agility is explicit. A new algorithm/profile creates a new identifier; it
  never changes the meaning of an existing identifier in place.

## v0 hash algorithms and textual form

v0 uses SHA-256 as specified by
[NIST FIPS 180-4](https://csrc.nist.gov/pubs/fips/180-4/upd1/final). NIST has
announced that FIPS 180-4 will be revised, so the standards register must be
checked before release; an algorithm change requires a new profile identifier.

```text
HashAlgorithmId = "sha256"
DigestText       = "sha256:" + 64 lowercase hexadecimal digits
```

- [ ] Decoders reject uppercase, truncated, overlong, unprefixed, unknown, and
      nonhex digest text.
- [ ] Internal storage keeps algorithm and 32 digest bytes as separate typed
      fields rather than repeatedly parsing text.
- [ ] Constant-time comparison is required where a digest gates integrity or
      trust-sensitive processing; digest comparison alone still grants no
      authority.
- [ ] Test vectors include empty, one-byte, UTF-8, large, and adversarial inputs.

## Exact byte digests

`ByteDigestV1(bytes)` is SHA-256 over exactly the supplied byte sequence, with no
normalization, prefix, suffix, or serializer pass.

Typed uses are:

- `SourceContentDigest`: exact captured source bytes, including a BOM and original
  CR/LF bytes when present;
- `VocabularyContentDigest`: exact captured vocabulary bundle bytes;
- `EncodedArtifactDigest`: exact external IR artifact bytes; and
- `FixtureManifestDigest`: exact accepted fixture-manifest bytes.

Typed wrappers prevent source, vocabulary, artifact, and manifest digests from
being accidentally interchanged, although equal byte sequences intentionally
have equal underlying SHA-256 results.

### Verification rules

1. Enforce the applicable byte limit.
2. Compute the digest while streaming or before proportional parsing work.
3. Compare the expected algorithm and digest.
4. On mismatch, return a bounded integrity failure and do not parse as trusted
   content.
5. Continue to treat matching bytes as untrusted input requiring full structural
   and semantic validation.

If two byte sequences are simultaneously available, have the same digest, and
differ byte-for-byte, report an internal integrity/collision defect and stop.
Never silently choose either sequence. Storage deduplication must verify length
and digest and may verify bytes according to the storage threat model.

## Neutral Hash Transcript v1

Semantic fingerprints use `NHT-v1`, a tiny unambiguous binary transcript. It is
a hashing format only—not a public interchange encoding or canonical Neutral IR
serialization.

### Primitive framing

All integers are unsigned big-endian unless a field explicitly defines a signed
representation.

```text
frame(tag, payload) =
    u16be(byte_length(tag_utf8))
    || tag_utf8
    || u64be(byte_length(payload))
    || payload

hash(domain, payload) =
    SHA-256(frame("neutral-nht-v1", frame(domain, payload)))
```

Rules:

- tags and domain strings are fixed ASCII;
- strings are their exact Unicode scalar sequence encoded as UTF-8, with no
  Unicode normalization;
- Boolean payload is one byte: `00` false, `01` true;
- null is an empty payload under a null-specific tag;
- byte sequences are length-prefixed and otherwise unchanged;
- a sequence payload begins with `u64be(item_count)` followed by each framed
  item;
- unordered keyed collections are sorted by their already-framed key bytes;
- ordered lists preserve item order;
- duplicate keys are invalid before transcript construction;
- unknown node/tag versions fail closed; and
- transcript construction uses checked lengths and structural limits.

### Exact numeric transcript

The normalized `num` transcript contains:

```text
sign        = 00 for zero/positive, 01 for negative
coefficient = ASCII decimal digits with no leading zero except "0"
scale_sign  = 00 for zero/positive, 01 for negative
scale       = ASCII decimal magnitude with no leading zero except "0"
```

Zero is always sign `00`, coefficient `"0"`, scale sign `00`, and scale `"0"`.
A zero scale may never use negative sign. Equivalent accepted source spellings
must produce the same tuple. Numeric limits apply before materializing
coefficient or scale buffers.

### Required independent test vectors

- [ ] Every primitive framing form has exact hexadecimal transcript and digest.
- [ ] Equivalent numeric spellings have equal transcripts.
- [ ] Distinct list order changes the transcript.
- [ ] Different map insertion order preserves the transcript.
- [ ] Unicode scalar sequences are preserved without locale or normalization.
- [ ] Unknown tags, duplicate keys, overflowed lengths, and over-limit nesting
      fail before a fingerprint is exposed.
- [ ] At least two independent implementations agree on all accepted vectors.

## Identity contracts

### Logical module identity

`LogicalModuleIdentity` is a structured `(language_behavior_version,
module_name)` value. It is not source content identity and is not derived from a
path. Its storage hash, if needed, uses domain
`neutral/logical-module-identity/v1` and the NHT transcript of both fields.

### Module-symbol identity

`ModuleSymbolIdentity` is structured `(logical_module_identity,
declaration_name)`. It represents declaration continuity. Formatting,
declaration order, unrelated source edits, and `ElementId` allocation do not
change it. Renaming or moving the declaration does.

### Declaration fingerprint

```text
DeclarationFingerprintV1 = hash(
    "neutral/declaration-fingerprint/v1",
    NHT(declaration_kind, resolved_type, logical_definition)
)
```

Rules:

- declaration source name and module are excluded because continuity is carried
  by `ModuleSymbolIdentity`;
- source spans, comments, provenance, derivation, diagnostics, producer build,
  map order, and `ElementId` spelling are excluded;
- exact scalar values use normalized logical forms;
- record declaration and contextual record fields are keyed by field name and
  sorted by framed field-name bytes; v0 record field order is nonsemantic;
- list values preserve logical order;
- ordinary reuse contributes the final logical value, not the reuse path;
- a default contributes the final logical value, not its provenance category;
- identity references encode expected type plus target
  `ModuleSymbolIdentity`, never graph-local `ElementId`; and
- vocabulary-owned types encode exact logical vocabulary identity, vocabulary
  version, schema version, and qualified type identity.

Any change to inclusion/order rules creates `declaration-fingerprint/v2`.

### Graph-local ElementId

`ElementId` is an opaque validated-document label. It may be an integer or other
compact implementation type, but:

- it is unique only inside one validated document;
- it is not hashed into durable identities;
- allocation order is nonsemantic;
- it is never persisted as a cross-document reference; and
- logical comparison maps it through graph alpha-equivalence.

### Derivation identity

Derivation is partitioned into meaning, acceptance/resource, and
diagnostic/output-policy records. Each partition has its own NHT transcript and
fingerprint. The overall derivation identity hashes:

```text
derivation_schema_version
meaning_partition_fingerprint
acceptance_partition_fingerprint
diagnostic_partition_fingerprint
```

The meaning partition includes exact source/vocabulary content digests,
language behavior version, semantic options, and required structural-feature
versions. Operational clock/deadline values, host paths, credentials, and
rendered message text are excluded. Acceptance limits and diagnostic disclosure
policy enter only their named partitions.

### Logical IR equality

Logical IR equality remains structural graph alpha-equivalence. A whole-document
hash may accelerate comparison only after validation and only when followed by
structural equality before a security-sensitive decision. v0 does not define a
canonical external IR byte sequence.

## Captured vocabulary bundle encoding v0

The v0 bundle is strict UTF-8 JSON conforming to
[RFC 8259](https://www.rfc-editor.org/info/rfc8259/), narrowed by this contract.
The encoding identifier is:

```text
application/vnd.neutral.vocabulary+json;version=0.1
```

This encoding is independent from the logical vocabulary schema version and
vocabulary release version.

### JSON restrictions

- The byte stream is UTF-8 without BOM.
- The top level is exactly one object.
- Duplicate object member names are errors at every depth.
- Unknown members are errors because the schema is closed.
- JSON object order and insignificant whitespace are nonsemantic.
- Arrays preserve order only where the logical schema declares order; set-like
  arrays are duplicate-free and normalized by the validator.
- Unpaired surrogates and non-Unicode string content are errors.
- Raw JSON number tokens are forbidden. Neutral exact numbers are tagged decimal
  strings so no JSON/host floating-point conversion can occur.
- NaN, infinity, comments, trailing commas, and implementation extensions are
  errors.
- Parsing enforces byte, depth, string, member, array, type, field, default, and
  feature limits before proportional allocation.

The exact-byte `VocabularyContentDigest` is computed before parsing. Formatting
changes therefore change content identity even when the validated logical
vocabulary contract remains equal. JSON canonicalization is neither required
nor used for v0 identity.

### Closed envelope

The top-level object contains exactly:

```json
{
  "format": "neutral-vocabulary-bundle",
  "encoding_version": "0.1",
  "schema_version": "0.1",
  "identity": "Fixture",
  "version": "0.1.0",
  "required_features": [],
  "types": []
}
```

Contract rules:

- `format` and `encoding_version` must match exactly;
- `schema_version` selects one closed logical schema;
- `identity` is an uppercase Neutral vocabulary identity, not a path or URL;
- `version` is one exact accepted vocabulary release identifier, never a range
  or `latest`;
- `required_features` is a duplicate-free list of immutable qualified feature
  IDs and has set semantics; and
- `types` is a duplicate-free list of nominal type definitions with order
  normalized by type name.

### Type and field objects

A type object contains exactly:

```json
{
  "name": "Metadata",
  "fields": []
}
```

Field-array order is nonsemantic and normalized by field name. Duplicate type
or field names fail before normalization.

A field object contains exactly:

```json
{
  "name": "label",
  "type": { "kind": "string" },
  "default": null
}
```

`default: null` in the field object means no default. It is schema metadata, not
the Neutral `null` value. A Neutral default value is always a tagged value
object.

### Type encoding

Allowed type objects are closed tagged unions:

```json
{ "kind": "num" }
{ "kind": "string" }
{ "kind": "bool" }
{ "kind": "nullable", "inner": { "kind": "string" } }
{ "kind": "list", "element": { "kind": "string" } }
{ "kind": "ref", "target": "Metadata" }
{ "kind": "record", "target": "Metadata" }
```

Only schema-approved keys are valid for each `kind`. User generics, executable
validators, imports, paths, callbacks, and arbitrary expressions are invalid.

### Closed default encoding

Defaults are closed tagged values:

```json
{ "kind": "num", "value": "10.5" }
{ "kind": "string", "value": "sample" }
{ "kind": "bool", "value": true }
{ "kind": "null" }
{ "kind": "list", "items": [] }
{ "kind": "record", "fields": [] }
```

Record default fields are objects with exactly `name` and `value`. Duplicate,
unknown, missing, incompatible, name-reuse, or `ref` defaults are invalid.
Numeric strings use Neutral numeric grammar and normalize through the same exact
numeric implementation as source.

### Validation sequence

1. Enforce captured byte limit and compute exact-byte digest.
2. Match captured lock identity, release version, encoding/schema version,
   digest, and allowed feature declaration.
3. Parse strict bounded JSON while detecting duplicates.
4. Validate the closed envelope and reject unknown members.
5. Validate required structural feature IDs.
6. Collect nominal type names and reject duplicates/protected collisions.
7. Validate field/type graphs, closed defaults, and recursion rules.
8. Produce an immutable validated logical vocabulary contract.
9. Validate source vocabulary-owned payloads against that contract.

No stage executes bundle content or performs external I/O.

### Reader contract

An external IR reader receives either:

- the exact validated logical vocabulary contract plus its captured identity,
  version, schema/encoding versions, and content digest; or
- exact captured bundle bytes and lock facts, which it validates using the same
  contract before validating IR.

Missing/mismatched contracts, unknown required features, and unavailable schema
versions fail closed. The reader never searches a registry, filesystem, cache,
or network.

## Required conformance vectors

- [ ] Exact source, vocabulary, artifact, and fixture-manifest byte digests.
- [ ] NHT framing and every identity/fingerprint domain.
- [ ] Formatting/order changes that preserve or change each identity as defined.
- [ ] Digest mismatch and synthetic collision-handling paths.
- [ ] Duplicate JSON keys at every nesting level.
- [ ] Unknown envelope/type/value fields and unknown tagged kinds.
- [ ] Raw JSON numbers, BOM, invalid UTF-8, surrogates, depth/count/size limits,
      and truncated input.
- [ ] Every allowed type/default form and every prohibited executable form.
- [ ] Logical equality for differently formatted bundles with different byte
      digests.
- [ ] External reader behavior with exact, missing, stale, and mismatched
      contracts.

The contract-freeze gate cannot pass until these vectors are independently
reviewed and executable expected results are committed.
