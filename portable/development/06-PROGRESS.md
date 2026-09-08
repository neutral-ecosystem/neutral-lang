<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v0 development progress

Status: Stage 10 workflow overhaul in progress.

## Current focus

- Stage: Stage 10 workflow overhaul and v0 qualification
- Status: v0.1.0 draft prepared; publication and roadmap archive remain
- Last updated: 2026-09-08

## Next actions

- [x] Record the sole-maintainer approval and compensating review.
- [x] Implement tag-to-`main` identity enforcement and tag-only GitHub release
      publication with verified artifact transfer.
- [x] Commit the final release changes on `main`, rerun
      `cargo xtask release prepare`, and sign `v0.1.0` at that exact HEAD.
- [x] Push `main` and the signed tag, then verify the draft release assets and
      their exact digests.
- [ ] Publish the approved GitHub draft titled `neutral-lang v0.1.0`.
- [ ] Archive the v0 portable snapshot in the roadmap repository.

## Candidate policy

- Qualification always uses the clean checked-out `main` `HEAD`. `v0.1.0` is an
  eventual publication tag, created only after the candidate has passed every
  qualification and approval gate; it never selects an earlier source revision.

## Completed log

- [x] 2026-09-08: Prepared signed `v0.1.0` from qualified clean `main` commit
  `ceb439b5af0644682c3be95f6d9593a86b5de796`. GitHub release run
  `34245962106` passed tag-to-HEAD verification, the complete release
  preparation, checksum verification, artifact transfer, and draft creation.
  An isolated download of all eleven draft assets passed its own
  `SHA256SUMS`. The draft identity and asset digests are retained in
  `development/evidence/stage10-release-publication.md`.

- [x] 2026-09-08: Recorded the Stage 10 Step 9 technical, test/quality,
  security, release, and standards approvals under the disclosed
  sole-maintainer staffing exception. Hardened the release workflow to fetch
  `main`, reject a pushed tag that does not identify that exact HEAD, run the
  same stable `cargo xtask release prepare` command used locally, verify
  checksums before and after artifact transfer, and grant `contents: write`
  only to the tag-only publication job. Local YAML parsing, automation tests,
  formatting, and repository checks pass. Publication and roadmap archival
  remain pending the final clean committed HEAD and authenticated GitHub
  operations.

- [x] 2026-09-08: Completed Stage 10, Step 8 from clean `main` revision
  `b1353acc6d93c796795492d9e96976ed96f4f7fa`. The atomic package command
  generated the selected binaries, deterministic source archive, complete
  release manifest and checksums, locked dependency/SBOM inventory,
  provenance, installation guide, license, and release notes. crates.io was
  explicitly unselected. Every digest passed, the source archive regenerated
  byte-for-byte and excluded transient outputs, and an isolated consumer used
  the packaged CLI and probe successfully. Evidence is in
  `development/evidence/stage10-distribution-assembly.md`.

- [x] 2026-09-08: Completed Stage 10, Step 7 from clean `main` revision
  `088541693a9382bd1d00abd3ce32437ed496ed7e`. Built every release deliverable
  and Rustdoc; passed public documentation tests; exercised positive compile,
  validation, formatting, and standalone inspection; and proved invalid,
  cancellation, source-limit, vocabulary-lock, and output-publication paths
  leave no valid replacement. Copies of only the release CLI/probe ran outside
  the workspace with explicit source and vocabulary inputs. Reader/probe
  equivalence and the compiler-free probe dependency closure passed. Evidence
  is in `development/evidence/stage10-consumer-deliverables.md`.

- [x] 2026-09-08: Completed Stage 10, Step 6 against clean `main` revision
  `47633a635deafa6d56260442a02e5aab14ff9441`. Release CI and unchanged
  85%/90%/80% coverage gates passed at 93.86% lines, 93.66% functions, and
  85.16% regions. Release and 5,000-iteration soak profiles completed. No
  production, fuzz-target, production-dependency, or allocation-affecting
  change followed Stage 9, so its complete mutation, five 900-second fuzz, and
  Valgrind evidence remains applicable. Fresh cargo-audit 0.22.2 scans loaded
  1,242 advisories and found no vulnerability in either lockfile. Evidence is
  in `development/evidence/stage10-candidate-qualification.md`.

