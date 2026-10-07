<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 7 contract, vectors, and identity integration

Reviewed: 05-10-2026. Scope: contract/vector, core, integration, and validation gates; no package
version bump, tag, approval, or release is performed.

The [identity contract](../../specs/contracts/PROJECT-IDENTITY.md) is pinned in
[freeze.toml](../../specs/contracts/freeze.toml). The
[conformance manifest](../../conformance/manifest.toml) pins six literal complete
NHT transcript/SHA-256 vectors and their oracle. Crate-owned vector copies remain
executable after the portable plan is archived.

## Contract decisions and ownership

- `neutral-core` owns the existing shared NHT envelope and SHA-256 primitive.
  `neutral-ir::project_identity` owns the frozen profile, four separate domain
  tags, typed identities, and exact framing/field order. Package release numbers
  do not select identity behavior; changing the frozen schema requires new
  domains/profile and new vectors, not overwriting accepted expectations.
- Captured identity covers every exact source and vocabulary lock, including
  disconnected members, byte digests/lengths, and source IDs. Capture permutation,
  correlation keys, host mappings, controls, and roots are excluded. The compiler
  adapter projects already validated capture facts without I/O or recapture.
- Complete logical identity covers private declarations, disconnected modules,
  canonical signatures/values/defaults, imports, locked semantic vocabulary
  schemas, and public dependency topology. It excludes source/byte evidence,
  aliases, graph-local labels, host mappings, roots, and processing companions.
  Exact numbers and decoded strings normalize; list order and inert location
  types remain significant. Public fingerprints are not complete project hashes.
- Derivation adds exact captured identity, explicit producer identity/revision,
  and acceptance/producer limits. Artifact identity adds explicit kind, format,
  normalized view roots, and options. Artifact identity is not a checksum of
  arbitrary noncanonical wire bytes. These digests confer no authority.
- Construction uses independent intersected byte/node limits, shared type/value
  depth bounds, bounded comparison keys, checked arithmetic, fallible temporary
  reservations, in-place frames, and cancellation before publication. Errors
  return `Limit`, `InvalidInput`, or `Cancelled`, never a partial transcript.
  Structural identity construction does not replace semantic reader validation
  or validation of a view's root existence/publicness.

## Initial contract/core checks

| Command | Outcome |
| --- | --- |
| `cargo xtask fixtures sync` / `cargo xtask fixtures check` | 51 fixture/oracle pairs registered; literal vector and contract digests pinned |
| `cargo test --package neutral-test-suite project_identity` | 13 identity tests pass; all six literal transcripts and SHA-256 digests match exactly |
| `cargo test --package neutral-core` | Complete-transcript hashing preserves existing NHT framing; all foundational tests pass |
| Strict Clippy over core, IR, compiler, and cross-package tests | Pass; no lint suppression introduced |
| `cargo xtask ci pr` | Pass: 465 workspace tests, zero skips, doc tests, strict lint, dependency/hygiene checks, smoke, standalone probe build, and generated docs |

Identity tests additionally cover vocabulary permutation/alias/byte-evidence
separation, producer/control changes, root normalization, private-content
non-equivalence, all four layers at exact byte/node boundaries and one below,
zero budgets, duplicate/malformed context, aggregate import bounds, depth, and
cancellation. Routine toolchain/command results stay in ignored workflow logs,
not separate Markdown reports.

## Public integration and validation review

The crate-owned [independent encoder](../../../crates/neutral-test-suite/tests/project_identity/reference.py)
uses Python standard-library big-endian packing and SHA-256. It does not import
Rust framing, transcripts, or computed expectations. The test adapter projects
typed input facts only. All six unchanged literal byte/hash baselines agree;
the complete/private/cyclic/vocabulary/reuse/location corpus and typed-value
cases also agree. Separate malformed capture, canonical order, derivation,
artifact option/root, cancellation, depth, exact-byte/node, and one-over-work
vectors compare acceptance or failure classifications. Framed tuple boundaries
distinguish delimiter-free collision paths. Missing Python fails, never skips.

`ValidatedProject` exposes complete logical transcripts and a context-bound set
of typed captured/logical/derivation identities with processing facts. Explicit
capture companions must match the validated IR; producer and lock claims remain
caller inputs, not authentication or proof of faithful compilation. Producer
text is bounded before ownership. Artifact construction validates public/existing
roots before publication. Empty, subset, full, and permuted root selections
leave all complete upstream bytes/digests unchanged. The standalone reader-only
probe computes the same complete logical digest without compiler linkage.

The compiler's caller-owned syntax cache executes real parse reuse, with parsed,
reused, and rejected unit counts. Source-sensitive syntax keys use exact source
digest **and bytes**, module, and profile; logical identity/public fingerprints
never substitute for those keys. Private source changes reparse the affected
unit. Changed source IDs, capture order, vocabulary bytes/visibility, and controls
always reconstruct current graph, semantics, source maps, provenance, accounting,
and complete output. Invalid runs preserve the prior generation. Tests compare
full IR, derivation/logical transcripts, diagnostics, and encoded artifacts with
clean compilation, including shuffled concurrent schedules, stale/misplaced
entries, simulated digest collisions, and retention bounds. There is no external
cache import, persistent cache, shared global cache, or final-artifact memoization.

