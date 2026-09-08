<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral language v0 development plan

Status: active v0 operational index — Stage 9 approved; Stage 10 workflow
overhaul active

This is the entry point for implementing and tracking Neutral language v0 in
this repository. The directory is a self-contained, version-scoped package of
architecture, requirements, contracts, decisions, fixtures, executable
conformance assets, implementation gates, and retained evidence. Archive it
when v0 closes; initialize the next active `portable/` package from that
version's Neutral roadmap package, then adapt its progress and repository
evidence locally.

## Governing specification

Read in this order:

1. [Neutral v0 architecture](ARCHITECTURE.md)
2. [Neutral v0 requirements](specs/REQUIREMENTS.md)
3. [Architectural choices](specs/contracts/choices.md)
4. [Master syntax checklist](specs/contracts/syntax.md)
5. [Accepted v0 decisions](specs/decisions/README.md)
6. [Proposed syntax guide](specs/contracts/proposed-syntax-guide.md)
7. [Conformance fixtures](specs/fixtures/README.md)
8. [Implementation roadmap](ROADMAP.md)

More specific accepted decisions govern. Development and test documents order
and verify work; they cannot silently extend source syntax, semantics, IR, APIs,
or vocabulary behavior.

## Operational documents

| Document | Authority and purpose |
| --- | --- |
| [Contract freeze](development/02-CONTRACT-FREEZE.md) | Mandatory gate between scaffolding and production compiler behavior |
| [Implementation stages](development/03-IMPLEMENTATION-STAGES.md) | Stage and vertical-slice execution order |
| [Identity and vocabulary](development/01-IDENTITY-AND-VOCABULARY.md) | Exact byte digests, semantic fingerprints, identities, strict vocabulary JSON encoding |
| [Testing and quality](development/04-TESTING.md) | TDD, test ownership, suite activation, levels, evidence, quality and ISO alignment |
| [Environment and automation](development/00-ENVIRONMENT-AUTOMATION.md) | Bootstrap, toolchains, containers, `xtask`, CI, offline execution, evidence paths |
| [Release qualification](development/05-RELEASE.md) | Stage 10 artifacts, release matrix, audits, evidence and approvals |

Each concern has one owning document. Cross-links replace duplicated checklists.
If two operational documents conflict, stop work and resolve them before closing
the affected gate.

## Non-negotiable delivery rules

- [ ] Freeze normative contracts before production compiler behavior.
- [ ] Implement fixture → syntax → semantics → IR → reader → probe as one
      vertical slice for every source feature.
- [ ] Write the failing test first and retain red-phase evidence.
- [ ] Keep lexer, layout, parser/recovery, symbol/type, semantic, and lowering
      models private.
- [ ] Emit authoritative IR only after complete success.
- [ ] Keep `compile_captured` deterministic and I/O-free.
- [x] Treat source, vocabulary, and encoded IR bytes as untrusted even after a
      digest matches.
- [ ] Enforce structural limits before proportional allocation or conversion.
- [x] Keep logical IR equality independent of map order, thread scheduling,
      pretty printing, encoding bytes, and graph-local `ElementId` spelling.
- [x] Keep ordinary reuse/default origin in provenance, not new logical value
      kinds.
- [x] Give `Ref<T>` identity-only meaning; infer no ownership, containment,
      dependency, readiness, order, or runtime behavior.
- [ ] Add no excluded syntax through parser convenience, private IR, vocabulary,
      CLI, formatter, or probe conventions.
- [ ] Keep the standalone generic probe free of compiler/private-model
      dependencies.
- [ ] Keep temporary milestone checks outside stable diagnostics and normative
      conformance.
- [ ] Never convert flaky, retried, skipped, missing, or indeterminate required
      evidence into a passing release gate.

## Critical path

```text
Stage 1: implementation foundation
    ↓
Mandatory normative contract freeze
    ↓
Stage 2: minimal scalar end-to-end core
    ↓
Stage 3: source text and scalar vertical slices
    ↓
Stage 4: record, default, and list vertical slices
    ↓
Stage 5: reuse, reference, and graph-identity vertical slices
    ↓
Stage 6: captured vocabulary vertical boundary
    ↓
Stage 7: external Neutral IR encoding and hostile reader
    ↓
Stage 8: formatter, CLI, standalone probe, documentation
    ↓
Stage 9: correctness, security, determinism, performance hardening
    ↓
Stage 10: workflow overhaul and release qualification
```

## Current gate status

