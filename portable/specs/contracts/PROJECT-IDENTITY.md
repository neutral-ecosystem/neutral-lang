<!-- SPDX-License-Identifier: Apache-2.0 -->

# Complete project identity and canonical transcript contract

Status: frozen contract and core implementation; reader integration and full
independent reproducibility review remain separate gates.

This refines [PROJECT](PROJECT.md), [CAPTURE-REQUEST](CAPTURE-REQUEST.md), and
[PROJECT-IR](PROJECT-IR.md). Package release numbers never select hash behavior.
The identity profile is exactly `neutral.project-identity/1`.

## Framing and bounds — V1-ID-003

`F(tag, payload)` is `u16be(ASCII tag byte length) || ASCII tag ||
u64be(payload byte length) || payload`. A transcript is exactly
`F("neutral-nht-v1", F(domain, F("identity-profile", UTF8(profile)) || body))`.
The digest is SHA-256 of those complete bytes exactly once, not of hex, JSON,
an inner payload, or an already hashed digest. SHA-256 here supplies content
identity, not authenticity, authority, or proof of faithful compilation.

Strings use exact UTF-8; no Unicode normalization, case folding, path or URL
interpretation occurs. Integers are eight-byte big-endian unsigned values;
number scales are eight-byte big-endian two's-complement signed values. Boolean
and number-sign payloads are one byte, `00` or `01`. Empty payloads and empty
named collection frames are explicit. Tuple order and all tags below are frozen.
Collection counts are determined by frames, never delimiter concatenation.

Callers provide independent nonzero byte and framed-node limits. Limits intersect
hard ceilings of 67,108,864 complete transcript bytes and 1,000,000 frames/items;
value/type recursion intersects the shared 64-layer project ceiling. Key bytes
inspected before sorting/order comparisons are cumulatively bounded by the byte
limit. Checked arithmetic, fallible reservations, in-place frame backpatching,
and explicit cancellation checkpoints prevent partial transcript publication.
Zero bounds, exhausted budgets/allocation failure, and excessive depth classify
as `Limit`; malformed/noncanonical structural input as `InvalidInput`;
cancellation as `Cancelled`. Limits and cancellation tokens are not identity
inputs. A structural transcript builder does not replace independent semantic
validation of untrusted IR or root selection.

## Captured closure — V1-ID-001

Domain: `neutral/project-captured/v1`. Body fields, in order:

1. `profile`: exact source language profile.
2. `sources`: `source` frames in increasing module-name order. Each contains
   `module`, `source-id`, `digest` (32 exact raw source SHA-256 bytes), and
   `byte-length`. Source IDs must be unique, nonempty, and control-free.
3. `vocabularies`: `vocabulary` frames in increasing canonical identity order.
   Each contains `identity`, `version`, `encoding-version`, `schema-version`,
   `digest` (32 exact raw vocabulary SHA-256 bytes), `byte-length`, and
   `features` containing sorted unique `feature` UTF-8 frames.

All supplied members, including disconnected units, are included. Exact source
bytes (via their digest/length), source IDs, module names, profile, every exact
vocabulary lock field, and vocabulary byte identity affect capture identity.
Capture sequence, correlation/project key, expected-digest assertion presence,
host mappings/paths, acceptance controls, cancellation, roots, and authoring
metadata supplied separately from semantic bundles do not.

## Complete logical form — V1-ID-002, V1-ID-003

Domain: `neutral/project-logical/v1`. Body fields, in order:

1. `schema`: exact complete project IR schema; `profile`: exact core profile.
2. `modules`: `module` frames containing `name` and `imports` (`target` frames).
3. `declarations`: `declaration` frames containing a stable symbol, `public`,
   `signature`, `value`, and `defaults`. Private declarations are included.
4. `vocabulary-records`: `record` frames containing `identity`, `version`,
   `name`, `public`, and `fields` (`field` frames with `name` then canonical type).
5. `vocabulary-catalogues`: `vocabulary` frames containing `identity`, `version`,
   and `public-types` (`name` frames).
6. `public-edges`: `edge` frames containing `from` symbol, `to` symbol, and
   `kind` (`type`, `reference-type`, `value`, or `reference`). Validated public
   dependency topology is logical meaning; source occurrence locations are not.

