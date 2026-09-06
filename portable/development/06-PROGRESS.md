<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v0 development progress

Status: Stage 9 hardening in progress.

## Current focus

- Stage: Stage 9 hardening
- Status: stable hardening gates pass; external coverage/fuzz/release performance evidence remains
- Last updated: 2026-09-06

## Next actions

- [ ] Run all five coverage-guided fuzz targets for the approved time budget.
- [ ] Supply LLVM coverage tools and meet the configured thresholds.
- [ ] Complete remaining named mutation targets and controlled release/memory/soak profiles.

## Blockers

- The current stable environment has neither `rustup` nor
  `llvm-tools-preview`, so `cargo-llvm-cov` cannot produce coverage evidence.
- No compatible nightly/libFuzzer toolchain is present to run `cargo-fuzz`.

## Completed log

- [*] 2026-09-06: Began Stage 9 hardening. Moved every inline Rust test body
  into path-based crate-local `tests/` modules and added a CI layout gate.
  Expanded metamorphic, concurrent/adversarial determinism, exact structural
  boundary, source-fact isolation, and stable source/vocabulary/IR/formatter/
  probe mutation campaigns. Added cancellation checks and executable tests at
  every compiler handoff, a dependency-free controlled benchmark/stress/soak
  harness, five isolated `cargo-fuzz` targets, quality thresholds, standards
  register, threat model, dependency/unsafe/native review, cache-isolation
  review, static review, quality evaluation, and residual-risk record. RustSec
  found no advisory in the current 22-dependency lockfile. After an initial 38
  surviving critical language-predicate mutants, exhaustive tests now catch all
  38. Coverage-guided runs, LLVM coverage, broader mutation, and dedicated
  release/memory/soak evidence remain open, so Stage 9 is not yet approved.

- [*] 2026-09-06: Completed Stage 8, Step 4 and Stage 8 validation. Added the
  published requirement/syntax evidence index spanning decisions, fixtures,
  implementation owners, and executable tests; checked every master syntax
  item only after its completed vertical evidence was identified. Added
  `cargo xtask traceability check` to PR/release CI to reject missing accepted
  IDs, divergent or unchecked syntax inventories, unregistered normative
  fixtures/oracles, and missing manifest paths. The full example embedded in
  the language showcase now compiles against the exact captured vocabulary as
  conformance evidence. Rustdoc/doc tests, dependency boundaries, explicit
  exclusions, compiler-private model isolation, and all active Stage 8 suites
  pass.

- [*] 2026-09-06: Completed Stage 8, Step 3. The compiler-free
  `neutral-probe` now enumerates logical/identity metadata, source and
  derivation facts, resource and acceptance facts, schemas, final typed values,
  vocabulary contracts, source mappings, and every provenance category through
  validated public reader views. Rendering is shared by the library and binary;
  a manually constructed public-IR artifact proves exact in-process, decoded,
  and executable output parity without compiler linkage. Separate tests prove
  consumer diagnostics retain the expected original source digest/span and
  hostile external traversal obeys caller-selected decoder limits. PR and
  release CI already enforce the complete standalone-probe dependency
  allowlist.

- [*] 2026-09-06: Migrated the active v0 portable package to the homogeneous
  Neutral roadmap layout. `PLAN.md` is now the operational entry point;
  architecture is root-level; normative requirements, contracts, decisions,
  examples, and fixtures are grouped under `specs/`; lifecycle documents are
  numbered under `development/`; implementation reviews live under
  `development/evidence/`; and executable manifests and oracles live under
  `conformance/`. Updated every repository path consumer, Markdown link,
  include, oracle, review record, automation path, and freeze digest while
  preserving the accepted v0 behavior and Stage 8 Step 2 status.