Immutability review: the normative contract, profile, four domain tags, framing,
SHA-256 usage, and six literal vectors are unchanged. Package versions do not
select identity behavior. A future semantic change must introduce a new identity
profile/domain and new vectors; neither expected digests nor existing versioned
transcripts may be rewritten to bless incompatible behavior. Existing document
identity and wire contracts are unchanged.

| Final command | Outcome |
| --- | --- |
| `cargo test --package neutral-test-suite project_identity` | 31 tests pass, including the independent encoder and public integration |
| `cargo test --package neutral-compiler cache` | Five cache tests pass, including collision/context rejection and exact/over retention bounds |
| `cargo xtask fixtures sync` / `cargo xtask fixtures check` | Oracle now covers V1-ID-001..005; literal vector bytes are unchanged |
| `cargo xtask ci pr` | 495 workspace tests pass, zero skips; strict Clippy, doc tests, contract/hygiene checks, command smoke, standalone probe build, and docs pass |

Release promotion confirmation, release-quality approval, and the next release
remain separate unchecked tasks. No version, tag, commit, or publication was changed.

## Standalone composition catalogue/value slice (06-10-2026)

This slice does not extend the frozen project identity profile. Eight literal
positive/negative/boundary/migration catalogue inputs and acceptance/error oracles
are registered with SHA-256 byte pins and runtime-owned copies. Supplied closed
values reuse the default/type/restriction validator, including heterogeneous
variant payloads. Independent materialization-node, work, depth, list, string-byte,
numeric-digit and scale limits reject without partial results. Optional absence,
explicit null, defaults and supplied occurrences remain separate safe origin
facts; equivalent materialized values compare equal without equating origins.

The compiler-independent reader exposes public materialization and bounded
reference-type paths through lists, nullable wrappers and every variant alternative.
Private/missing root types share a safe unavailable classification. These are type
dependencies and occurrence classifications, not source binding edges, source-map
spans, a serialized project, or independently verified new logical identities.

Validation: `cargo xtask test all` passes 638 Nextest tests (zero skips) plus the
compiler compile-fail doctest. `cargo xtask ci pr`, strict workspace Clippy,
`cargo xtask fixtures check` (59 fixture/oracle entries), and portable verification
pass. The vocabulary fuzz harness additionally exercises bounded supplied-value
probes on accepted catalogues and builds with `CCACHE_DISABLE=1`; no full fuzz
campaign or allocation-fault gate is claimed by this slice. The existing public
semantics hash was synchronized with the already accepted trailing-comma change.

Still required: full source variant/composition syntax and semantics, non-null
reference-value edges, complete project IR/provenance/source attribution, new codec
and independent probe/view integration, frozen new identity transcripts and literal
independent vectors, actual clean/cache equivalence, and complete hostile/allocation
validation. The full extension and promotion-conformance checklist remains open;
creating a package tag does not substitute for these requirements.

## Composition contract and literal registration (06-10-2026)

This closes only the two contract-freeze and fixture/vector-registration items.
The [schema/default/restriction contract](../../specs/contracts/VOCABULARY-COMPOSITION.md),
[shared variant rules](../../specs/contracts/VARIANTS.md), and
[composition profile](../../specs/contracts/COMPOSITION-PROFILE.md) are frozen
with exact file hashes. Explicit capture /2 features select successor project
IR/result/view/identity profiles; no package version or failed old parse selects
them. Compatibility, source diagnostic precedence/recovery, safe attribution,
public/embedded/reference closure and independent limit counting are specified.
The successor binary layout is a requirement, not a codec availability claim.

The manifest adds five frozen fixture records: four request families and one
identity corpus. Together they register 26 literal positive/negative/boundary/
migration cases with exact embedded source/bundle byte digests and transitive
locks. Cases cover both variant origins, imported variants, heterogeneous values,
composed list/nullable/reference types, default/absence behavior, source error
classifications, diamond/missing/extra/conflicting/private/cyclic dependencies,
embedded recursion, executable members, exact/one-over alternatives and edges,
and explicit old-schema adapter/relabel rejection. Catalogue outcomes execute
against the existing standalone validator; future source outcomes remain frozen
requirements, not passing compiler tests.

The new identity corpus freezes five complete captured/logical/derivation/project-
artifact/view-artifact transcript byte strings and SHA-256 digests, twelve
semantic/control variations and eight literal rejection vectors. Variations
include unused defaults, restrictions, exact fractional/sign/scale choices,
optional absence versus null, selected variants, heterogeneous list order,
visibility, nested list/nullable/reference contracts, canonical dependencies and
exact revisions. Roots/presentation/occurrence facts are excluded from complete
meaning. The test-only Python oracle extends only unchanged independent /1
primitives; a separate Rust length-prefix walker/hash check verifies the literal
frames/digests. Tests do not generate or bless expected outputs. /1 vectors are
unchanged and protected by their original byte digest.

