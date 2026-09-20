<!-- SPDX-License-Identifier: Apache-2.0 -->

# Captured project request contract

Status: accepted Stage 2 contract

This contract freezes the host-to-compiler boundary before implementation. It
governs `CapturedProjectRequest`, immutable capture, request construction, and
capture failures. It does not activate module imports, graph construction, or
v1 semantic compilation; those remain owned by later stages.

## Request envelope

`CapturedProjectRequest` is a closed, versioned value with these fields:

| Field | Required | Meaning |
| --- | --- | --- |
| `request_version` | yes | Exact `neutral.capture/v1` API version. |
| `profile` | yes | One recognized exact core language profile. Stage 2 accepts only `1.0` requests for project capture. |
| `project_key` | no | Bounded opaque host correlation text; non-semantic and excluded from every identity. |
| `sources` | yes | Non-empty complete sequence of captured source-unit inputs. |
| `vocabularies` | yes | Possibly empty exact captured vocabulary bundle/lock sequence. |
| `controls` | yes | Complete deterministic limits and cooperative cancellation token. |

The closed public type has no extension map. Unknown request versions or fields
fail before source allocation or parsing. It contains no root selection,
filesystem path, URL, credential, environment fact, package state, cache key,
resolver, callback, clock, locale, compiler scheduling option, output policy,
or authoring metadata.

Each source-unit input contains exactly:

- a non-empty bounded logical source ID;
- one exact qualified logical module ID;
- exact immutable source bytes; and
- an optional expected `SourceContentDigest` for capture-time integrity.

A logical source ID is inert exact UTF-8 with no control characters. It is not
a path, URL, module name, lookup key, or authorization token. Source IDs and
module IDs are independently unique within one request. The source byte digest
is computed before header scanning and must equal an expected digest when one
is supplied.

Each vocabulary input contains exact immutable bundle bytes and one lock:
canonical identity, semantic release, bundle-encoding version, logical-schema
version, exact content digest, and sorted unique required feature IDs. The
captured lock sequence must be the exact cover required by the supplied source
set: missing, extra, unused, duplicate, conflicting-revision, or byte-mismatched
locks fail the whole capture.

## Processing controls and limits

The host supplies every limit as a non-zero unsigned value. There are no
ambient defaults inside project capture. The frozen limit fields are:

| Limit | Bounded work or retained output |
| --- | --- |
| `total_source_bytes` | Sum of all exact source bytes. |
| `source_bytes_per_unit` | Exact bytes in any one source unit. |
| `source_units` | Number of supplied source units/modules. |
| `source_id_bytes` | UTF-8 bytes in one logical source ID. |
| `module_id_bytes` | UTF-8 bytes in one logical module ID. |
| `vocabulary_units` | Number of captured vocabulary inputs. |
| `vocabulary_bytes_per_unit` | Bytes in one captured vocabulary bundle. |
| `total_vocabulary_bytes` | Sum of captured vocabulary bytes. |
| `imports_per_module` | Imports retained for one module once Stage 3 activates them. |
| `import_edges` | Total project import edges once Stage 3 activates them. |
| `scc_units` | Modules retained in one SCC once Stage 3 activates them. |
| `declarations` | Total project declarations. |
| `diagnostics` | Retained ordered diagnostics. |
| `output_bytes` | Complete authoritative encoded output. |

Every total uses checked arithmetic. Exact-boundary input is permitted; one
over fails before proportional allocation or traversal. Cancellation is
observed before capture, after source/lock integrity, after header validation,
and before publishing immutable output. Timeouts, wall clocks, thread counts,
retry counts, and allocation heuristics are not request controls.

## Header agreement and supplied closure

Capture scans only the bounded profile and module headers needed to establish
agreement. Each source header profile must equal the request profile and each
module header must exactly equal that source input's requested module ID. No
case folding, Unicode normalization, path derivation, aliasing, or best-effort
correction occurs.

Every supplied source is retained in the captured project, including units not
reachable from another supplied unit. There are no capture roots. Stage 2
therefore proves complete supplied-set retention and cannot prune disconnected
units. Import grammar, missing-import validation, deterministic graph building,
and SCC computation activate only in Stage 3.

## Identity rules

The logical module identity is the exact pair `(profile, qualified module ID)`.
It does not include logical source ID, host location, capture order, or source
bytes. The logical source identity is the exact request-scoped source ID plus
the exact source-content digest; it is not interchangeable with module
identity.

Immutable capture orders source units by `(module ID, source ID)` and
vocabularies by canonical identity before deriving captured-closure facts.
Input sequence, `project_key`, host mapping, host location, and allocation order
cannot affect capture meaning. Exact source bytes, source IDs, module IDs,
profile, and exact vocabulary locks do affect the captured closure. The hash
transcript for captured-closure identity remains reserved for Stage 7; Stage 2
must not invent a provisional public digest.

## Host mapping boundary

Hosts may map arbitrary local locations to source inputs before request
construction. Mapping keys and locations never enter `CapturedProjectRequest`.
Two host mappings that produce identical source IDs, module IDs, bytes, locks,
and controls must construct equivalent requests. A host adapter must reject two
different byte sequences or module IDs mapped to the same source ID before
calling capture. The stable adapter diagnostic is `NEU-HOST-001`.

## Fail-closed outcomes

Capture returns either one complete immutable project or one bounded failure;
partial source sets, partial locks, recovery projects, and authoritative IR are
never exposed on failure.

| Code | Stable failure |
| --- | --- |
| `NEU-CAP-001` | Unsupported or malformed request version/envelope. |
| `NEU-CAP-002` | Unknown, unavailable, or request/header-mismatched profile. |
| `NEU-CAP-003` | Empty supplied source set. |
| `NEU-CAP-004` | Duplicate logical source ID. |
| `NEU-CAP-005` | Duplicate logical module identity. |
| `NEU-CAP-006` | Requested module ID and source header disagree. |
| `NEU-CAP-007` | Missing, malformed, or multiply declared required header. |
| `NEU-CAP-008` | Missing required vocabulary lock/input. |
| `NEU-CAP-009` | Extra or unused vocabulary lock/input. |
| `NEU-CAP-010` | Duplicate or conflicting vocabulary identity/revision. |
| `NEU-CAP-011` | Source or vocabulary bytes disagree with the expected digest/lock. |
| `NEU-CAP-012` | A captured structural limit is zero or exceeded. |
| `NEU-CAP-013` | Cooperative cancellation observed. |

All capture diagnostics use the `NEU-CAP` family, deterministic source/module
ordering, safe parameters, and no host location or source-text disclosure.