- [x] 2026-09-08: Completed Stage 10, Step 5. Split lightweight stable
  bootstrap from the complete release-workstation audit; added actionable
  checks and path-independent evidence for Rust/Cargo, Rustfmt, Clippy, isolated
  nightly LLVM/fuzz tooling, mutation, Valgrind, and release-shell utilities;
  hardened supported-host and ignore policy; and rejected ambient effects in
  the pure compiler closure plus user-specific release paths. A temporary clean
  Git tree rebuilt and passed `cargo xtask ci pr` fully offline with empty final
  status. Command-local nightly fuzz smoke and coverage passed while stable
  Rust remained selected. The clean run also exposed and fixed a portable link
  to absent generated Rustdoc. Evidence is in
  `development/evidence/stage10-environment-reproduction.md`.

- [x] 2026-09-08: Completed Stage 10, Step 4. Added an executable top-level
  ownership/lifecycle inventory and missing directory READMEs; made every
  durable test level independently runnable across owning crates; enforced
  generated/archive hygiene, fixture/test layout, subsystem fuzz ownership,
  and manifest/Rustdoc warning policy; expanded the contributor path; and kept
  CI as thin stable-command adapters. Nightly coverage now retains HTML and
  JSON and passes unchanged 85%/90%/80% gates at 93.86% lines, 93.66%
  functions, and 85.16% regions under the reviewed production scope. Release
  quality, documentation, all individual test levels, fuzz regression, and the
  PR performance profile pass. Evidence is in
  `development/evidence/stage10-repository-quality-overhaul.md`.

- [x] 2026-09-08: Completed Stage 10, Step 3. Root
  `[workspace.package].version` now derives the release tag and every package
  inherits it. Version tooling displays all independent frozen contract
  domains, verifies offline lock/source policy, enforces forward SemVer
  transitions, and emits deterministic review plans without editing contracts
  or tagging. Added generated-output ownership, active portable lifecycle
  identity, local-link/fixture-digest/archive-boundary checks, atomic
  digest-addressed snapshots, snapshot revalidation, and the explicit
  v0-roadmap-archive to v1-rollover procedure. Evidence is in
  `development/evidence/stage10-version-portable-lifecycle.md`.

- [x] 2026-09-08: Simplified release identity configuration to the final
  `v0.1.0` tag. Removed duplicated `candidate_commit` and
  `candidate_evidence` inputs: release tooling now resolves the authoritative
  commit from Git and requires the annotated tag to name the clean checked-out
  `HEAD`; canonical Stage 9 residual-risk approval remains mandatory.

- [x] 2026-09-08: Completed Stage 10, Step 2 as an implementation overhaul.
  Added one typed, stage-free `xtask` grammar; durable format/lint/check/test/
  quality/build/validate/package/release/version/portable/clean entry points;
  current rather than stage-selected test minimums; fail-closed release-scope,
  tag, evidence, clean-tree, and binary selection checks; machine-readable
  aggregate summaries; and executable command-document/workflow coherence.
  Added the exact Linux/Windows bootstrap/environment/release adapter trees and
  responsibility READMEs, replaced duplicated CI YAML policy with stable
  commands, and published the root migration table. The PR quality composition,
  release build/binary validation, Linux bootstrap, version/portable checks,
  strict Clippy, automation tests, traceability, and workflow checks pass.
  Package assembly correctly rejected the superseded candidate rather than
  building from the wrong revision; full evidence is in
  `development/evidence/stage10-workflow-overhaul.md`.

- [x] 2026-09-08: Completed Stage 10, Step 1. The sole maintainer approved the
  Stage 9 residual-risk treatment for candidate preparation and selected an
  annotated source tag plus GitHub binary assets as the v0 distribution scope;
  crates.io publication is excluded. Created local annotated tag
  `v0.1.0-rc.1` for the recorded clean `main` revision. Nothing was pushed,
  published, or uploaded.