All runtime copies live under the owning test crate, without portable-path reads.
The separate oracle also pins each copied fixture's file digest, preventing
runtime/plan drift after archival. Missing Python fails instead of skipping.
Independent exact-byte/frame boundaries, zero controls, cancellation, normalized
numbers, canonical ordering and invalid reference targets are tested.

Validation: `cargo xtask test all` and standard `cargo xtask ci pr` pass 646
Nextest tests, zero skipped, plus the compile-fail doctest. Strict Clippy,
formatting, repository link/ownership checks, portable verification and
`cargo xtask fixtures check` pass with 64 registered fixture/oracle pairs.
Contract, reference-oracle, vector, manifest and review hashes are retained in
`freeze.toml`; routine command logs remain ignored generated output.

Activation remains blocked on full source/compiler/project/IR/codec/independent
reader/probe/view implementation, production comparison to every new vector and
oracle, and hostile decoder/allocation-fault/fuzz/clean-cache validation. The
new suite remains `frozen`, not `required`. Before activation, verify its existing
pins and all production comparisons; synchronization is not permission to rewrite
accepted expectations. No version, release, tag, commit or push was performed.

## Composition capture and captured-identity integration (06-10-2026)

The compiler now exposes `CapturedCompositionProjectRequest` and
`capture_composition_project` as a distinct capture-only boundary. Exact envelope,
profile and sorted feature selection precede zero controls and cancellation.
Shared integrity/header checks preserve capture /1 behavior; successor capture
adds per-member/line cancellation, aggregate composition count/byte ceilings before
proportional catalogue work, and exact transitive schema/type/default/restriction
validation. Repeated identities with distinct module-local aliases share one lock;
duplicate aliases reject, and dependency bundles are never implicit source aliases.
No path, URL, host resolver, environment access, acquisition or partial publication
is introduced. Replay retains exact bytes, asserted digests, features and both policies.

The separate output type cannot enter `compile_project` or expose a public downcast
to capture /1. Successful capture checks headers and catalogue facts, **not source
variant/value semantics**. Raw complete catalogue facts remain available to their
capturing owner; `CompositionCatalogue::from_shared` independently exposes only its
existing public inspection/materialization APIs. Private/missing lookups remain
indistinguishable. Reference paths include every alternative and composed wrapper;
these are type dependencies, not resolved source binding edges or invented spans.

IR owns shared profile selectors and the bounded
`captured_composition_closure` implementation. `CompositionCapturedClosureIdentity`
is distinct from /1 captured identity. Common capture-field framing is reused with
explicit private profile/domain selection, leaving all old transcript bytes unchanged.
The frozen production comparison matches the captured baseline's exact 682 bytes,
23 frames and literal SHA-256. Exact/one-over limits, zeros, cancellation, unsupported
features, canonical capture order, changed source evidence and replay are tested.
This does not implement successor logical, derivation, interface or artifact identities.

Cross-package tests execute all 26 registered catalogue cases through capture, then
check diamond deduplication, exact dependencies, complete restrictions/defaults,
optional absence, legacy leaf adaptation, privacy and reference paths through the
compiler-independent reader. Catalogue-only cases use an explicit synthetic test
source declaring their reviewed roots; source-bearing cases preserve literal byte
pins. Source semantic expectations are deliberately not claimed as executed.
Capture duplicate/conflicting locks return `NEU-CAP-010` at integrity; standalone
catalogue validation returns `NEU-COM-005`. The full-project oracle's outcome remains
unchanged and pending comparison at the future project boundary. Existing /1 member
and schema failure precedence remains unchanged, with no retry through composition.

Full extension checkboxes remain open: both-origin source variants and composition
semantics, source reference-value resolution, successor complete IR/companions and
attribution, codec/probe/public views, remaining identity partitions, actual clean/cache
execution, hostile decoder/allocation-fault review and fuzz campaigns. The frozen suite
is not activated and no expected inputs, vectors, contract hashes or thresholds changed.

Validation: `cargo xtask test all` and `cargo xtask ci pr` pass 669 Nextest tests
with zero skips, plus both compile-fail doctests (no resolver on old capture;
no successor-capture downcast into old compilation). Strict workspace Clippy,
formatting, documentation generation and repository checks pass.
`cargo xtask portable verify` and `cargo xtask fixtures check` pass with all
64 fixture/oracle pairs and zero manifest/freeze updates. No new coverage,
mutation, full fuzz campaign or release qualification result is claimed.

## Shared resolved composition scope (06-10-2026)

