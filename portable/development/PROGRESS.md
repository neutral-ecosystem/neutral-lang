<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v0 development progress

Status: Stage 7 Step 1 complete.

## Current focus

- Stage: Stage 7, Step 1
- Status: External IR encoding decision accepted
- Last updated: 2026-09-03

## Next actions

- [ ] Continue with Stage 7, Step 2: encode validated documents.

## Blockers

None recorded.

## Completed log

- [*] 2026-09-03: Completed Stage 7, Step 1. Selected Neutral IR Framed CBOR
  0.1 after comparing JSON, Protocol Buffers, MessagePack, FlatBuffers, and a
  restricted CBOR profile. Froze the fixed header and section directory,
  capability assignments, five complete artifact sections, exact-number
  representation, closed CBOR schemas, hard allocation ceilings, validation
  order, malformed/unsupported result classes, and the rule that external bytes
  are noncanonical while logical equality remains structural alpha-equivalence.
  The security/allocation review approved bounds-before-allocation and
  fail-closed version, capability, integrity, and schema handling.

- [*] 2026-09-03: Completed Stage 6, Slice 6.2 and Stage 6 validation. Activated
  optional `use` and `Vocabulary::Type` grammar; exact host-captured bundle and
  lock inputs; bundle-before-payload validation; qualified contextual values;
  distinct captured-default provenance; exact vocabulary identity, version,
  schema, encoding, digest, feature, logical schema, and derivation records;
  fail-closed reader validation; and generic probe enumeration. Frozen cases
  now cover missing and mismatched captures, namespace collisions, unknown
  features/types, executable shapes, and missing, duplicate, unknown, and
  incompatible payload fields without registry, path, or network acquisition.

- [*] 2026-09-02: Completed Stage 6, Slice 6.1. Added typed exact vocabulary
  digests and strict lowercase digest parsing; exact lock verification before
  parsing; a dependency-free bounded JSON decoder that retains object members
  until duplicate detection; closed envelope, feature, name, type, field,
  default, and recursion validation; immutable separate captured/logical bundle
  projections; reference-only recursive type graphs; and grouped accepted and
  hostile fixtures with frozen oracles. Raw numbers, BOM/UTF-8/surrogate errors,
  unknown or executable shapes, bad defaults/targets, embedded cycles, digest
  mismatch, and allocation limits fail closed without code loading or I/O.

- [*] 2026-09-01: Completed Stage 5, Slice 5.3 and Stage 5 validation. Replaced
  literal graph-ID equality with a one-to-one whole-document mapping across
  record and binding nodes; recursively compares identity edges by mapped IDs;
  separated logical-payload comparison from exact companion/envelope equality;
  made the hostile reader independently recompute logical fingerprints;
  documented the cross-document `ElementId` prohibition; and added reflexive,
  symmetric, transitive, generated-renaming, changed payload/edge/fingerprint,
  duplicate-ID, dangling-ID, and hostile-reader property vectors. All Stage 2–5
  compiler, reader, probe, boundary, conformance, property, and security suites
  remain green.

- [*] 2026-08-31: Completed Stage 5, Slice 5.2. Added invariant `Ref<T>` and
  `ref(name)` grammar, forward exact-type target resolution, a distinct
  reference failure class and stable diagnostics, reference-only nominal
  recursion, identity cycles outside value dependencies, graph-local IR edges
  with durable-symbol fingerprints, reference provenance, hostile-reader edge
  validation, ID-based probe traversal, and frozen grouped fixtures/oracles.
  Ordinary reuse and typed identity references remain distinct end to end.

- [*] 2026-08-31: Completed Stage 5, Slice 5.1. Activated unqualified immutable
  value reuse after full declaration collection; added deterministic dependency
  ordering, forward/transitive/nested reuse, exact outer-nullable widening,
  invariant container checks, bounded chains, stable unknown/wrong-kind/cycle
  diagnostics with primary and related locations, final-value fingerprints,
  explicit reuse provenance, hostile-reader validation, probe exposure, and
  frozen grouped Stage 5 fixtures/oracles. Identity references remain inactive
  for Slice 5.2.

- [*] 2026-08-30: Completed Stage 4, Slice 4.3 and Stage 4 validation. Added
  invariant `List<T>` grammar and IR, ordered/empty/nested/nullable lists,
  closed record-list defaults, list fingerprints and reader/probe traversal,
  item/depth/traversal limits, grouped fixtures/oracles, and removal of
  transitional negative cases whose list syntax is now valid v0.