A stable symbol is `profile`, `module`, `declaration` UTF-8 frames, not an
allocation label. Modules sort by logical module identity; declarations by full
module-symbol identity; records by `(vocabulary identity, revision, name)`;
catalogues by `(identity, revision)`; public edges by `(from, kind, to)` with kind
in the order listed above. Imports, fields/defaults, type names, and feature
names use increasing exact UTF-8 byte order. Duplicates/unordered canonical IR
fail instead of being silently merged. Ordered lists never sort.

`signature` wraps `binding` plus its type, or `record` plus `field` frames
(`name`, type). Types use empty `num`, `string`, `bool`, `url`, `path` frames;
`List`, `Ref`, and `nullable` wrap their canonical inner type; `nominal` wraps a
stable symbol; `vocabulary` wraps `identity`, `version`, `name`.

`value` contains `absent` only for declarations without values; explicit null is
`null`, not absence. Values use `bool` plus its byte; `num` plus `negative`,
normalized ASCII `coefficient`, signed `scale`; `string`, `url`, or `path` plus
decoded exact text; `Ref` plus stable target symbol; `List` plus ordered canonical
values; `record` plus sorted `field` frames (`name`, value). Defaults use the
same field/value framing and include unused closed defaults.

Host mappings, aliases, capture order, graph-local labels, roots, source IDs,
source/vocabulary byte evidence, source maps, occurrence provenance, processing
limits/resource accounting, producer version, and separately supplied authoring
metadata are excluded. Complete locked semantic schemas/revisions, visibility,
types, exact values/defaults, nominal/reference targets, modules/imports, and
public dependency topology are included. The public export index/fingerprint
is independently validated derived data, not a substitute for private meaning.

## Derivation — V1-ID-004

Domain: `neutral/project-derivation/v1`. Body fields are `logical` (32 raw
logical identity bytes), `captured` (32 raw captured identity bytes), `producer`,
`producer-version`, `capture-limits`, and `project-limits`. Producer identity and
revision are explicit inputs, not ambient environment/package discovery.

Capture-limit fields in order: `total-source-bytes`, `source-bytes-per-unit`,
`source-units`, `source-id-bytes`, `module-id-bytes`, `vocabulary-units`,
`vocabulary-bytes-per-unit`, `total-vocabulary-bytes`, `imports-per-module`,
`import-edges`, `scc-units`, `declarations`, `diagnostics`, `output-bytes`.
Project-limit fields: `modules`, `declarations`, `import-edges`, `nodes`,
`text-bytes`, `artifact-bytes`. All limit values are positive unsigned integers.
Changed source evidence, producer revision, or acceptance context can change
derivation identity without changing logical meaning. Host correlation/location,
roots, wall clocks, and thread scheduling never enter this layer.

## Artifact — V1-ID-004

Domain: `neutral/project-artifact/v1`. Body fields are `derivation` (32 raw
derivation identity bytes), `kind`, `format`, `roots`, and `options`.
Kinds are exactly `project` and `view`. A complete project artifact has no roots.
A view includes sorted unique `root` frames containing stable symbols; selection
order is non-semantic. Empty view selection differs from complete publication.
The reader must validate root existence/publicness before treating the identity
as authoritative. Format is exact nonempty control-free ASCII, independent of
kind; options are sorted unique nonempty-name `option` frames (`name`, `value`).

All transport choices that affect an artifact must be explicit kind/format/
selection/options inputs. Changing kind, format, roots, or options changes only
the artifact layer. Artifact identity identifies this derivation/representation
contract, not arbitrary noncanonical wire bytes; use a separate byte checksum
for transport integrity. Hashes carry no permission or fetch/open capability.

## Literal vectors and immutability

[vectors.json](../fixtures/stage7/vectors.json) publishes exact captured source,
source digest, controls, complete minimal logical facts, producer revision,
formats, six literal complete transcript byte strings, and expected SHA-256
digests. [The manifest](../../conformance/manifest.toml) pins fixture/oracle bytes.
Tests compare bytes and digests, not output generated and blessed at test time.
These baseline vectors were assembled outside the Rust implementation using
explicit NHT framing and standard SHA-256. Full independent implementation,
adversarial-vector review, reader/probe integration, and actual cache equivalence
remain Stage 7.3/7.4 requirements. Frozen profile/domain changes require a new
identity schema/domain and new immutable vectors; old vectors are not rewritten
to make a changed implementation pass.