`SourceCompositionDefinition` associates an exact source module-symbol owner with
the same `CompositionDefinition` record/variant body used by vocabulary declarations.
`validate_composition_scope` accepts already resolved source contracts and an
immutable validated vocabulary catalogue. It validates owner grammar/profile,
canonical definition/member/tag order, presence/default consistency, wrapper/ref
shapes, every nominal target, public/external closure and all embedded cycles.
Unselected alternatives and private unused declarations participate. Lists and
nullable wrappers do not hide embedding; nominal reference cycles remain legal.
Vocabulary definitions cannot acquire source owners through this API.

Default, restriction and closed supplied-value validation use one common engine
for both origins. Source-owned records can embed exact public vocabulary variants;
heterogeneous variant lists retain order and check each selected payload. Unknown
tags, incompatible payloads, incomplete records, invalid unused defaults and
contradictory presence fail before publication. Normalized finite choices and
inclusive exact numeric/Unicode-scalar/list-length checks reuse existing code.
Canonical materialized meaning remains separate from safe supplied/null/omitted/
defaulted origin paths; no source span or host information is invented.

The scope independently bounds source/vocabulary type/field/alternative/choice
totals, per-module types, wrapper/value depth, scalar retention, cumulative default
materialization visits and work. Raw defaults/types are preflighted before
recursive consumers or cloning retained restrictions. Existing capture remains
responsible for exact bytes, locks and dependency depth; this scope does not
recapture a previously accepted catalogue. Root occurrence depth starts at zero,
record/list/variant children add one, nullable checks spend visits without adding
depth, and nominal lookup does not double-charge depth. Standalone catalogue
materialization retains its previous depth policy and all its tests.

`CompositionTypeCatalogue` opens the validated scope without compiler linkage.
It enumerates only public source contracts, shares exact public vocabulary lookup,
materializes closed values through the common engine and inspects every declared
reference-type branch/wrapper. Missing/private source roots share the same lookup,
value and reference failures. Debug output contains counts only. Complete project
public-view closure, binding edges and source attribution are not implemented here.

This slice does **not** parse source declarations, check captured import/alias
visibility, resolve source value reuse/non-null references, compile successor
project IR, encode/decode `.nir`, derive public views or complete logical/derivation/
artifact identities. The remaining eight full extension gates stay open. Contract,
oracle, vector and manifest bytes/pins are unchanged; the suite remains frozen.

Validation: 24 new resolved-model tests and six compiler-independent reader tests
pass. `cargo xtask test all` and `cargo xtask ci pr` pass 699 Nextest tests,
zero skips, plus both compile-fail doctests. Strict workspace Clippy, formatting,
documentation generation, dependency and repository checks pass.
`cargo xtask portable verify` and `cargo xtask fixtures check` pass; all 64
fixture/oracle pairs retain their pins with zero manifest/freeze updates.
No coverage, mutation, full fuzz, allocation-fault, clean/cache or release gate
is newly claimed by these resolved-model tests.

## Resolved binding validation and reader inspection (07-10-2026)

`CompositionValue<R>` shares the structural scalar/list/record/variant model
between `ClosedValue` defaults and `BindingValue` resolved module-symbol references.
Closed references are uninhabited; a compile-fail doctest proves a binding target
cannot become a contract default. No frozen project or wire enum is extended.

`validate_composition_bindings` validates a whole resolved binding index before
publication. Canonical owner/signature checks precede value materialization;
targets must be present, invariantly typed and visible. Public values cannot
expose private targets. Forward and cyclic references remain identity-only,
without evaluation or embedding. The same default/restriction engine checks both
nominal origins and nested selected payload/list/nullable values. Budgets and
materialization visits span the entire request; repeated references pay for
type-key comparisons too. Stricter semantic policy rechecks dormant contracts
and unused defaults. Capture remains responsible for exact original bytes,
digest checks and captured-byte budgets; the resolved model cannot attest them.

Materialized meaning, supplied/null/absent/default origin paths and actual binding
reference occurrences are separate immutable facts. `CompositionBindingCatalogue`
exposes only public bindings and public compatible targets, sharing the independent
type catalogue without compiler linkage. Missing/private owners have the same safe
lookup failure; debug shows counts, not private names or data.

Eleven new vocabulary tests and two reader tests cover both origins, forward/cyclic
references, private/dangling/incompatible targets, default constraints, required
fields, omission versus null, exact numeric choices, Unicode-scalar/list-length
boundaries, root-zero depth and cumulative visit boundaries, cancellation,
permutation and concurrent isolation. Runtime inputs use existing crate-owned
literal vocabulary fixtures; no portable-path reads or expected-input rewrites
are introduced.

Standard `cargo xtask ci pr` passes 712 Nextest tests (zero skips), three compile-fail
doctests, strict workspace Clippy, formatting, dependency/hygiene checks, standalone
probe build and generated docs. This is not new coverage, mutation, fuzz,
allocation-fault, clean/cache or release evidence.
`cargo xtask test all`, `cargo xtask portable verify` and `cargo xtask fixtures
check` also pass; all 64 fixture/oracle pairs keep their original pins, with
zero manifest or freeze updates.