- [*] 2026-08-29: Relocated the obsolete `portable/conformance` README to the
  root conformance asset directory, clarified manifest/oracle/report ownership,
  and replaced stale Stage 1 CLI/probe shell wording with the scheduled Stage 8
  activation boundary.

- [*] 2026-08-29: Completed Stage 4, Slice 4.2. Added closed scalar, null, and
  recursively contextual record defaults; omission materialization; schema and
  definition fingerprint defaults; explicit versus user-default field
  provenance; reader/probe validation and exposure; retained-string resource
  accounting; stable non-constant/type/syntax rejection; and frozen grouped
  fixtures/oracles.

- [*] 2026-08-29: Completed Stage 4, Slice 4.1. Added bounded record grammar,
  two-pass root collection, nominal schema resolution, contextual and nested
  record lowering, duplicate/missing/unknown/wrong-kind/recursion diagnostics,
  canonical declaration and field order, public reader/probe traversal, and
  frozen Stage 4 fixtures/oracles.

- [*] 2026-08-27: Completed Stage 3, Slice 3.4 and Stage 3 validation. Added
  recursive outer-nullable resolved types, explicit typed null, exact scalar to
  nullable widening, null fingerprints/provenance, reader type/value checks,
  probe traversal, frozen nullability fixtures/oracles, and future grammar
  exclusion evidence while keeping Stage 2 green.

- [*] 2026-08-27: Completed Stage 3, Slice 3.3. Added frozen signed decimal
  grammar, separator/fraction/exponent normalization, canonical zero, exact
  NHT-backed fingerprints, explicit numeric digit/scale limits, stable
  numeric-limit diagnostics, and grouped positive/negative numeric fixtures
  with conformance evidence.

- [*] 2026-08-27: Reorganized v0 source fixtures by outcome and primary
  feature (`syntax`, `identifiers`, `strings`, `booleans`, `values`, and
  `vocabulary`); updated manifests, oracles, compiler includes, and fixture
  documentation to use the grouped paths.

- [*] 2026-08-27: Replaced duplicated compiler diagnostic literals with the
  owning `neutral-compiler::diagnostics` and `neutral-probe::diagnostics`
  namespaces; cross-package version assertions now use exported compiler and
  IR contract constants instead of raw version strings.

- [*] 2026-08-27: Completed Stage 3, Slice 3.2. Added bounded string escape and
  Unicode-scalar decoding, exact Boolean tokens, scalar type checking, typed IR
  and fingerprints, safe reader/probe rendering, decoded-string resource
  accounting, stable invalid-string/type/limit diagnostics, and frozen
  positive/negative conformance evidence.

- [*] 2026-08-27: Centralized frozen language spellings and the protected-core
  namespace in compiler-private `language.rs`; lexer, parser, and semantics now
  share the same named constants rather than repeating source-language strings.

- [*] 2026-08-27: Completed Stage 3, Slice 3.1. Froze nine identifier,
  protected-name, comment, punctuation, newline, and string-boundary cases;
  implemented exact private trivia retention, full ASCII name classification,
  stable boundary diagnostics, and raw newline behavior; and verified logical
  comment invariance, source facts, reader/probe output, deterministic hostile
  comment rejection, limits, and future-grammar exclusion.

- [*] 2026-08-26: Completed Stage 2, Step 4 and Stage 2 validation. The minimal
  fixture now validates names/types/exact values, receives frozen NHT-backed
  identities, lowers to immutable logical IR/source-map/provenance/derivation
  artifacts, traverses through reader/probe contracts, and maps a consumer
  diagnostic back to source. Active unit, smoke, integration, system,
  conformance, property, security, and fuzz-smoke suites pass under
  `cargo xtask ci pr`.

- [*] 2026-08-26: Completed Stage 2, Step 3. Added the private minimal raw
  lexer, physical-newline retention, semantic line-end normalization, and exact
  parser for `neu "0.1"`, one module header, and one `num` binding. Frozen
  negative diagnostics and spans, newline equivalence, BOM/UTF-8/NUL safety,
  and private parser boundaries are covered by tests.

- [*] 2026-08-26: Configured continuous integration for every push to `main`
  and release qualification for every pushed tag; both workflows retain manual
  dispatch.
- [*] 2026-08-26: Upgraded workflow repository checkout steps from v4 to
  `actions/checkout@v6` for the current credential-handling implementation.