- [*] 2026-09-06: Completed Stage 8, Step 2. Activated strict built-binary
  `compile`, `validate`, and `format` commands with explicit source, destination,
  captured-vocabulary lock, structural-limit, overwrite, standard-stream, and
  cooperative-cancellation policy. Output uses synchronized same-directory
  temporary files and atomic commit semantics; validation, cancellation,
  permission, commit, and broken-pipe failures publish no partial authoritative
  output. Stable usage and exit classes, path-safe diagnostics, Unicode and
  spaced paths, exact vocabulary acquisition, clean stdout artifacts, and
  compiler-free artifact decoding are covered at the child-process boundary.
  Inspection remains exclusively owned by the independent `neutral-probe`.

- [*] 2026-09-06: Completed Stage 8, Step 1. Added the I/O-free public
  `format`/`format_captured` boundary backed by compiler-private syntax and
  trivia, canonical LF/header/spacing/four-space/multiline-comma rendering, and
  deterministic source-order comment placement. Every positive source fixture
  is idempotent after one formatting pass and recompiles to alpha-equivalent
  logical IR with identical value, field, reuse, and reference provenance.
  Formatted bytes are explicitly ordinary recapturable source rather than IR,
  encoded-artifact identity, or signing material.

- [*] 2026-09-06: Completed Stage 7 validation. Activated the standalone
  `neutral-probe` encoded-artifact path through public core, encoding, and
  reader contracts; added a compiler-free process-boundary system proof and
  dependency-closure enforcement; activated reproducible truncation,
  structured-mutation, and arbitrary-byte decoder fuzz targets; and approved
  the bounds-before-allocation review. Exact-number reconstruction now validates
  borrowed coefficient text before retaining its owned copy.

- [*] 2026-09-06: Replaced the exact development/release Rust pin with the
  rolling `stable` channel. CI and release workflows now install current stable,
  bootstrap accepts stable distribution compilers while rejecting beta/nightly,
  and evidence records the selected channel plus exact resolved compiler. The
  separately declared Rust 1.97.1 MSRV remains unchanged.

- [*] 2026-09-06: Completed Stage 7, Step 3. Added a bounds-first decoder for
  the fixed frame and duplicate-preserving restricted CBOR; stable classified
  failures for size, framing, versions, capabilities, integrity, schema,
  logical IR, source maps, provenance, derivation, cancellation, and internal
  defects; exact reconstruction of every logical and companion contract; and a
  final trusted-reader gate before any public view is returned. All positive
  fixtures round-trip exactly, while frame, duplicate-key, version, integrity,
  ownership, name-category, fingerprint, derivation, vocabulary-identity, limit,
  cancellation, and single-byte mutation vectors fail boundedly. Captured
  vocabulary mismatches require no registry, path, network, or other lookup.
  Compiler, vocabulary, IR, and decoder name checks now share one protected-name
  and ASCII-category contract module instead of duplicating language spellings.

- [*] 2026-09-05: Completed Stage 7, Step 2. Added the `neutral-encoding`
  boundary, which accepts only immutable `ValidatedDocument` input and emits
  the five fixed Neutral IR Framed CBOR 0.1 sections under centralized framing,
  schema, capability, and size constants. The encoder derives capabilities from
  actual types, values, vocabulary, and provenance; preserves logical,
  source-map, provenance, and derivation contracts; records exact SHA-256
  section integrity; and confines producer/build facts to the envelope. Every
  current positive source fixture, including captured vocabulary, encodes within
  limits. Nonmutation, envelope isolation, deterministic implementation output,
  fixed framing, and oversized producer rejection are executable evidence.

- [*] 2026-09-03: Added `cargo docs`, which builds workspace rustdoc and
  generates `target/doc/index.html` from Cargo metadata. The responsive landing
  page automatically groups publishable and internal packages, exposes package
  descriptions, owners, versions, searchable dependency links, and counts, and
  contains no hand-maintained crate list. Each complete package card opens its
  API while dependency chips retain their own links. Every generated rustdoc
  page also receives a page-depth-aware back button to the workspace index and
  higher-contrast inline-code, signature-line, `Source`, and `unstable` styling.
  A content-derived cache token forces rustdoc regeneration whenever the shared
  header or theme changes. CI uses the same generator, while all generated
  output remains ignored.

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
relevant stage or slice in `03-IMPLEMENTATION-STAGES.md`.