At this resolved-model slice, the two full integration gates remained unchecked: `.neu` source parsing,
import/reuse resolution and original-byte attribution are not established by a
resolved binding index. Complete successor project IR/companions, independent
encoded-artifact validation, probe/root views and the remaining /2 identity layers
still require implementation. Registered contracts, manifests, oracles and identity
vectors remain unchanged; the new suite stays frozen, not activated.

## Composition source IR reader and wire integration (07-10-2026)

The explicit library pipeline now implements `capture_composition_project` →
`compile_composition_project` → `ValidatedCompositionProject::from_ir` →
`encode_composition_project` / `decode_composition_project`. The old capture,
compiler, project schema, codec and identity selectors are unchanged; there is
no successor-to-legacy downcast or decode fallback.

The request-local compiler resolves source imports and both source/vocabulary
nominal origins before contextual values. Source records and variants share
the common contract model; final member commas are optional. Closed defaults,
finite choices, exact numeric ranges and Unicode-scalar/string/list-length
restrictions use the same bounded scope/binding engine as independent consumers.
Ordinary reuse materializes immutable values and rejects evaluation cycles;
typed references remain non-embedding and may cycle. Reuse and contract copies
are charged before allocation, preventing unchecked repeated expansion.

`CompositionProjectIr` retains complete private/public/disconnected declarations,
canonical catalogues/dependencies, materialized values, source/vocabulary evidence,
source maps, reuse/type/reference provenance, origin paths, independent policy
and retained resource counts. Iterative raw inspection precedes recursive copies.
The reader intersects producer/caller/hard controls, independently validates
catalogues and source signatures, reconstructs supplied/default/omitted states,
rematerializes values, and rejects stale resource/interface/companion facts.
Original declaration spans are checked; fine-grained origin attribution remains
explicitly unavailable rather than fabricated. Complete-data access is not a
redacted public view.

The separate `NIR-PROJECT-CBOR/2` codec implements the frozen sixteen-position
envelope and restricted tuple grammar. It rejects unknown tags, wrong arity,
nonminimal arguments, truncation, trailing bytes, invalid categories and consumer
bounds. Minimal-width enforcement is explicitly selected for the successor;
legacy lexical acceptance remains unchanged. Transcript-framing bounds are
independent of encoded artifact size, so an exact artifact-byte budget is valid.

Production complete logical /2 matches the frozen independent transcript exactly:
5,551 bytes, 278 frames, digest
`c01399c882bc6342db64023cacee743fc47e4985773d1967308a9d1239e8e8e7`.
The reader recomputes the separate interface /2 identity. Frozen contracts,
fixture/oracle inputs and vectors are not rewritten to match production.

Fourteen new tests (thirteen [pipeline tests](../../../crates/neutral-test-suite/tests/composition_contract/pipeline.rs)
and one [lexical-profile regression](../../../crates/neutral-encoding/tests/decoder/mod.rs))
cover accepted literal source families, registered source-negative codes,
boundary/migration captures, nested defaults, optional final commas, both variant
origins, heterogeneous lists, reuse/reference cycles, invalid payload/defaults,
independent byte/count controls, every truncated artifact prefix, wire tag/arity/
minimal-width defects, stale facts, ordering and concurrent isolation, plus bounded
exponentially expanding reuse without cross-request contamination.
`cargo xtask test all` passes 726 Nextest tests, zero skips and three compile-fail
doctests. No new coverage, mutation, fuzz, allocation-fault or release approval
is claimed by this functional integration.

`cargo xtask ci pr` also passes formatting, strict workspace Clippy, all tests,
dependency/hygiene/traceability checks, legacy probe smoke/build and generated
documentation. `cargo xtask portable verify` and `cargo xtask fixtures check`
pass: all 64 fixture/oracle pairs keep their reviewed hashes, with zero manifest
or freeze updates. The probe build checks existing behavior, not successor
standalone probe activation.

The compiler/IR/encoding/reader implementation checkbox is now complete.
Standalone successor probe, public views/redaction, fine-grained attribution,
derivation/artifact identity partitions, all adversarial identity variations,
complete hostile/fault/fuzz campaigns and actual clean/cache execution remain
open. The registered suite stays frozen; full extension activation and promotion
are not claimed.

## Composition public views probe and attribution (07-10-2026)

The successor standalone probe now detects the explicit binary frame, independently
validates the complete project and derives a public-only reader view. Both source
and vocabulary contracts retain all fields, variant alternatives, restrictions,
defaults, reference types and actual public reference targets; exact transitive
vocabulary locks remain in the interpretation closure. Binding/type worklists are
iterative, including cyclic identity-reference chains. Selection does not mutate
complete IR or its complete logical digest.