- [*] 2026-08-26: Renamed the main-branch workflow from `stage1.yml` to
  `ci.yml`; it remains the continuous-integration workflow.

- [*] 2026-08-26: Moved automation names and output-category prefixes into the
  dedicated `xtask/src/constants.rs` module, including `[info]`, `[error]`,
  `[warn]`, and `[manifest]` linkage.
- [*] 2026-08-26: Removed duplicated CLI/probe package-name output literals by
  deriving names from Cargo package metadata while retaining category prefixes.

- [*] 2026-08-26: Centralized workspace package and tool command names in the
  `xtask` constants namespace, replacing repeated command literals (including
  `NEUTRAL_COMPILER`), and linked host bootstrap scripts to safe
  `NEUTRAL_CARGO_COMMAND`/`NEUTRAL_RUSTC_COMMAND` overrides.

- [*] 2026-08-26: Completed Stage 2, Step 2. Added typed exact SHA-256 source
  identity, checked spans and line/column derivation, deterministic diagnostics
  and limits, cancellation/result classes, immutable capture, and the I/O-free
  compilation boundary. The SHA-256 dependency and its transitive closure are
  explicitly reviewed by automation policy.
- [*] 2026-08-26: Approved the v0 contract freeze with the repository owner,
  promoted the governing specifications and author guide, assigned the `0.1.0`
  contract family, and completed Stage 2, Step 1 with three frozen source cases
  and complete per-case oracles.
- [*] 2026-08-26: Classified all known contract-freeze questions in a blocking
  ledger and linked it from the freeze manifest and development entry point.
  Stage 2 remains blocked until every blocking entry is accepted and closed.
- [*] 2026-08-26: Added a responsibility and ecosystem README to every
  workspace package, including the non-production `xtask` package.
- [*] 2026-08-26: Added a review-candidate fixture/oracle registry for all 14
  current source fixtures. It locks source SHA-256 values and required oracle
  shapes while explicitly recording that no oracle is yet approved or immutable.
- [*] 2026-08-26: Organized generated evidence beneath `test-results/` by
  bootstrap, CI profile/stage, suite, and analysis category; CI runs now write
  to `test-results/ci/<profile>/run-<process-id>-<sequence>/`.
- [*] 2026-08-26: Started the mandatory contract-freeze gate with a draft
  manifest that hashes each governing source and records the unresolved approval,
  versioning, fixture/oracle, and review blockers. It does not authorize Stage 2.
- [*] 2026-08-26: Corrected the dev-container Apache-2.0 header to a JSONC
  comment so it is not interpreted as an unsupported configuration property.
- [*] 2026-08-26: Removed the time-based nightly workflow schedule; the nightly
  profile now runs only on pushes to `main` or manual dispatch.
- [*] 2026-08-26: Moved host bootstrap scripts to `scripts/linux/` and
  `scripts/win/`.
- [*] 2026-08-26: Stage 1, Step 1 completed. Created the 11-package virtual Rust
  workspace with explicit ownership, non-published support packages, pinned
  toolchain and quality configuration, documented behavior-free shells, and a
  committed lockfile. `cargo metadata`, formatting, workspace check, strict
  Clippy, tests, and documentation passed.
- [*] 2026-08-26: Stage 1, Step 2 completed. Added `cargo xtask boundary check`
  to enforce direct package dependencies, the pure compiler closure, and the
  standalone probe allowlist. Negative tests prove forbidden compiler and probe
  edges are rejected; the workspace audit remains green.
- [*] 2026-08-26: Added a workspace-enforced Rust documentation rule for every
  function, including private helpers and test functions; documented all current
  function definitions.
- [*] 2026-08-26: Stage 1, Step 3 and Stage 1 validation completed. Added
  bootstrap scripts, the pinned development container, active-suite and planned
  conformance configuration, workflow shells, and the full `cargo xtask`
  automation interface. `cargo xtask ci stage1` passed locally and in the
  network-disabled non-root development container.
- [*] 2026-08-26: Standardized current CLI, probe, bootstrap, and automation
  output as `[category] message`, including `[info]`, `[error]`, and
  `[manifest]` payloads.

## Working rule

Keep this file small. Update it when the active step, blocker, or completed
validation changes. Completion requires the validation evidence named by the
relevant stage or slice in `IMPLEMENTATION-STAGES.md`.
