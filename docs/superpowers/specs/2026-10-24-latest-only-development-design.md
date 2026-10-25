<!-- SPDX-License-Identifier: Apache-2.0 -->

# Latest-only development pipeline

Status: approved for implementation on 24-10-2026.

## Intent and compatibility policy

Before the first stable `1.0.0` release, the development checkout supports one
current language pipeline. It need not continue compiling earlier source profiles
or reading earlier project artifacts. Backward compatibility becomes a requirement
for the stable baseline, not for every development experiment.

The user approved removing obsolete public compiler/reader/encoding paths, keeping
shared code needed by the current pipeline, migrating useful tests and rejecting
old artifacts explicitly. This is a breaking development cleanup, not a release,
history rewrite, schema renumbering, new language feature or quality-gate waiver.

## Approaches

1. **Selected: remove obsolete pipelines and retain shared primitives.** Migrate
   their callers and useful tests, then delete old-only implementation. This
   reduces maintenance and makes the running code match the development policy.
2. Disable old dispatch but keep its implementations and acceptance tests. This
   is easier initially but retains the parallel maintenance burden; rejected.
3. Move old implementations into optional compatibility features/crates. This
   still maintains compatibility code and configuration before it is needed;
   rejected. Git history and published releases already preserve past versions.

## Current implementation and cutover boundary

The current complete expression-project pipeline is the target:

| Boundary | Current selector |
| --- | --- |
| Source header | `neu "1.0"` |
| Capture / project IR | `neutral.capture/v3` / `neutral.project-ir/3` |
| Binary project artifact | `NIR-PROJECT-CBOR/3`, magic `NEUP3\r\n\x1a` |
| View / result / identity | Existing `/3` selectors |
| Expression / evaluator / expansion / decimal | Existing independently versioned `/1` profiles |
| Captured composition vocabulary | Existing schema `2.0` |

These numbers identify separate contracts. Vocabulary schema `2.0` and evaluator
`/1` are still current and must not be removed merely because project IR is `/3`.
Keep exact framing/profile/feature checks; unknown and old selectors fail closed.
Never retry an old artifact under the current decoder or relabel historical bytes.

At design review, the checkout exported older compiler and codec entry points alongside
the current pipeline. The CLI still compiles through the earlier single-document
path, while the standalone probe dispatches older formats. Current expression
code also imports scalar, type, graph, contract and companion helpers from modules
named `project` and `composition`; deleting those modules wholesale would break
the current pipeline.

## Required changes

### One production pipeline and API

- Remove old-only source-profile variants, capture/compile/analyze entry points,
  old complete document/project authority types, old encoders/decoders and old
  probe dispatch. Do not retain compatibility aliases or feature flags.
- Make the current complete project model and its capture, compile, validation,
  transport and public-inspection APIs the ordinary supported path. Prefer
  unqualified project names once only one implementation remains; do not rename
  serialized selectors as part of Rust API cleanup.
- First extract genuinely shared scalar/type/value/schema, graph, nominal
  resolution, companion and bounded-CBOR helpers into their existing owning
  layers. Keep their checks, budgets, cancellation and fallible allocation.
  Delete an old module only after its live callers have migrated.
- Keep one caller-owned syntax cache and its atomic successful generations.
  Remove historical grammar branches and duplicate cache wrappers. Preserve exact
  bytes, module, current profile and parser-policy reuse keys; recompute semantics,
  vocabulary contracts, source evidence and identities on every request.

### CLI, formatting and probe

- Route `compile` and `validate` through the current complete project pipeline;
  emitted `.nir` is the current complete artifact, never an older wrapper.
- Preserve existing safe host I/O, cancellation, diagnostics, atomic output and
  overwrite behavior. Continue using explicitly supplied sources and exact locks;
  no import fetching, implicit registry lookup or filesystem discovery is added.
  Existing single-source CLI scope does not require inventing a new project file.
- Route capture/reporting through the current request contract. Profile discovery
  describes the one development implementation honestly; this does not declare
  the unfinished authoring service or stable release complete.
- `format` must accept the current grammar, preserve meaning and be idempotent.
  Reuse lexical/layout machinery where suitable; do not keep an old compiler just
  to format files. Validate formatted output with the current pipeline.
- Standalone probe accepts current artifacts only, remains compiler-free and never
  evaluates. Preserve safe counts, typed identities, redacted public views and
  explicit root selection. Root listing is not implicit entry-contract negotiation;
  typed entry discovery continues requiring exact public contracts and features.

### Tests, fixtures and development tools

- Migrate still-relevant source, property, security, bounds, allocation-failure,
  cancellation, identity, reader, CLI and cache tests to the current pipeline.
  Preserve the behavior each test protects, not an obsolete public API.
- Remove old-success acceptance suites and old-only test builders. Retain small
  literal rejection fixtures for unsupported headers, magic and selectors.
- Update active conformance registration, fixture builders, fuzz targets/seeds,
  mutation file selection, benchmarks, documentation generation, smoke commands
  and packaging checks. No active gate may depend on an obsolete implementation.
- Preserve current frozen semantic/wire/identity vectors. Historical release
  specifications, immutable vectors and evidence remain historical records, not
  requirements to execute old pipelines in the development checkout. Do not edit
  their bytes or delete approval evidence to manufacture a passing result.
- Record a current compatibility-policy amendment and update active plan/checklist
  references that require historical acceptance. The user's new policy supersedes
  that preservation requirement, not unrelated semantic or quality requirements.
- Existing source-bound measurements are not proof for the new tree. Keep quality
  thresholds; invalidate stale receipts normally and require fresh qualification
  before promotion. Do not claim coverage/mutation/fuzz/performance passed without
  running them against the resulting inputs.

## Stable boundary and non-goals

At the first stable release, freeze the supported source, vocabulary, artifact,
public API and interpretation contracts as the compatibility baseline. Later work
must preserve that baseline or use an explicitly reviewed breaking-release policy;
do not silently reinterpret stable artifacts or promise unsupported future versions.

Not included: new syntax, automatic artifact migration, legacy compatibility
packages, executors/backend adapters, acquisition, authoring GUI implementation,
dependency upgrades, commits, tags, publication or Git-history rewriting.

The current pipeline becomes the sole development implementation, not a claim of
stable readiness. Remaining deferred-project Task 11 qualification and independent
authoring/consumer gates remain open until their own evidence passes.

## Verification and completion

1. Inventory every old public entry point and its callers; classify shared versus
   obsolete implementation before deletion. Keep each migration batch buildable.
2. Demonstrate current `.neu` → complete `.nir` → independent reader/probe and
   explicit consumer reports through both library and CLI paths. Check arithmetic,
   deferred inputs, templates/matrices, vocabularies and public redaction.
3. Prove old/unknown formats reject without fallback or partial output. Verify
   source/profile discovery and formatting round trips use only the current path.
4. Run owning tests after each boundary migration, then `cargo xtask test all`,
   `cargo xtask ci pr`, dependency checks and current literal-oracle verification.
   Do not close the cleanup with references to deleted APIs, missing fixture paths,
   disabled checks or an unbuildable fuzz/benchmark harness.
5. Update the implementation checklist and documentation with actual results and
   outstanding long campaigns. Do not carry forward old measurement approvals.

Builder choices: extraction/module names, migration batch order and removal of
provably dead files. Keep changes reviewable and reuse house helpers. No broad
renaming or alternate framework is necessary to satisfy this policy.