- [x] 2026-09-08: Began Stage 10, Step 1 without making release-authority
  decisions. Reconfirmed the approved freeze and all governing contract
  revisions, verified its fixture and oracle-manifest digests, passed
  traceability coherence, and recorded the clean inspected `main` revision,
  lockfile digest, toolchain contents, and fixture identities in
  `development/evidence/stage10-candidate-preparation.md`. No candidate tag,
  publication, version bump, or distribution target has been assumed.

- [x] 2026-09-08: Reframed Stage 10 as a workflow overhaul followed by v0
  qualification. It now requires implementation of a stable stage-free `xtask`
  interface, thin `scripts/linux` / `scripts/win` adapters, centralized
  workspace version tooling, deterministic generated/lock metadata checks, and
  the active-portable → immutable-roadmap-archive → next-version-portable
  lifecycle before candidate qualification, packaging, and publication proceed.

- [x] 2026-09-08: Installed the approved system-wide Stage 9 tooling and reran
  the configured whole-workspace LLVM coverage gate on nightly. It passed at
  90.57% lines, 90.71% functions, and 81.72% regions against 85%/90%/80%
  thresholds. The final 271-mutant broad review caught 244, classified 27 as
  unviable, and left no missed viable mutant. Coverage and mutation were
  closed; component-level allocation evidence remained the only Stage 9 gap at
  that point.

- [x] 2026-09-08: Ran Valgrind 3.27.1 against the direct optimized Stage 9
  benchmark. Massif measured a 524,640 B total peak for both release and the
  50,000-iteration extended soak; useful heap differed by only six bytes.
  Memcheck found zero memory errors and no definite, indirect, or possible
  leaks across 322,503 allocations. Five direct release samples completed in
  0.02 seconds each with 3,616–3,848 KiB peak RSS. Allocation evidence is
  closed; Stage 9 is ready for maintainer residual-risk approval.

- [x] 2026-09-07: Completed every required full-duration libFuzzer campaign on
  an untraced runner: source (7,517,511 executions), vocabulary (10,720,881),
  IR (85,143,833), formatter (9,337,700), and probe (86,633,280), each for at
  least 900 seconds without a failure. Recorded controlled 250-iteration
  release and 50,000-iteration extended-soak baselines, including whole-command
  peak RSS. The expanded 271-mutant review completed with 178 caught, 66
  missed, and 27 unviable mutants, so its gate correctly remains open.

- [x] 2026-09-07: Added exact-number canonical reconstruction and source
  normalization boundary tests. The focused `ExactNumber` mutation subset now
  catches 34 of 37 generated mutants (with 3 unviable and none missed).
  Workspace formatting, strict Clippy, all targets, traceability, dependency
  boundaries, and crate-local test-layout checks pass.

- [x] 2026-09-07: Provisioned checksum-verified isolated Stage 9 tools without
  changing the stable repository toolchain. Reconfirmed 38/38 critical mutants
  caught; ran all five libFuzzer targets for 256 readiness executions each;
  completed local 250-iteration release and 5,000-iteration soak profiles; and
  produced the first retained LLVM coverage summary. Coverage measured
  84.91% lines, 80.46% functions, and 75.55% regions and therefore correctly
  failed the configured 85%/90%/80% gate. Repeated, concurrent, and adversarial
  determinism remains green and its Stage 9 validation item is approved.

- [x] 2026-09-07: Activated Stage 9 in the repository configuration and
  replaced the transitional automation stubs with real configuration-driven
  quality entry points. `cargo xtask coverage` now enforces the configured
  line/function/region thresholds, `cargo xtask mutate` targets the configured
  critical module, and `cargo xtask fuzz campaign` runs all five configured
  coverage-guided targets for their configured budgets. Automation unit tests,
  Clippy, environment verification, and test-layout validation pass. Initial
  direct probes reported the three absent external Cargo tools honestly;
  deterministic fuzz smoke remains separate from coverage-guided evidence. The
  complete Stage 9-aware `cargo xtask ci pr` profile passes.

- [x] 2026-09-06: Began Stage 9 hardening. Moved every inline Rust test body
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

- [x] 2026-09-06: Completed Stage 8, Step 4 and Stage 8 validation. Added the
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