Stage 1, the mandatory normative contract freeze, Stage 2, all Stage 3 slices,
all Stage 4 slices, all Stage 5 slices, all Stage 6 slices, all three Stage 7
steps and validation, and all Stage 8 steps and validation are complete. The
private frontend
retains exact nonsemantic trivia, enforces frozen ASCII/token boundaries,
decodes bounded Unicode strings, and normalizes signed decimal exact numbers
without floating-point conversion. It supports nullable scalar values with
explicit typed null and collects one root scope before resolving nominal record
schemas and contextual values. It validates closed scalar/null/record field
defaults and materializes omitted fields. It also contextually types invariant
ordered lists, including
empty, nested, nullable-element, and closed-default forms, under explicit item,
depth, and traversal limits. Ordinary immutable values now resolve after root
collection through a deterministic dependency graph, including forward,
transitive, nested, and outer-nullable reuse; cycles fail with stable related
locations, while final values and reuse edges cross IR, reader, and probe
boundaries. Typed identity references now resolve exact binding targets to
document-local `ElementId` edges without entering value dependencies; only
`Ref<T>` breaks nominal embedding cycles, and reader/probe consumers validate
and traverse those typed edges by ID. Whole logical documents now compare by a
one-to-one graph mapping independent of local ID spelling and companion records;
duplicate and dangling graphs fail closed. Typed IR, source facts, provenance,
reader validation, and probe output are covered by frozen oracles and generated
property vectors. Captured vocabulary bundles now pass an exact typed-digest
gate before a strict bounded JSON decoder validates the closed schema, immutable
feature set, nominal type graph, and closed defaults. Captured byte identity is
kept separate from the normalized logical vocabulary contract, and no bundle
content can trigger code loading or external I/O. Optional `use` and qualified
vocabulary types now resolve only from exact host-captured lock input. Closed
payloads, vocabulary defaults, exact IR/derivation facts, reader validation,
and generic probe enumeration form the complete Stage 6 vertical boundary.
[Neutral IR Framed CBOR 0.1](specs/decisions/11-external-ir-encoding.md) is now
the accepted external artifact format: a checked fixed-width frame encloses
closed, restricted-CBOR envelope, logical payload, source-map, provenance, and
derivation sections. Exact decimals remain normalized string/integer
components; unknown versions, capabilities, members, and malformed or oversized
input fail closed before a reader view exists. The validated-document encoder
now derives exact capabilities, projects every logical and companion contract
into five bounded restricted-CBOR sections, hashes sections 2 through 5 into an
envelope-only integrity list, and emits the fixed frame. Producer/build facts
affect only the envelope; deterministic emitted byte order is nonsemantic
implementation behavior. The hostile decoder now validates the fixed frame and
directory before CBOR allocation, retains duplicate map entries until closed
schema checks finish, applies host and captured bounds, verifies integrity and
capabilities, reconstructs every logical and companion contract, and returns a
reader view only after complete cross-section and trusted-reader validation.
Stable failure classes cover malformed, unsupported, oversized, inconsistent,
and cancelled inputs; captured vocabulary identity is checked without external
lookup. The standalone `neutral-probe` binary now reads external encoded
artifacts through public encoding/reader contracts without compiler linkage,
and its process-boundary system test verifies deterministic categorized output.
The stable decoder campaign covers all truncation boundaries, seeded structured
mutations, and arbitrary byte sequences. A reviewed bounds-before-allocation
record closes Stage 7. The Stage 8 reference formatter now renders validated
captured source through the private parsed representation without publishing an
AST. It enforces canonical headers, LF newlines, four-space recursive layout,
field/item commas, normalized spacing, and deterministic source-order comment
placement. Complete-corpus tests prove idempotence, parse/format/parse logical
equivalence, provenance-category preservation, and source-identity separation.
The host CLI now exposes strict `compile`, `validate`, and `format` process
boundaries with explicit source and destination selection, captured-vocabulary
locks, reviewed structural limits, standard streams, overwrite authorization,
cooperative cancellation, safe diagnostic disclosure, stable exit classes, and
synchronized same-directory atomic output. Built-binary system tests cover
Unicode and spaced paths, stdin/stdout separation, permissions, broken pipes,
failed commits, cancellation, output preservation, temporary cleanup, and
external artifact decoding. Inspection remains a separate compiler-free
`neutral-probe` responsibility. That probe now enumerates logical, identity,
source-map, derivation, resource, type, value, vocabulary, and provenance views;
its shared renderer produces byte-for-byte-equivalent in-process and executable
observations. A consumer-owned diagnostic maps through the public source map,
hostile external traversal remains bounded by decoder limits, and CI enforces
the compiler-free dependency closure. The complete requirement/syntax evidence
index now maps accepted contracts to decisions, fixtures, implementation, and
tests. CI rejects missing IDs, unchecked master syntax, orphaned normative
fixtures/oracles, and broken registered paths; the published full-language
example compiles as conformance evidence. Stage 9 hardening now has complete
stable property, structural-limit, cancellation, isolation, dependency, static
review, critical and broad mutation evidence, whole-workspace LLVM coverage,
coverage-guided fuzzing, controlled performance, extended soak, and allocation
evidence. The sole maintainer approved its residual-risk treatment before
Stage 10 began.
The approved
[freeze manifest](specs/contracts/freeze.toml) identifies
the v0 contract family, and the
[contract question ledger](development/evidence/contract-question-ledger.md) records the
resolutions accepted during review.

## Stage summary and exit evidence