The compiler traces original-byte initializer, nested default and ordinary reuse
occurrences. The reader checks attribution against retained source maps, actual
source-default contract owners, validated reuse edges and canonical vocabulary
default identity/revision/type/field owners. Vocabulary byte spans are not invented;
unavailable evidence remains absent. In particular, already materialized vocabulary
defaults copied through ordinary reuse do not get falsely relabelled as newly
defaulted occurrences. These companion checks are not producer authentication or
proof of faithful compilation from original bytes.

Public views remove source IDs/spans, private definitions and ordinary reuse
implementation provenance. Safe public vocabulary default owners remain without
captured-byte spans. The actual standalone executable accepts successor artifacts,
supports public roots and pretty JSON, and produces no partial success for private
selection or truncated input. Inspection text is presentation, not JSON IR transport.

Five [consumer regressions](../../../crates/neutral-test-suite/tests/composition_contract/reader_probe.rs)
passed, including a 96-binding cyclic reference closure, precise `42` default/reuse
spans, hostile attribution, empty/root-invariant selections and the compiler-free
binary. Full validation at this slice passed 731 Nextest tests, zero skips, three
compile-fail doctests and `cargo xtask ci pr`. The reader/probe checklist gate is
complete; this does not activate the whole extension or its later quality gates.

## Composition hostile boundaries and actual cache execution (07-10-2026)

`CompositionCompilationCache` is an explicit caller-owned successor syntax cache,
separate from the old-profile cache. Exact bytes/module/features are required;
digests are fast filters, not sufficient authority. Every request reruns graph,
catalogue/default, semantic/value, identity and companion construction under current
controls. Retention is bounded and generation publication is atomic: cancelled or
failed runs leave the previous successful generation intact.

Six [cross-package hardening tests](../../../crates/neutral-test-suite/tests/composition_contract/hardening.rs)
compare complete IR, companions and encoded bytes under real cold/warm hits,
changed/removed units, host-ID remapping, changed locked vocabulary defaults,
lowered controls, failed/cancelled generations, tiny retention and concurrent
caller-owned caches. Three [compiler-local tests](../../../crates/neutral-compiler/tests/project_cache/composition.rs)
add digest-collision simulation, misplaced module/feature context and cancellation
arriving after real parsing. This is actual incremental execution, not capture replay.

Each of the fifteen independent composition consumer controls and seven CBOR
consumer controls is swept at its acceptance threshold and one below/above in
complete decoding; the composition sweep must agree with independent in-process
reader validation. These implementation boundary sweeps supplement, not replace,
literal catalogue/dependency boundary expectations. Every byte of a reviewed
successor artifact is changed by three masks; any accepted result must independently
revalidate and re-encode. No frozen contract, oracle or identity vector is rewritten.

Source, IR and probe fuzz targets now include the successor pipeline, with bounded
explicit capture and valid successor seed mutation; vocabulary fuzzing continues
strict schema/default/restriction checking. All five targets completed their
configured 60-second runs outside the restricted sandbox, without disabling leak
checking: source 367,126 executions, vocabulary 725,962, IR 69,903, formatter
2,058,541, probe 104,292. These are raw sanitizer completions, not a release
approval: concurrent test/documentation edits correctly invalidated that measurement
receipt, requiring a fixed-input rerun. Only the automatic validated receipt, not
this raw execution table, establishes a final source-bound campaign pass. The first
sandboxed source attempt instead failed LeakSanitizer shutdown; it is an environment
failure, not a confirmed source defect. Raw reports remain ignored under generated
quality-gate output, never committed or included in immutable portable fixtures.

Full validation passes 740 Nextest tests, zero skips, three compile-fail doctests,
strict workspace Clippy and `cargo xtask ci pr`. Portable verification and all 64
fixture/oracle pairs pass without manifest/freeze updates. The broad
hardening checkbox remains open until allocation-fault/fallible-copy review,
phase-complete cancellation/fault injection, all producer/JSON retention boundaries
and transitive multi-bundle adversarial campaign coverage are completed.

## Composition phase faults producer boundaries and transitive fuzzing (07-10-2026)

Ten private compiler checkpoints cover graph construction, parsing, signatures,
initial/default/final scope, values, bindings, assembly and final publication.
Cancellation and simulated processing faults at each checkpoint preserve the
previous successful cache generation; the next unchanged request still reuses
its syntax and matches clean compilation. These are deterministic processing
faults, not real allocator exhaustion or proof of every subsystem's fault paths.

The hardening suite independently sweeps all fifteen producer composition controls
and thirteen JSON/scalar/shape controls at their acceptance threshold and one
below/above. Successful producer results must pass independent reader and wire
validation. These implementation sweeps supplement the frozen literal boundary
oracles; positive-only controls whose first accepted value is one also check
invalid zero policy, not a fabricated larger workload boundary.

Occurrence field names and retained origin paths now reserve storage fallibly.
Private reservation callbacks test failure at each vector/string copy boundary
without changing a global allocator. Exact Unicode bytes and all path segment
kinds are preserved. Boxes, ordered-map insertion and other recursive clones remain
subject to separate allocation review; no system-wide allocator-safety claim is made.

