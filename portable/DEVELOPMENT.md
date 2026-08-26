# Neutral language v0 development plan

Status: proposed operational index

This is the entry point for implementing Neutral language v0. This directory is
a portable, standalone repository seed: copy its *contents* into the root of a
new `neutral-lang` repository. It turns the v0 architecture, requirements,
decisions, fixtures, and roadmap into gated work without adding language
behavior.

## Governing specification

Read in this order:

1. [Neutral v0 architecture](spec/v0/architecture.md)
2. [Neutral v0 requirements](spec/v0/requirements.md)
3. [Architectural choices](spec/v0/choices.md)
4. [Master syntax checklist](spec/v0/syntax.md)
5. [Accepted v0 decisions](spec/v0/decisions/README.md)
6. [Proposed syntax guide](spec/v0/proposed-syntax-guide.md)
7. [Conformance fixtures](spec/v0/fixtures/README.md)
8. [Implementation roadmap](ROADMAP.md)

More specific accepted decisions govern. Development and test documents order
and verify work; they cannot silently extend source syntax, semantics, IR, APIs,
or vocabulary behavior.

## Operational documents

| Document | Authority and purpose |
| --- | --- |
| [Contract freeze](development/CONTRACT-FREEZE.md) | Mandatory gate between scaffolding and production compiler behavior |
| [Implementation stages](development/IMPLEMENTATION-STAGES.md) | Stage and vertical-slice execution order |
| [Identity and vocabulary](development/IDENTITY-AND-VOCABULARY.md) | Exact byte digests, semantic fingerprints, identities, strict vocabulary JSON encoding |
| [Testing and quality](development/TESTING.md) | TDD, test ownership, suite activation, levels, evidence, quality and ISO alignment |
| [Environment and automation](development/ENVIRONMENT-AUTOMATION.md) | Bootstrap, toolchains, containers, `xtask`, CI, offline execution, evidence paths |
| [Release qualification](development/RELEASE.md) | Stage 10 artifacts, release matrix, audits, evidence and approvals |

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
- [ ] Treat source, vocabulary, and encoded IR bytes as untrusted even after a
      digest matches.
- [ ] Enforce structural limits before proportional allocation or conversion.
- [ ] Keep logical IR equality independent of map order, thread scheduling,
      pretty printing, encoding bytes, and graph-local `ElementId` spelling.
- [ ] Keep ordinary reuse/default origin in provenance, not new logical value
      kinds.
- [ ] Give `Ref<T>` identity-only meaning; infer no ownership, containment,
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
Stage 10: release qualification
```

## Stage summary and exit evidence

| Stage | Demonstrable outcome | Owning gate |
| --- | --- | --- |
| 1 | Reproducible workspace, automation, package/effect boundaries, and only feasible foundation tests | [Implementation Stage 1](development/IMPLEMENTATION-STAGES.md#stage-1-initialize-the-implementation-foundation) |
| Freeze | Grammar, semantics, IR, identity, vocabulary, APIs, diagnostics, limits, fixtures, and traceability accepted | [Contract freeze](development/CONTRACT-FREEZE.md) |
| 2 | Minimal `.neu` scalar compiles through validated IR, reader, and independent probe library | [Implementation Stage 2](development/IMPLEMENTATION-STAGES.md#stage-2-implement-the-minimal-atomic-core) |
| 3 | Full source-text and scalar behavior complete end to end; no future constructs parsed early | [Implementation Stage 3](development/IMPLEMENTATION-STAGES.md#stage-3-complete-source-text-and-scalar-vertical-slices) |
| 4 | Records, defaults, nullability, and lists complete as separate vertical slices | [Implementation Stage 4](development/IMPLEMENTATION-STAGES.md#stage-4-implement-records-defaults-and-lists-as-vertical-slices) |
| 5 | Reuse, cycles, typed references, recursion boundary, and alpha-equivalence complete | [Implementation Stage 5](development/IMPLEMENTATION-STAGES.md#stage-5-implement-reuse-and-references-as-vertical-slices) |
| 6 | Strict captured vocabulary bytes and qualified source values pass compiler/reader/probe | [Implementation Stage 6](development/IMPLEMENTATION-STAGES.md#stage-6-implement-captured-vocabulary-as-one-vertical-boundary) |
| 7 | Valid encoded IR round-trips logically; hostile IR fails boundedly | [Implementation Stage 7](development/IMPLEMENTATION-STAGES.md#stage-7-implement-one-external-neutral-ir-encoding) |
| 8 | Formatter, host CLI, standalone probe, docs, and traceability complete | [Implementation Stage 8](development/IMPLEMENTATION-STAGES.md#stage-8-complete-formatter-cli-standalone-probe-and-traceability) |
| 9 | Property, fuzz, limits, security, performance, soak, mutation, and quality gates pass | [Implementation Stage 9](development/IMPLEMENTATION-STAGES.md#stage-9-harden-correctness-security-and-performance) |
| 10 | Complete artifacts and retained release evidence approved | [Release qualification](development/RELEASE.md) |

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

After host bootstrap, run from the future implementation repository root:

```bash
cargo xtask environment verify
cargo xtask ci stage1       # before contract freeze / during Stage 1
cargo xtask ci pr           # Stage 2 onward; selects active suites
cargo xtask ci nightly
cargo xtask ci release      # release candidates only
```

Individual tasks and evidence behavior are defined in
[ENVIRONMENT-AUTOMATION.md](development/ENVIRONMENT-AUTOMATION.md). Test
activation and quality gates are defined in
[TESTING.md](development/TESTING.md).

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