- [x] 2026-09-06: Completed Stage 8, Step 3. The compiler-free
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

- [x] 2026-09-06: Migrated the active v0 portable package to the homogeneous
  Neutral roadmap layout. `PLAN.md` is now the operational entry point;
  architecture is root-level; normative requirements, contracts, decisions,
  examples, and fixtures are grouped under `specs/`; lifecycle documents are
  numbered under `development/`; implementation reviews live under
  `development/evidence/`; and executable manifests and oracles live under
  `conformance/`. Updated every repository path consumer, Markdown link,
  include, oracle, review record, automation path, and freeze digest while
  preserving the accepted v0 behavior and Stage 8 Step 2 status.

- [x] 2026-09-06: Completed Stage 8, Step 2. Activated strict built-binary
  `compile`, `validate`, and `format` commands with explicit source, destination,
  captured-vocabulary lock, structural-limit, overwrite, standard-stream, and
  cooperative-cancellation policy. Output uses synchronized same-directory
  temporary files and atomic commit semantics; validation, cancellation,
  permission, commit, and broken-pipe failures publish no partial authoritative
  output. Stable usage and exit classes, path-safe diagnostics, Unicode and
  spaced paths, exact vocabulary acquisition, clean stdout artifacts, and
  compiler-free artifact decoding are covered at the child-process boundary.
  Inspection remains exclusively owned by the independent `neutral-probe`.

- [x] 2026-09-06: Completed Stage 8, Step 1. Added the I/O-free public
  `format`/`format_captured` boundary backed by compiler-private syntax and
  trivia, canonical LF/header/spacing/four-space/multiline-comma rendering, and
  deterministic source-order comment placement. Every positive source fixture
  is idempotent after one formatting pass and recompiles to alpha-equivalent
  logical IR with identical value, field, reuse, and reference provenance.
  Formatted bytes are explicitly ordinary recapturable source rather than IR,
  encoded-artifact identity, or signing material.

- [x] 2026-09-06: Completed Stage 7 validation. Activated the standalone
  `neutral-probe` encoded-artifact path through public core, encoding, and
  reader contracts; added a compiler-free process-boundary system proof and
  dependency-closure enforcement; activated reproducible truncation,
  structured-mutation, and arbitrary-byte decoder fuzz targets; and approved
  the bounds-before-allocation review. Exact-number reconstruction now validates
  borrowed coefficient text before retaining its owned copy.

- [x] 2026-09-06: Replaced the exact development/release Rust pin with the
  rolling `stable` channel. CI and release workflows now install current stable,
  bootstrap accepts stable distribution compilers while rejecting beta/nightly,
  and evidence records the selected channel plus exact resolved compiler. The
  separately declared Rust 1.97.1 MSRV remains unchanged.

- [x] 2026-09-06: Completed Stage 7, Step 3. Added a bounds-first decoder for
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

- [x] 2026-09-05: Completed Stage 7, Step 2. Added the `neutral-encoding`
  boundary, which accepts only immutable `ValidatedDocument` input and emits
  the five fixed Neutral IR Framed CBOR 0.1 sections under centralized framing,
  schema, capability, and size constants. The encoder derives capabilities from
  actual types, values, vocabulary, and provenance; preserves logical,
  source-map, provenance, and derivation contracts; records exact SHA-256
  section integrity; and confines producer/build facts to the envelope. Every
  current positive source fixture, including captured vocabulary, encodes within
  limits. Nonmutation, envelope isolation, deterministic implementation output,
  fixed framing, and oversized producer rejection are executable evidence.

- [x] 2026-09-03: Added `cargo docs`, which builds workspace rustdoc and
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

- [x] 2026-09-03: Completed Stage 7, Step 1. Selected Neutral IR Framed CBOR
  0.1 after comparing JSON, Protocol Buffers, MessagePack, FlatBuffers, and a
  restricted CBOR profile. Froze the fixed header and section directory,
  capability assignments, five complete artifact sections, exact-number
  representation, closed CBOR schemas, hard allocation ceilings, validation
  order, malformed/unsupported result classes, and the rule that external bytes
  are noncanonical while logical equality remains structural alpha-equivalence.
  The security/allocation review approved bounds-before-allocation and
  fail-closed version, capability, integrity, and schema handling.