The vocabulary fuzz target now mutates one member of an exactly locked four-bundle
diamond and drives source compilation, complete IR, independent wire validation
and public probe closure. Unmodified reviewed seeds must capture and encode, so a
silently rejected seed cannot count as coverage. A sanitizer smoke run completed
8,131 executions with the seed assertions enabled. This single topology does not
establish exhaustive transitive graph, cancellation or allocation-fault coverage.
The final configured campaign is run only after edits stop; its automatically
validated source-bound receipt, rather than this narrative, is measurement authority.
Generated reports and mutable corpora remain ignored.

`cargo xtask test all` passes 745 Nextest tests with zero skips and three compile-fail
doctests. `cargo xtask ci pr`, strict isolated fuzz-package Clippy, portable
verification and all 64 fixture/oracle pairs pass without manifest/freeze updates.
The broad hardening checkbox remains open for the explicitly listed
allocation, non-compiler fault, capture/retention and broader topology gaps.

## Fallible successor ownership and graph indexes (08-10-2026)

With explicit maintainer approval, successor public compiler/reader/encoding
ownership now uses `neutral_core::allocation::Shared<T>` and its fallible
constructor. `CompositionCatalogue::new` returns an allocation-aware result.
Old-profile public APIs retain their standard shared-owner types. Private captured
source storage moves owned strings/bytes into fallible shared headers instead of
allocating slice conversions; cache source retention shares those existing bytes.
Syntax, wire bytes and frozen identity/vector inputs are unchanged.

The core allocation boundary exposes reviewed `trybox::or_drop` and std-only
`triomphe::Arc::try_new` through safe wrappers. Recursive owned contract/type/value
copies and materialization boxes propagate allocation failure. Request-local
scope/dependency indexes use fallibly reserved sorted vectors rather than
infallible tree insertion. Their insertion/removal shifts are charged to the
aggregate work policy. Graph worklists reserve before growth; canonical sorting
uses nonallocating unstable sorting. Selected dependency source/unsafe review is
recorded in the [dependency review](../../../quality/reviews/dependency-review.md).

Two JSON regressions interrupt every reservation checkpoint with synthetic faults
or mid-parse cancellation and verify a subsequent unchanged parse succeeds.
UTF-8 raw/escaped scalar limits reject before retaining an over-limit character.
Core tests exercise constituent-copy failure, index semantics and cross-thread
shared ownership with exactly one final payload destruction. These are local
failure/control-flow tests, not actual system-wide allocator exhaustion.

Three generated graph tests supplement reviewed literal fixtures: chains, wide
fanout, diamonds and unequal reconvergent paths have identical complete wire
output under reversed capture and concurrent schedules. Bundle/edge/fanout/longest
depth controls are independently exact/one-over, including shared-leaf memo hits.
Missing/extra/divergent/private/cyclic dependencies fail before capture publication.
Runtime locks are recomputed after intentional test mutations; frozen inputs and
oracles are never rewritten.

Validation: 755 Nextest tests pass with zero skips and three compile-fail doctests;
strict workspace and isolated fuzz-package Clippy pass. Portable verification and
all 64 fixture/oracle pairs pass with no manifest/freeze changes. The configured
five-target sanitizer campaign is rerun on fixed final inputs; its validated
generated receipt is the authority for that run, not this narrative.

The broad checkbox remains open. Schema decoding still has infallible string,
box and collection construction; source parsing/cache AST copies and retained
companions also need review. Shared-owner migration is necessary but does not
establish no allocator abort throughout the pipeline. Fault coverage outside the
exercised checkpoints and remaining capture/retention bounds are not declared
complete.

## Decoder parser/cache and companion retention follow-up (08-10-2026)

Successor schema decoding no longer retains strings, boxes, dependency/member
sets or outer collections through infallible growth. Request-local helpers check
cancellation/work before reservation and cancellation after copying. Private
fault policy is never exposed through public input; raw contracts are not returned
on failure. Nested literal contracts exercise every schema retention checkpoint
with synthetic allocation errors and actual token cancellation. Independent
count/depth errors precede proportional retention. Legacy leaf adaptation is
fallible, but its old-schema validator remains a separate unfinished audit path.

The shared CBOR lexical parser now reserves vector growth fallibly and checks
remaining node/count/input bounds first. Geometric growth avoids quadratic
single-element reallocation. Nested array/map/text/byte tests interrupt every
lexical reservation; subsequent fresh decoding still succeeds. Shared wire schema
helpers reserve list/text/type ownership fallibly. Successor vocabulary joining
is a linear ordered merge over borrowed owner keys, not an allocating tree or
cloned ownership tuple. Public-name emission and numeric-bound emission borrow
existing facts rather than making temporary owned copies. Wire acceptance,
canonical encoding and frozen identity inputs are unchanged.