| Stage | Demonstrable outcome | Owning gate |
| --- | --- | --- |
| 1 | Reproducible workspace, automation, package/effect boundaries, and only feasible foundation tests | [Implementation Stage 1](development/03-IMPLEMENTATION-STAGES.md#stage-1-initialize-the-implementation-foundation) |
| Freeze | Grammar, semantics, IR, identity, vocabulary, APIs, diagnostics, limits, fixtures, and traceability accepted | [Contract freeze](development/02-CONTRACT-FREEZE.md) |
| 2 | Minimal `.neu` scalar compiles through validated IR, reader, and independent probe library | [Implementation Stage 2](development/03-IMPLEMENTATION-STAGES.md#stage-2-implement-the-minimal-atomic-core) |
| 3 | Full source-text and scalar behavior complete end to end; no future constructs parsed early | [Implementation Stage 3](development/03-IMPLEMENTATION-STAGES.md#stage-3-complete-source-text-and-scalar-vertical-slices) |
| 4 | Records, defaults, nullability, and lists complete as separate vertical slices | [Implementation Stage 4](development/03-IMPLEMENTATION-STAGES.md#stage-4-implement-records-defaults-and-lists-as-vertical-slices) |
| 5 | Reuse, cycles, typed references, recursion boundary, and alpha-equivalence complete | [Implementation Stage 5](development/03-IMPLEMENTATION-STAGES.md#stage-5-implement-reuse-and-references-as-vertical-slices) |
| 6 | Strict captured vocabulary bytes and qualified source values pass compiler/reader/probe | [Implementation Stage 6](development/03-IMPLEMENTATION-STAGES.md#stage-6-implement-captured-vocabulary-as-one-vertical-boundary) |
| 7 | Valid encoded IR round-trips logically; hostile IR fails boundedly | [Implementation Stage 7](development/03-IMPLEMENTATION-STAGES.md#stage-7-implement-one-external-neutral-ir-encoding) |
| 8 | Formatter, host CLI, standalone probe, docs, and traceability complete | [Implementation Stage 8](development/03-IMPLEMENTATION-STAGES.md#stage-8-complete-formatter-cli-standalone-probe-and-traceability) |
| 9 | Property, fuzz, limits, security, performance, soak, mutation, and quality gates pass | [Implementation Stage 9](development/03-IMPLEMENTATION-STAGES.md#stage-9-harden-correctness-security-and-performance) |
| 10 | Complete artifacts and retained release evidence approved | [Release qualification](development/05-RELEASE.md) |

Do not advance because a happy path works. Close the active stage's invalid
cases, public-reader behavior, source maps, provenance, derivation, limits,
diagnostics, tests, and evidence first.

## Implementation package boundaries

The initial Rust workspace follows responsibility boundaries, not ecosystem
layers:

```text
neutral-core
neutral-ir
neutral-vocabulary
neutral-compiler
neutral-reader
neutral-encoding      # validated-document external artifact encoder
neutral-probe          # reader-only library + standalone binary
neutral-cli            # compile/validate/format host
neutral-test-support   # helpers only
neutral-test-suite     # cross-package executable tests
neutral-bench          # benchmark harnesses/corpora
xtask                  # development and CI automation
```

The exact split may be consolidated before Stage 1 closes if public/private,
effect/pure, reader/probe, and test-ownership boundaries remain enforceable.

## Standard local gates

After host bootstrap, run from this implementation repository root:

```bash
cargo xtask environment verify
cargo xtask fmt
cargo xtask lint
cargo xtask check
cargo xtask test all
cargo xtask quality
cargo xtask build --profile release
cargo xtask validate binaries
```

Release operators use `cargo xtask release prepare`; it fails unless the
approved candidate identity, distribution scope, evidence, tag target, and
checkout agree. It never pushes, uploads, publishes, or creates a tag.

Generate the workspace API site with `cargo docs`, then open the generated
`target/doc/index.html`. The automation runs rustdoc
and builds the landing page from Cargo metadata, so package names, versions,
descriptions, ownership, publication groups, and workspace dependency links
stay synchronized without a hand-maintained crate list. All generated site
files remain ignored.

Individual tasks and evidence behavior are defined in
[00-ENVIRONMENT-AUTOMATION.md](development/00-ENVIRONMENT-AUTOMATION.md). Test
activation and quality gates are defined in
[04-TESTING.md](development/04-TESTING.md).

## Change control

- [ ] Every implementation change links frozen `NL-*`/`SYN-*` requirements,
      tests, and expected evidence.
- [ ] A specification defect changes the governing contract and freeze manifest
      through review; tests do not redefine behavior silently.
- [ ] Digest/transcript or encoding changes create new version identifiers.
- [ ] Stage activation changes in the same reviewed commit as complete slice
      implementation and evidence.
- [ ] Deferred features remain outside v0 unless an independent accepted proposal
      revises the scope.

## Deliberately postponed work

v0 does not add namespaces, visibility, multiple source units, imports, secret
references, static/member access, maps, sets, tuples, unions, enums, user
generics, operators, functions, control structures, mutation, override,
composition, templates, macros, executable plugins, external effects, public IR
transformation/migration APIs, canonical external encoding, GUI contracts, or
application-specific behavior.