- [x] 2026-09-03: Completed Stage 6, Slice 6.2 and Stage 6 validation. Activated
  optional `use` and `Vocabulary::Type` grammar; exact host-captured bundle and
  lock inputs; bundle-before-payload validation; qualified contextual values;
  distinct captured-default provenance; exact vocabulary identity, version,
  schema, encoding, digest, feature, logical schema, and derivation records;
  fail-closed reader validation; and generic probe enumeration. Frozen cases
  now cover missing and mismatched captures, namespace collisions, unknown
  features/types, executable shapes, and missing, duplicate, unknown, and
  incompatible payload fields without registry, path, or network acquisition.

- [x] 2026-09-02: Completed Stage 6, Slice 6.1. Added typed exact vocabulary
  digests and strict lowercase digest parsing; exact lock verification before
  parsing; a dependency-free bounded JSON decoder that retains object members
  until duplicate detection; closed envelope, feature, name, type, field,
  default, and recursion validation; immutable separate captured/logical bundle
  projections; reference-only recursive type graphs; and grouped accepted and
  hostile fixtures with frozen oracles. Raw numbers, BOM/UTF-8/surrogate errors,
  unknown or executable shapes, bad defaults/targets, embedded cycles, digest
  mismatch, and allocation limits fail closed without code loading or I/O.

- [x] 2026-09-01: Completed Stage 5, Slice 5.3 and Stage 5 validation. Replaced
  literal graph-ID equality with a one-to-one whole-document mapping across
  record and binding nodes; recursively compares identity edges by mapped IDs;
  separated logical-payload comparison from exact companion/envelope equality;
  made the hostile reader independently recompute logical fingerprints;
  documented the cross-document `ElementId` prohibition; and added reflexive,
  symmetric, transitive, generated-renaming, changed payload/edge/fingerprint,
  duplicate-ID, dangling-ID, and hostile-reader property vectors. All Stage 2–5
  compiler, reader, probe, boundary, conformance, property, and security suites
  remain green.

- [x] 2026-08-31: Completed Stage 5, Slice 5.2. Added invariant `Ref<T>` and
  `ref(name)` grammar, forward exact-type target resolution, a distinct
  reference failure class and stable diagnostics, reference-only nominal
  recursion, identity cycles outside value dependencies, graph-local IR edges
  with durable-symbol fingerprints, reference provenance, hostile-reader edge
  validation, ID-based probe traversal, and frozen grouped fixtures/oracles.
  Ordinary reuse and typed identity references remain distinct end to end.

- [x] 2026-08-31: Completed Stage 5, Slice 5.1. Activated unqualified immutable
  value reuse after full declaration collection; added deterministic dependency
  ordering, forward/transitive/nested reuse, exact outer-nullable widening,
  invariant container checks, bounded chains, stable unknown/wrong-kind/cycle
  diagnostics with primary and related locations, final-value fingerprints,
  explicit reuse provenance, hostile-reader validation, probe exposure, and
  frozen grouped Stage 5 fixtures/oracles. Identity references remain inactive
  for Slice 5.2.

- [x] 2026-08-30: Completed Stage 4, Slice 4.3 and Stage 4 validation. Added
  invariant `List<T>` grammar and IR, ordered/empty/nested/nullable lists,
  closed record-list defaults, list fingerprints and reader/probe traversal,
  item/depth/traversal limits, grouped fixtures/oracles, and removal of
  transitional negative cases whose list syntax is now valid v0.

- [x] 2026-08-29: Relocated the obsolete `portable/conformance` README to the
  root conformance asset directory, clarified manifest/oracle/report ownership,
  and replaced stale Stage 1 CLI/probe shell wording with the scheduled Stage 8
  activation boundary.

- [x] 2026-08-29: Completed Stage 4, Slice 4.2. Added closed scalar, null, and
  recursively contextual record defaults; omission materialization; schema and
  definition fingerprint defaults; explicit versus user-default field
  provenance; reader/probe validation and exposure; retained-string resource
  accounting; stable non-constant/type/syntax rejection; and frozen grouped
  fixtures/oracles.