The lexer retains token/trivia/text storage fallibly and observes successor
cancellation inside strings/comments. UTF-8 scalar length is checked before adding
raw or escaped characters. Type parser wrappers retain recursive syntax fallibly;
successor callers distinguish allocation failure from malformed grammar. Exact
number parsing reserves coefficient storage fallibly and normalizes in place.
Successor cache generations copy syntax with fallible constituent ownership, share
feature/source bytes and reserve pending membership fallibly. Four warm-generation
retention transitions have request-local fault tests proving prior cache reuse
survives failure. Exact original-byte retention is tested below/at/above the bound.
Reader declaration and origin reconstruction copies are fallible and cancellable.

The expanded full suite passes 761 Nextest tests with zero skips and three
compile-fail doctests. CI, isolated fuzz-package Clippy, portable/fixture validation
and the configured sanitizer campaign are rerun after final edits; only their
actual successful command results and source-bound receipt establish those passes.

The full hardening gate is still open: producer semantic resolver/graph/companion
assembly, old-schema validation, remaining reader/view/identity indexes and copies,
and fault/boundary paths outside those exercised above are not yet allocation-safe.
These checkpoint tests establish fail-closed local control flow, not actual global
allocator exhaustion. No frozen contract, manifest, oracle or identity vector was
changed to conceal unfinished work.

## Maintainer scope decision (08-10-2026)

Standalone legacy validation/allocation review is excluded from the remaining
composition hardening work at the maintainer's request. Earlier audit notes record
the findings at that time, not a continuing requirement to harden old-profile-only
entry points. Existing compatibility tests and accepted legacy behavior are retained;
this decision does not delete validators or change wire/profile acceptance.

The successor still accepts explicitly adapted legacy leaf contracts. Any shared
validator reached through that successor path remains subject to its frozen
fail-closed allocation contract. Excluding standalone legacy work cannot establish
successor allocation safety by omission. Resolver/graph/companion assembly and
remaining consumer/view/identity retention work keep the broad gate unchecked.

## Resolver, graph, companions, views and identity retention (08-10-2026)

Successor resolver root/type/value/active indexes now use fallible ordered storage
with charged insertion/removal movement. Source names borrow token text rather
than retaining temporary qualifiers. Root/type/body/value/default copies, wrapper
boxes and declaration/origin/provenance retention are fallible. Root copies and
nominal body copies check their traversal work before copying. Final provenance
is moved into complete IR, not cloned. Module, source and vocabulary companions
use explicit fallible vector/text retention; type-edge enumeration does not build
temporary type lists.

The private successor graph shares the exact module/import scanner but avoids
constructing old-profile public `Arc` projections. Its import topology and
iterative forward/reverse SCC worklists are fallibly reserved, work-charged and
cancellable. The scanner retains names, aliases and imports fallibly; forbidden
import classification borrows bytes instead of allocating a temporary prefix.
Tests cover exact/one-over SCC sizes, exact traversal work, pre-cancellation and a
10,000-module acyclic chain. Cross-package source chain/diamond/reference-type
cycle tests compare topology with the existing graph API and validate complete
wire/probe output under reversed capture order.

Reader companion, source-attribution, reachability and ordinary-reuse indexes
use fallible retention and charged shifts. Resource sums stream over facts without
temporary vectors. Views use fallible signature/value/reference copies, redact
source evidence while retaining origins, and avoid allocating temporary nominal
lookup keys. Identity framing borrows exact numeric restrictions and uses borrowed
canonical declaration lookup; unique semantic edges are retained in a fallibly
reserved vector, sorted and deduplicated without changing transcript bytes.
Capture alias indexes are shared fallibly rather than cloned with the capture.

Validation: 767 Nextest tests passed without skips, including independent decoding,
probe, public-view privacy and frozen identity comparisons; all three compile-fail
doctests passed. Strict workspace and isolated fuzz-package Clippy pass. The final
portable/fixture checks and configured five-target sanitizer campaign are rerun
against fixed inputs; their actual command results and validated source-bound
receipt, not this prose, establish completion of those measurements. The final
graph scan also intersects the successor decoded-string bound before lexical
retention; a raw multibyte exact/one-over test guards this pre-resolution check.
Separate all-unit scanning and edge validation preserve syntax-first diagnostic
precedence and diagnostic-overflow classification; regressions compare both the
returned code and original-byte location with the existing graph boundary.

The requested resolver/graph/companion/view/identity retention migration is
implemented. The broad hardening checkbox is deliberately still open: shared
source/header/lock capture validators and explicitly adapted old-schema leaf
validation still contain successor-reachable infallible retention. Synthetic
phase/lexical/cache checkpoints do not establish complete allocator-exhaustion
coverage for every newly migrated boundary. Standalone legacy entry-point review
remains excluded; these findings concern actual successor dependencies. Frozen
contracts, fixture manifests, oracles and identity vectors are unchanged.