- [x] 2026-08-29: Completed Stage 4, Slice 4.1. Added bounded record grammar,
  two-pass root collection, nominal schema resolution, contextual and nested
  record lowering, duplicate/missing/unknown/wrong-kind/recursion diagnostics,
  canonical declaration and field order, public reader/probe traversal, and
  frozen Stage 4 fixtures/oracles.

- [x] 2026-08-27: Completed Stage 3, Slice 3.4 and Stage 3 validation. Added
  recursive outer-nullable resolved types, explicit typed null, exact scalar to
  nullable widening, null fingerprints/provenance, reader type/value checks,
  probe traversal, frozen nullability fixtures/oracles, and future grammar
  exclusion evidence while keeping Stage 2 green.

- [x] 2026-08-27: Completed Stage 3, Slice 3.3. Added frozen signed decimal
  grammar, separator/fraction/exponent normalization, canonical zero, exact
  NHT-backed fingerprints, explicit numeric digit/scale limits, stable
  numeric-limit diagnostics, and grouped positive/negative numeric fixtures
  with conformance evidence.

- [x] 2026-08-27: Reorganized v0 source fixtures by outcome and primary
  feature (`syntax`, `identifiers`, `strings`, `booleans`, `values`, and
  `vocabulary`); updated manifests, oracles, compiler includes, and fixture
  documentation to use the grouped paths.

- [x] 2026-08-27: Replaced duplicated compiler diagnostic literals with the
  owning `neutral-compiler::diagnostics` and `neutral-probe::diagnostics`
  namespaces; cross-package version assertions now use exported compiler and
  IR contract constants instead of raw version strings.

- [x] 2026-08-27: Completed Stage 3, Slice 3.2. Added bounded string escape and
  Unicode-scalar decoding, exact Boolean tokens, scalar type checking, typed IR
  and fingerprints, safe reader/probe rendering, decoded-string resource
  accounting, stable invalid-string/type/limit diagnostics, and frozen
  positive/negative conformance evidence.

- [x] 2026-08-27: Centralized frozen language spellings and the protected-core
  namespace in compiler-private `language.rs`; lexer, parser, and semantics now
  share the same named constants rather than repeating source-language strings.

- [x] 2026-08-27: Completed Stage 3, Slice 3.1. Froze nine identifier,
  protected-name, comment, punctuation, newline, and string-boundary cases;
  implemented exact private trivia retention, full ASCII name classification,
  stable boundary diagnostics, and raw newline behavior; and verified logical
  comment invariance, source facts, reader/probe output, deterministic hostile
  comment rejection, limits, and future-grammar exclusion.

- [x] 2026-08-26: Completed Stage 2, Step 4 and Stage 2 validation. The minimal
  fixture now validates names/types/exact values, receives frozen NHT-backed
  identities, lowers to immutable logical IR/source-map/provenance/derivation
  artifacts, traverses through reader/probe contracts, and maps a consumer
  diagnostic back to source. Active unit, smoke, integration, system,
  conformance, property, security, and fuzz-smoke suites pass under
  `cargo xtask ci pr`.

- [x] 2026-08-26: Completed Stage 2, Step 3. Added the private minimal raw
  lexer, physical-newline retention, semantic line-end normalization, and exact
  parser for `neu "0.1"`, one module header, and one `num` binding. Frozen
  negative diagnostics and spans, newline equivalence, BOM/UTF-8/NUL safety,
  and private parser boundaries are covered by tests.

- [x] 2026-08-26: Configured continuous integration for every push to `main`
  and release qualification for every pushed tag; both workflows retain manual
  dispatch.
- [x] 2026-08-26: Upgraded workflow repository checkout steps from v4 to
  `actions/checkout@v6` for the current credential-handling implementation.
- [x] 2026-08-26: Renamed the main-branch workflow from `stage1.yml` to
  `ci.yml`; it remains the continuous-integration workflow.

- [x] 2026-08-26: Moved automation names and output-category prefixes into the
  dedicated `xtask/src/constants.rs` module, including `[info]`, `[error]`,
  `[warn]`, and `[manifest]` linkage.
- [x] 2026-08-26: Removed duplicated CLI/probe package-name output literals by
  deriving names from Cargo package metadata while retaining category prefixes.

- [x] 2026-08-26: Centralized workspace package and tool command names in the
  `xtask` constants namespace, replacing repeated command literals (including
  `NEUTRAL_COMPILER`), and linked host bootstrap scripts to safe
  `NEUTRAL_CARGO_COMMAND`/`NEUTRAL_RUSTC_COMMAND` overrides.

- [x] 2026-08-26: Completed Stage 2, Step 2. Added typed exact SHA-256 source
  identity, checked spans and line/column derivation, deterministic diagnostics
  and limits, cancellation/result classes, immutable capture, and the I/O-free
  compilation boundary. The SHA-256 dependency and its transitive closure are
  explicitly reviewed by automation policy.
- [x] 2026-08-26: Approved the v0 contract freeze with the repository owner,
  promoted the governing specifications and author guide, assigned the `0.1.0`
  contract family, and completed Stage 2, Step 1 with three frozen source cases
  and complete per-case oracles.
- [x] 2026-08-26: Classified all known contract-freeze questions in a blocking
  ledger and linked it from the freeze manifest and development entry point.
  Stage 2 remains blocked until every blocking entry is accepted and closed.
- [x] 2026-08-26: Added a responsibility and ecosystem README to every
  workspace package, including the non-production `xtask` package.
- [x] 2026-08-26: Added a review-candidate fixture/oracle registry for all 14
  current source fixtures. It locks source SHA-256 values and required oracle
  shapes while explicitly recording that no oracle is yet approved or immutable.
- [x] 2026-08-26: Organized generated evidence beneath `test-results/` by
  bootstrap, CI profile/stage, suite, and analysis category; CI runs now write
  to `test-results/ci/<profile>/run-<process-id>-<sequence>/`.
- [x] 2026-08-26: Started the mandatory contract-freeze gate with a draft
  manifest that hashes each governing source and records the unresolved approval,
  versioning, fixture/oracle, and review blockers. It does not authorize Stage 2.
- [x] 2026-08-26: Corrected the dev-container Apache-2.0 header to a JSONC
  comment so it is not interpreted as an unsupported configuration property.
- [x] 2026-08-26: Removed the time-based nightly workflow schedule; the nightly
  profile now runs only on pushes to `main` or manual dispatch.
- [x] 2026-08-26: Moved host bootstrap scripts to `scripts/linux/` and
  `scripts/win/`.
- [x] 2026-08-26: Stage 1, Step 1 completed. Created the 11-package virtual Rust
  workspace with explicit ownership, non-published support packages, pinned
  toolchain and quality configuration, documented behavior-free shells, and a
  committed lockfile. `cargo metadata`, formatting, workspace check, strict
  Clippy, tests, and documentation passed.
- [x] 2026-08-26: Stage 1, Step 2 completed. Added `cargo xtask boundary check`
  to enforce direct package dependencies, the pure compiler closure, and the
  standalone probe allowlist. Negative tests prove forbidden compiler and probe
  edges are rejected; the workspace audit remains green.
- [x] 2026-08-26: Added a workspace-enforced Rust documentation rule for every
  function, including private helpers and test functions; documented all current
  function definitions.
- [x] 2026-08-26: Stage 1, Step 3 and Stage 1 validation completed. Added
  bootstrap scripts, the pinned development container, active-suite and planned
  conformance configuration, workflow shells, and the full `cargo xtask`
  automation interface. `cargo xtask ci stage1` passed locally and in the
  network-disabled non-root development container.
- [x] 2026-08-26: Standardized current CLI, probe, bootstrap, and automation
  output as `[category] message`, including `[info]`, `[error]`, and
  `[manifest]` payloads.

## Working rule

Keep this file small. Update it when the active step, blocker, or completed
validation changes. Completion requires the validation evidence named by the
relevant stage or slice in `03-IMPLEMENTATION-STAGES.md`.
