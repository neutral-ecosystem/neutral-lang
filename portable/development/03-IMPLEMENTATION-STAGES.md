<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v0 implementation stages

Status: ordered execution checklist

This document owns implementation sequencing. Cross-cutting environment,
testing, identity, and release details are defined in sibling documents and are
linked rather than duplicated.

## Delivery invariant

After Stage 2, every source feature is implemented as one complete vertical
slice:

```text
frozen requirement and fixture
    → raw tokens/layout needed by the feature
    → parser production
    → static semantics
    → logical IR and companion records
    → public reader
    → standalone probe observation
    → diagnostics, limits, and conformance
```

A parser production does not become accepted production behavior before the
rest of its slice is complete. Temporary development tests do not enter the
normative fixture corpus or stable diagnostic catalogue.

## Stage 1: initialize the implementation foundation

Stage 1 creates a compilable, testable shell without implementing Neutral
source behavior.

### Step 1: create workspace packages and ownership

Create this initial package set:

```text
neutral-core          source identity, spans, diagnostics, limits, cancellation
neutral-ir            public logical IR, source map, provenance, derivation
neutral-vocabulary    closed logical schema and strict bundle validation
neutral-compiler      capture, private frontend/semantics, IR lowering
neutral-reader        external artifact validation and immutable reader views
neutral-encoding      validated-document external artifact encoder
neutral-probe         reader-only library and standalone probe binary
neutral-cli           capture/compile/validate/format host commands
neutral-test-support  reusable test-only builders and assertions
neutral-test-suite    cross-package smoke/integration/system/conformance tests
neutral-bench         benchmark harnesses and immutable corpora
xtask                 developer/CI/evidence automation
```

- [x] Create virtual workspace manifest, lockfile, selected toolchain policy, formatting,
      lint, dependency, and quality/test-profile configuration.
- [x] Mark automation/test/benchmark packages non-published.
- [x] Keep unit tests colocated; use the ownership layout in
      [04-TESTING.md](04-TESTING.md).
- [x] Add meaningful crate/module documentation describing ownership and
      prohibited effects; do not enforce an exact line count.
- [x] Add compilable shells without placeholder panics or fake language behavior.
- [x] Keep package versions and public API stability at `0.x` until contract
      freeze/release policy says otherwise.

#### Step validation

- [x] `cargo metadata`, workspace check, lint, tests, and docs pass.
- [x] Every package has one owner and no duplicate test/fixture tree.
- [x] No production package depends on automation/test/benchmark packages.
- [x] Tracked files remain unchanged after checks.

### Step 2: enforce dependency and effect boundaries

- [x] `neutral-core` has no compiler, reader, CLI, or host dependencies.
- [x] `neutral-ir` depends only on core and reviewed value utilities.
- [x] `neutral-vocabulary` depends only on core/public logical model contracts.
- [x] `neutral-compiler` depends on core, IR, and vocabulary; its frontend,
      semantic model, and lowering remain private.
- [x] `neutral-reader` depends on core, IR, vocabulary, and later the selected IR
      encoding implementation; it performs no acquisition.
- [x] `neutral-probe` depends only on core/reader-facing contracts and approved
      output/argument utilities.
- [x] `neutral-cli` owns filesystem/process-facing host behavior but does not
      become the independent probe artifact.
- [x] Forbid filesystem, environment, network, command, locale, and clock access
      from `compile_captured` dependency closure.
- [x] Forbid unsafe code in project-owned v0 crates; audit transitive dependency
      unsafe separately rather than claiming it is absent.

#### Step validation

- [x] Automated package-graph policy rejects every forbidden edge.
- [x] `cargo tree --package neutral-probe --edges all` matches the allowlist.
- [x] A compile-time/dependency audit proves the pure compiler closure has no
      effectful host adapter.
- [x] Deliberate forbidden edges fail Stage 1 CI.

### Step 3: establish environment, automation, and Stage 1 tests

- [x] Implement [00-ENVIRONMENT-AUTOMATION.md](00-ENVIRONMENT-AUTOMATION.md) Layers
      0–2 and the stable `cargo xtask` interface.
- [x] Add only the active Stage 1 tests defined by
      [04-TESTING.md](04-TESTING.md): automation, environment, workspace, dependency,
      package shell, and probe allowlist.
- [x] Record the active stage in `config/development-stage.toml`.
- [x] Create the conformance manifest with all known cases planned but none
      falsely active as compiler behavior.
- [x] Configure Stage 1, PR, nightly, and release workflow shells; later profiles
      select only active suites.

#### Step validation

- [x] `cargo xtask ci stage1` passes from a clean checkout.
- [x] Every active suite is nonempty and every future suite is explicitly planned,
      not intentionally failing.
- [x] A zero-test active suite fails discovery.
- [x] A fresh supported host and development container pass Stage 1.

### Stage 1 validation

- [x] Workspace, environment, automation, dependency boundaries, documentation,
      and active Stage 1 tests pass.
- [x] No production source parser, semantic behavior, stable Neutral diagnostic,
      public IR payload, or vocabulary decoder has been implemented.
- [x] The standalone probe package is independently buildable even though it has
      no language document to inspect yet.
- [x] Stage 2 remains blocked by the normative contract-freeze gate.

---

## Mandatory contract-freeze gate

Complete and approve every gate in
[02-CONTRACT-FREEZE.md](02-CONTRACT-FREEZE.md) before Stage 2. The freeze includes
accepted identity/fingerprint and vocabulary bundle contracts from
[01-IDENTITY-AND-VOCABULARY.md](01-IDENTITY-AND-VOCABULARY.md).

- [x] Approved freeze manifest identifies every governing contract revision.
- [x] Initial fixture/oracle manifest is reviewed and immutable.
- [x] No blocking normative question remains.
- [x] Production Stage 2 tasks link frozen requirements and expected evidence.

---

## Stage 2: implement the minimal atomic core

The first complete source path is:

```neu
neu "0.1"
module minimal

num answer = 42
```

### Step 1: activate minimal fixtures and oracles

- [x] Add one positive minimal fixture and final-v0-invalid malformed variants.
- [x] Do not classify additional valid v0 declarations as a normative error.
- [x] Assign expected stable diagnostics only for behavior invalid in final v0.
- [x] Add expected logical IR, source map, provenance, derivation, resource facts,
      and standalone probe summary.
- [x] Activate these cases from Stage 2 in the conformance manifest.

#### Step validation

- [x] Every active case has requirement IDs and one complete oracle.
- [x] No milestone-only implementation limitation appears in conformance.
- [x] Fixture discovery is deterministic and nonempty.

### Step 2: implement foundational core, capture, and diagnostics

- [x] Implement typed logical source identity, exact byte content digest, checked
      half-open spans, line/column derivation, diagnostics, limits, cancellation,
      and result classes.
- [x] Implement `CompilationRequest`, resolver contract, immutable
      `CapturedCompilation`, `capture`, I/O-free `compile_captured`, and
      convenience `compile`.
- [x] Apply the accepted digest/transcript contract and test vectors.
- [x] Ensure any diagnostic/fatal/cancellation result exposes no authoritative IR.

#### Step validation

- [x] UTF-8/CRLF/BOM span and digest vectors pass.
- [x] Diagnostic ordering/rendering is deterministic, bounded, and safe.
- [x] Capture never falls back to ambient authority.
- [x] Recompiling one captured object is mutation-free and deterministic.

### Step 3: implement the minimal frontend slice

- [x] Lex only tokens needed for exact headers and one `num` binding, while
      retaining physical newlines and original spans.
- [x] Normalize layout into semantic line ends for those complete constructs.
- [x] Parse exact language/module headers and one explicit scalar binding.
- [x] Keep tokens/tree/recovery private and prevent recovered syntax from
      becoming authoritative.
- [x] Reject malformed final-v0-invalid variants with frozen diagnostics.

#### Step validation

- [x] Token/layout/parser fixtures agree with frozen oracles.
- [x] LF/CRLF/lone-CR/trailing/no-trailing newline forms are logically equal.
- [x] Malformed UTF-8/NUL/BOM/headers/numbers terminate safely within limits.
- [x] Parser types cannot be imported outside compiler internals.

### Step 4: implement minimal semantics, IR, reader, and probe

- [x] Validate exact `0.1`, one module scope, names, protected words, explicit
      `num`, and exact numeric value.
- [x] Implement module-symbol identity and declaration fingerprint using frozen
      contracts.
- [x] Lower module/declaration/type/value plus source map, explicit/normalization
      provenance, derivation partitions, and resource facts.
- [x] Expose immutable in-process reader views.
- [x] Implement probe library traversal and source-linked consumer diagnostic.
- [x] Implement standalone probe binary shell for later encoded input without
      linking the compiler.

#### Step validation

- [x] Minimal fixture compiles through reader/probe and matches all oracles.
- [x] Probe dependency allowlist passes.
- [x] Formatting-only source changes preserve logical meaning/fingerprint where
      specified and update source facts correctly.
- [x] Repeated/concurrent results are equal modulo `ElementId` mapping.

### Stage 2 validation

- [x] `cargo xtask ci pr` passes with newly active minimal smoke, unit,
      integration, system, conformance, property, security, and fuzz-smoke cases.
- [x] Every minimal failure returns no authoritative IR.
- [x] The end-to-end path remains runnable for all later stages.

---

## Stage 3: complete source-text and scalar vertical slices

Stage 3 extends shared lexical/layout behavior only as each scalar feature needs
it. It does not parse record, list, reuse, reference, or vocabulary productions.

### Slice 3.1: complete source text, identifiers, comments, and boundaries

- [x] Add fixtures/oracles for identifiers, protected names, punctuation
      rejection, comments, strings' lexical boundaries, newline/comment
      ambiguity, and explicit unsupported symbols.
- [x] Implement full ASCII identifier categories and protected names.
- [x] Implement line and non-nesting block comments as nonsemantic trivia.
- [x] Complete raw newline/layout behavior for currently accepted scalar
      declarations, including malformed delimiter recovery needed by them.
- [x] Preserve trivia privately for later formatter work without lowering it.
- [x] Carry every behavior through diagnostics, source facts, reader-observable
      unchanged semantics, limits, and conformance.

#### Slice validation

- [x] Comment insertion/removal preserves logical IR.
- [x] Identifier and boundary property tests match frozen grammar.
- [x] Unterminated/misleading comments fail safely and deterministically.
- [x] No future grammar production has become accepted.

### Slice 3.2: strings and Booleans

- [x] Activate string/escape/Unicode/control and Boolean fixtures.
- [x] Implement string and Boolean tokens/parser values.
- [x] Type-check explicit `string`/`bool` bindings.
- [x] Lower exact logical values, source maps, provenance, derivation, and limits.
- [x] Expose values through reader and probe.

#### Slice validation

- [x] Every escape, Unicode boundary, invalid surrogate/control, and limit case
      passes its oracle.
- [x] Safe rendering never emits hostile control text unescaped.
- [x] Reader/probe behavior uses typed values, not source parsing.

### Slice 3.3: complete exact numbers

- [x] Activate sign, separator, fraction, exponent, equality, normalization, and
      limit fixtures.
- [x] Implement full frozen numeric grammar and exact normalized representation.
- [x] Use no host floating-point conversion.
- [x] Apply NHT numeric fingerprint vectors.
- [x] Expose normalized exact values and normalization provenance.

#### Slice validation

- [x] Equivalent spellings normalize/fingerprint equally.
- [x] Boundary/over-limit values fail before proportional allocation.
- [x] Locale and host numeric types cannot affect output.

### Slice 3.4: nullable scalar and explicit null

- [x] Activate `T?`, outer widening, and null fixtures for scalar types.
- [x] Parse postfix nullability and `null` only in currently supported scalar
      contexts.
- [x] Implement exact identity plus outer `T` → `T?` compatibility.
- [x] Lower/read/probe typed null and nullable values.
- [x] Keep null distinct from structural omission.

#### Slice validation

- [x] Null without nullable expected type fails.
- [x] Inner/generic widening is not accidentally accepted.
- [x] IR/reader distinguishes null from absence.

### Stage 3 validation

- [x] Every Stage 3 slice is complete through probe and conformance.
- [x] No record/list/reuse/reference/vocabulary syntax is accepted yet.
- [x] Stage 2 remains green.

---

## Stage 4: implement records, defaults, and lists as vertical slices

### Slice 4.1: nominal record declarations and contextual values

- [x] Activate record declaration/value, field, nominal compatibility, duplicate,
      wrong-kind, and recursion fixtures.
- [x] Add record/field/contextual-value grammar only now.
- [x] Collect root declarations before resolution and enforce one scope.
- [x] Resolve nominal types and validate contextual fields.
- [x] Reject missing/unknown/duplicate fields, anonymous records, shorthand,
      structural compatibility, and embedded recursive cycles.
- [x] Lower record declarations/values and source/provenance/derivation facts.
- [x] Expose nominal records through reader/probe.

#### Slice validation

- [x] Declaration order is nonsemantic.
- [x] Every field failure has stable ownership/span.
- [x] Public IR contains no parser/private semantic types.
- [x] Record limits fail before proportional work.

### Slice 4.2: closed defaults and omission

- [x] Activate all required/defaulted × nullable/non-nullable combinations.
- [x] Add field-default grammar and closed-constant semantic validation.
- [x] Permit only scalar/null and recursively closed currently supported record
      constants; lists join when Slice 4.3 activates them.
- [x] Materialize final logical values for omitted defaulted fields.
- [x] Record explicit versus user-default provenance without changing logical
      value kind.
- [x] Reject names, `ref`, and expressions in defaults.

#### Slice validation

- [x] Final values and provenance match frozen oracles.
- [x] Omission is not represented as `null`, `none`, or `absent`.
- [x] Defaults create no value/reference dependency edge.

### Slice 4.3: ordered homogeneous lists

- [x] Activate `List<T>`, list values, empty context, nested/default list,
      invariance, order, and size/depth fixtures.
- [x] Add list type/value grammar only now.
- [x] Implement invariant generic resolution and contextual element typing.
- [x] Extend closed defaults to lists.
- [x] Preserve logical list order through IR/reader/probe/fingerprints.
- [x] Enforce item/depth/traversal limits.

#### Slice validation

- [x] Empty lists require expected type.
- [x] Generic covariance remains rejected.
- [x] Large lists fail before proportional allocation.
- [x] Record/default/list combined fixture passes end to end.

### Stage 4 validation

- [x] Records, defaults, nullability, and lists are complete vertical slices.
- [x] Every newly accepted parser form has public reader/probe evidence.
- [x] Stage 2–3 suites remain green.

---

## Stage 5: implement reuse and references as vertical slices

### Slice 5.1: ordinary immutable value reuse

- [x] Activate forward/transitive/nested reuse, unknown/wrong-kind, cycle, and
      traversal-limit fixtures.
- [x] Add unqualified name value grammar only now.
- [x] Resolve after declaration collection and build the value-dependency graph.
- [x] Detect every cycle deterministically with stable primary/related spans.
- [x] Lower the final logical value and reuse provenance, not a reuse value kind.
- [x] Expose final value/provenance through reader/probe.

#### Slice validation

- [x] Forward reuse works independent of declaration order.
- [x] Direct/indirect cycles fail with no IR.
- [x] Deep chains are bounded.
- [x] Fingerprints use final logical definitions as frozen.

### Slice 5.2: typed identity references and recursion boundary

- [x] Activate `Ref<T>`, `ref(name)`, forward target, unknown/wrong-kind/type,
      recursion, and edge-integrity fixtures.
- [x] Add reference type/value grammar only now.
- [x] Require exact target binding type and exclude identity edges from value
      dependency.
- [x] Permit nominal recursive cycles only through `Ref<T>`.
- [x] Lower typed identity edges using graph-local `ElementId` plus provenance.
- [x] Expose typed edge traversal through reader/probe.

#### Slice validation

- [x] Field names/source position add no relationship meaning.
- [x] Reader validates target existence/kind/type.
- [x] Identity cycles do not become value cycles.
- [x] Probe traverses IDs, not parsed strings.

### Slice 5.3: alpha-equivalence and graph identity

- [x] Implement one-to-one whole-graph `ElementId` mapping comparison.
- [x] Keep logical payload equality separate from companion/envelope comparison.
- [x] Add property vectors for reflexivity, symmetry, transitivity, random ID
      renaming, changed edge/value/type, duplicate ID, and dangling edge.
- [x] Prohibit cross-document persistence of `ElementId` in public docs/APIs.

#### Slice validation

- [x] All alpha-equivalence properties pass.
- [x] Fingerprints and structural equality agree on their documented scopes.
- [x] Invalid graph states never produce validated reader views.

### Stage 5 validation

- [x] Reuse and identity references remain semantically distinct end to end.
- [x] Full core fixtures pass compiler/reader/probe and all graph adversarial
      cases fail closed.
- [x] Stage 2–4 suites remain green.

---

## Stage 6: implement captured vocabulary as one vertical boundary

### Slice 6.1: strict captured bundle decoder and logical contract

- [x] Implement the accepted JSON byte/schema contract and exact digest checks
      from [01-IDENTITY-AND-VOCABULARY.md](01-IDENTITY-AND-VOCABULARY.md).
- [x] Activate duplicate/unknown/executable/malformed/limit/default/recursion and
      independent digest/transcript vectors.
- [x] Decode into untrusted intermediate data, then validate closed schema,
      features, names, types, fields, defaults, and recursion.
- [x] Expose only immutable validated logical vocabulary contracts.
- [x] Perform no code loading or external I/O.

#### Slice validation

- [x] All accepted/hostile bundle vectors pass.
- [x] Duplicate keys are detected before map collapse.
- [x] Raw JSON numbers and executable shapes fail closed.
- [x] Allocation-before-validation review passes.

### Slice 6.2: captured `use` and qualified values

- [x] Activate `use Fixture`, `Fixture::Metadata`, payload/default, lock mismatch,
      missing, collision, unknown feature/type, and reader contract fixtures.
- [x] Add `use` and qualified-type grammar only now.
- [x] Resolve exclusively from exact captured lock input.
- [x] Validate bundle before source payloads.
- [x] Type-check vocabulary contextual values using ordinary binding/value rules.
- [x] Apply vocabulary defaults as final values with distinct provenance.
- [x] Record exact identity/version/schema/encoding/digest/features in IR and
      derivation.
- [x] Expose qualified typed data through reader/probe without interpretation.

#### Slice validation

- [x] Minimal vocabulary fixture passes end to end.
- [x] Missing/mismatch/unknown/executable cases fail with frozen diagnostics.
- [x] Source cannot trigger registry/path/network acquisition.
- [x] Probe has no `Fixture`-specific behavior.

### Stage 6 validation

- [x] Vocabulary byte decoding, capture, source syntax, semantics, IR, reader,
      probe, diagnostics, provenance, derivation, and limits form one complete
      vertical boundary.
- [x] External-reader contract fixtures are ready for Stage 7 encoded IR.
- [x] Stage 2–5 suites remain green.

---

## Stage 7: implement one external Neutral IR encoding

### Step 1: accept the encoding decision

- [x] Compare candidates for exact numbers, duplicate detection, unknown fields,
      bounded decoding, ecosystem tooling, and language bindings.
- [x] Freeze framing, versions, capabilities, sizes, payload/companion/envelope
      sections, malformed behavior, and all invalid encoded states.
- [x] State that bytes are noncanonical and logical equality remains structural.

#### Step validation

- [x] The decision represents every frozen logical/companion contract.
- [x] Exact numbers require no host floating-point conversion.
- [x] Every unknown/malformed/version/capability case has a specified result.
- [x] Security and allocation review approves the framing design.

### Step 2: encode validated documents

- [x] Encode only fully validated in-memory documents.
- [x] Keep producer/build facts in the envelope.
- [x] Preserve all logical and companion contracts without making byte order
      semantic.

#### Step validation

- [x] Every valid in-memory fixture encodes within configured limits.
- [x] Encoding does not mutate validated input.
- [x] Producer/envelope changes do not alter logical payload equality.
- [x] Byte determinism, where provided, is documented as implementation behavior
      rather than logical identity.

### Step 3: decode and validate hostile input

- [x] Validate framing/length/version/capability before allocation.
- [x] Decode into untrusted intermediate data.
- [x] Validate IDs, types, values, references, source maps, provenance,
      derivation, limits, and exact vocabulary contracts.
- [x] Expose reader views only after complete validation.

#### Step validation

- [x] Valid artifacts produce expected immutable reader observations.
- [x] Every invalid encoded state returns a bounded classified error.
- [x] No unchecked length controls proportional allocation.
- [x] Missing/mismatched vocabulary contracts fail without lookup.

### Stage 7 validation

- [x] Valid artifacts decode to alpha-equivalent logical IR.
- [x] Corrupt/truncated/oversized/duplicate/dangling/unknown cases fail boundedly.
- [x] Standalone probe inspects encoded artifacts without compiler linkage.
- [x] Decoder fuzzing and
      [allocation review](evidence/stage7-decoder-allocation-review.md) pass.

---

## Stage 8: complete formatter, CLI, standalone probe, and traceability

### Step 1: reference formatter vertical tool slice

- [x] Implement canonical header order, four-space indentation, field layout,
      spacing, commas, no semicolons, and deterministic comment placement.
- [x] Prove idempotence and parse/format/parse logical equality.
- [x] Keep formatted bytes separate from IR/source identity/signing.

#### Step validation

- [x] Formatting is idempotent across the complete source corpus.
- [x] Parse/format/parse preserves logical IR and accepted provenance categories.
- [x] Comment placement is deterministic and comments remain nonsemantic.

### Step 2: CLI host tools

- [x] Implement compile, validate, and format commands with explicit resolver,
      limits, disclosure, destinations, overwrite, atomic-write, and exit policy.
- [x] Keep inspect proof in standalone `neutral-probe`; shared rendering may use a
      reader-only public library.
- [x] Test child-process/filesystem/stdio/permission/cancellation behavior.

#### Step validation

- [x] Every command has stable usage and exit classes.
- [x] Failure/cancellation leaves no authoritative partial output.
- [x] Host paths/credentials obey disclosure policy.
- [x] System tests invoke built binaries, not CLI internals.

### Step 3: complete standalone probe

- [x] Enumerate all metadata/declarations/types/final values/references/
      vocabulary/provenance through reader APIs.
- [x] Map one consumer diagnostic to source.
- [x] Compare in-process reader/probe library and external probe binary summaries.
- [x] Enforce dependency allowlist in release CI.

#### Step validation

- [x] Probe package builds/tests independently from compiler packages.
- [x] In-process and encoded summaries match modulo envelope-only metadata.
- [x] Probe traversal is bounded and safe for hostile validated graphs.
- [x] Source-linked consumer diagnostic maps to the expected original span.

### Step 4: close documentation and traceability

- [x] Complete requirement → decision → fixture → implementation → test mapping.
- [x] Publish grammar, semantics, IR, identity, vocabulary, API, encoding,
      diagnostic, limits, formatter, and tool documentation.
- [x] Check master syntax items only with complete evidence.

#### Step validation

- [x] Every accepted `NL-*`/`SYN-*` ID maps to executable evidence.
- [x] No fixture, diagnostic, public API, or implementation behavior is orphaned.
- [x] Documentation examples compile and repository coherence checks pass.

### Stage 8 validation

- [x] Formatter, CLI, probe, docs, traceability, and all active tests pass.
- [x] No explicit v0 exclusion is accepted.
- [x] No public consumer needs source/private compiler models.

---

## Stage 9: harden correctness, security, and performance

- [x] Complete property/metamorphic suites.
- [x] Put all test bodies in the owning crate's `tests/` directory; production
      sources retain only path-based test-module declarations.
- [x] Complete source, vocabulary, IR, formatter, and probe fuzz campaigns.
- [x] Test every structural limit at and one over boundary.
- [x] Inject cancellation/faults at every stage.
- [x] Complete dependency/build-script/proc-macro/native/unsafe review.
- [x] Complete cache poisoning/cross-request/stale-source-fact review.
- [x] Close the focused exact-number mutation subset: 34 caught, 3 unviable,
      and no missed mutants.
- [ ] Complete controlled phase/end-to-end performance, growth, memory,
      concurrency, stress, and soak profiles.
- [ ] Complete coverage, mutation, static work-product reviews, threat model, and
      quality evaluation defined in [04-TESTING.md](04-TESTING.md).

#### Remaining Stage 9 evidence

- [x] Run every source, vocabulary, IR, formatter, and probe fuzz target for at
      least 900 seconds on an untraced runner without a crash, timeout, hang, or
      sanitizer finding.
- [x] Record controlled release and 50,000-iteration extended-soak baselines,
      including whole-command peak RSS.
- [ ] Obtain component-level allocation evidence, or approve and record a
      measurement exception consistent with the no-unsafe policy.
- [x] Pass the configured whole-workspace coverage gate: 90.57% lines,
      90.71% functions, and 81.72% regions against 85%/90%/80% thresholds.
- [x] Close the broader selected mutation review: 244 caught, 27 unviable,
      and no missed viable mutants out of 271 selected mutants.

### Stage 9 validation

- [x] No known input causes unbounded work, panic, stack exhaustion, invalid
      typed IR, stale source facts, cross-request leakage, or partial success.
- [x] Determinism holds under repeated/concurrent/adversarial execution.
- [ ] All approved quality gates and residual-risk reviews pass.

---

## Stage 10: qualify and release v0

Execute [05-RELEASE.md](05-RELEASE.md).

### Stage 10 — v0 Release Validation
# v0 Release Engineering & Repository Overhaul

## 1. Repository Structure and Cleanup

* [ ] Review and overhaul the repository structure for clarity, maintainability, and future growth.
* [ ] Organize crates, tools, scripts, tests, fixtures, documentation, generated files, reports, and release artifacts into clearly defined locations.
* [ ] Remove obsolete, deprecated, duplicated, experimental, or stage-specific files that are no longer required.
* [ ] Remove old test outputs, stale generated results, temporary files, deprecated documentation, and unused fixtures.
* [ ] Preserve useful historical development logs/evidence only where required for archival purposes.
* [ ] Ensure archived development evidence is clearly separated from active tests and release validation.
* [ ] Ensure tests never depend directly on archived or mutable development artifacts.
* [ ] Where historical/portable artifacts are needed for testing, copy or convert them into stable test fixtures owned by the test suite.
* [ ] Review the repository from the perspective of a new contributor cloning it for the first time.
* [ ] Ensure there are no hidden assumptions about local files, manually created directories, environment variables, tools, or developer-specific state.
* [ ] Review and correct `.gitignore`.
* [ ] Ensure generated files, build outputs, coverage data, fuzz artifacts, temporary files, IDE files, local environment files, and release outputs are ignored appropriately.
* [ ] Ensure required fixtures, lock files, manifests, schemas, contracts, and reproducibility-critical files are not accidentally ignored.

---

## 2. Development Environment

* [ ] Define all tools and dependencies required for development in a documented development-environment specification.
* [ ] Provide a single development-environment setup command/script.
* [ ] The setup process should install or verify:

  * Rust toolchain and required components.
  * Formatting tools.
  * Linting tools.
  * Testing tools.
  * Coverage tooling.
  * Fuzzing tooling.
  * Benchmark/performance tooling.
  * Packaging/release tooling.
  * Validation/probe tooling.
  * Any external utilities required by scripts or tests.
* [ ] Clearly separate runtime dependencies from development-only dependencies.
* [ ] Pin or constrain tool versions where reproducibility requires it.
* [ ] Detect missing tools and provide actionable error messages.
* [ ] Make environment setup idempotent where practical.
* [ ] Document the supported development platforms and known platform-specific requirements.
* [ ] Ensure a clean checkout can be prepared for development using only the documented setup process.

---

## 3. Project-Level Developer Commands

* [ ] Simplify common Cargo workflows behind easy-to-remember project-level commands.
* [ ] Avoid requiring developers to remember long combinations of `cargo` flags, package selections, feature flags, or tool-specific commands.
* [ ] Provide simple commands for at least:

  * `setup`
  * `fmt`
  * `lint`
  * `check`
  * `test`
  * `test-unit`
  * `test-smoke`
  * `test-integration`
  * `test-performance`
  * `coverage`
  * `fuzz`
  * `quality`
  * `build`
  * `validate`
  * `package`
  * `release`
  * `clean`
* [ ] Commands should work from the repository root.
* [ ] Commands should fail fast with clear diagnostics.
* [ ] Commands should compose existing project tooling rather than duplicating logic.
* [ ] CI must invoke the same project-level commands used by developers locally.
* [ ] Remove obsolete `stageX`, stage-numbered, or stage-dependent developer commands.
* [ ] Replace stage-specific command logic with stable Neutral project commands.
* [ ] Future development stages must extend existing commands rather than introduce parallel release/test systems.

---

## 4. Automation Scripts

* [ ] Create a clear `scripts/` hierarchy grouped by responsibility.
* [ ] Separate platform-specific automation from platform-neutral project automation.
* [ ] Provide a structure that can grow cleanly, for example:

```text
scripts/
── linux/
      ├── dev/
      ├── test/
      ├── quality/
      ├── release/
      ├── tooling/
      └── platform/

```

* [ ] Design the structure so future Windows and macOS automation can be added without reorganizing existing scripts.
* [ ] Avoid embedding large shell scripts directly inside CI configuration.
* [ ] Reuse scripts between local development and CI where practical.
* [ ] Document the responsibility and expected inputs/outputs of non-trivial scripts.

---

## 5. Testing Strategy

* [ ] Keep unit testing as the lowest-level test layer.
* [ ] Add a clearly defined smoke-test suite.
* [ ] Add a clearly defined integration-test suite.
* [ ] Add performance/regression testing.
* [ ] Clearly document what belongs in each test category.
* [ ] Ensure test categories can be run independently.
* [ ] Ensure the complete test suite can also be executed with one command.
* [ ] Ensure tests use controlled fixtures rather than mutable release/archive files.
* [ ] Review existing tests and remove obsolete, duplicated, brittle, or implementation-detail-heavy tests.
* [ ] Ensure tests verify behavior and contracts rather than incidental repository layout where possible.
* [ ] Ensure explicitly unsupported or forbidden behavior has negative tests proving it remains rejected.
* [ ] Ensure regression tests exist for previously discovered release-critical defects.
* [ ] Ensure tests are deterministic where practical.

---

## 6. Coverage

* [ ] Replace or improve the current coverage workflow with a reliable and portable coverage tool.
* [ ] Make coverage tooling installable through the documented development-environment setup.
* [ ] Provide one simple project-level coverage command.
* [ ] Generate human-readable coverage reports.
* [ ] Generate machine-readable coverage output where useful for CI.
* [ ] Store generated coverage reports in a clearly defined repository-local report directory.
* [ ] Keep generated coverage output out of version control unless a specific release-evidence file is intentionally retained.
* [ ] Make coverage collection work consistently across supported development and CI environments.
* [ ] Document coverage exclusions.
* [ ] Avoid misleading coverage numbers caused by generated code, fixtures, test helpers, or intentionally unreachable compatibility code.
* [ ] Establish reasonable coverage gates for release-critical components where appropriate.

---

## 7. Fuzzing and Quality Tooling

* [ ] Review and improve the fuzzing strategy.
* [ ] Make fuzz tooling installable through the normal development-environment setup.
* [ ] Provide one simple fuzz command.
* [ ] Clearly separate fuzz targets by subsystem.
* [ ] Preserve minimized regression inputs discovered by fuzzing as deterministic test fixtures where appropriate.
* [ ] Keep temporary fuzz corpora, crashes, and generated artifacts organized and ignored correctly.
* [ ] Add broader automated quality checks where appropriate.
* [ ] Create a unified `quality` command that can execute the required formatting, linting, static checks, tests, and other quality gates.
* [ ] Avoid introducing quality tools that duplicate existing checks without providing meaningful additional coverage.

---

## 8. Warnings and Static Quality

* [ ] Fix all existing compiler warnings.
* [ ] Fix warnings emitted by project tooling, tests, examples, benchmarks, and build scripts.
* [ ] Remove deprecated API usage where practical.
* [ ] Remove dead code unless explicitly justified.
* [ ] Review unnecessary `allow` attributes and warning suppressions.
* [ ] Any remaining suppression must have a clear documented reason.
* [ ] Configure CI so new release-relevant warnings cannot silently accumulate.
* [ ] Ensure the release candidate builds cleanly without unexpected warnings.

---

## 9. Versioning

* [ ] Centralize the project version in one authoritative source of truth.
* [ ] Changing a release version, for example `v0.1.0` → `v0.1.1`, must require editing only that authoritative source.
* [ ] Individual crates, packages, manifests, generated metadata, documentation, and release records must not require manual version synchronization.
* [ ] Define clearly which version values are authoritative and which are generated.
* [ ] Generate derived version metadata deterministically.
* [ ] Add validation that detects manually edited or stale generated version information.
* [ ] Ensure version propagation works correctly across the entire workspace.
* [ ] Ensure development, pre-release, and release versions follow one documented policy.
* [ ] No hardcoded value inline

---

## 10. Locking and Generated Metadata

* [ ] Overhaul the current lock system.
* [ ] Eliminate workflows that require manually editing lock versions, hashes, SHAs, or generated identifiers.
* [ ] Define a single authoritative source for lock metadata.
* [ ] Automatically regenerate derived lock information when its source changes.
* [ ] Validate lock consistency automatically.
* [ ] Fail clearly when generated lock information is stale.
* [ ] Ensure generated lock files are deterministic.
* [ ] Ensure lock generation does not depend on undocumented local machine state.
* [ ] Document exactly when lock information should change and why.
* [ ] Avoid coupling unrelated components through shared manually maintained hashes or version values.

---

## 11. Generated Files

* [ ] Identify every generated file in the repository.
* [ ] Document the source of truth for each generated file.
* [ ] Provide commands to regenerate generated files.
* [ ] Generated files must be reproducible from their authoritative inputs.
* [ ] Add validation that detects stale generated files.
* [ ] Avoid manually editing generated files.
* [ ] Clearly mark generated files where appropriate.
* [ ] Ensure generated data does not introduce unnecessary diffs between machines.
* [ ] Keep release-only generated artifacts separate from source-controlled generated metadata.

---

## 12. Release Workflow

* [ ] Define one documented release workflow for the entire v0 project.
* [ ] The complete v0 release must be buildable, testable, validatable, and packageable through that workflow.
* [ ] Provide a single high-level release/update command.
* [ ] The release command should orchestrate, as appropriate:

  * Version propagation.
  * Metadata generation.
  * Lock regeneration.
  * Formatting checks.
  * Linting.
  * Unit tests.
  * Smoke tests.
  * Integration tests.
  * Performance/regression checks.
  * Coverage checks.
  * Fuzz/quality gates where required.
  * Build.
  * Independent validation.
  * Packaging.
  * Release evidence generation.
* [ ] Repetitive release tasks must be automated wherever practical.
* [ ] Avoid manual edits across multiple files during release preparation.
* [ ] Release preparation must fail when generated metadata is stale or inconsistent.
* [ ] Release preparation must fail when required validation is incomplete.
* [ ] A clean checkout must be able to reproduce the release using only documented commands.
* [ ] The release must not depend on undocumented developer-local state.

---

## 13. Release Artifacts and Reports

* [ ] Define exactly which artifacts constitute a valid release.
* [ ] Generate all required release artifacts automatically.
* [ ] Store release outputs in a predictable location.
* [ ] Separate temporary build output from retained release artifacts.
* [ ] Generate a release validation report automatically where practical.
* [ ] Retain required validation evidence.
* [ ] Retained evidence should include enough information to reproduce or audit the release.
* [ ] Ensure reports capture failures clearly rather than silently producing incomplete evidence.
* [ ] Avoid retaining unnecessary transient test output as permanent release evidence.

---

## 14. Independent Validation

* [ ] Independent probe validation passes against the actual release candidate.
* [ ] Probe validation must not accidentally use workspace-only state unavailable to downstream users.
* [ ] Validate the packaged/released output rather than only the development workspace where applicable.
* [ ] Ensure probe tooling is clearly separated from the implementation being validated.
* [ ] Record the exact probe version used for the release.
* [ ] Record the exact inputs and compatibility contracts used by the probe.
* [ ] Retain the required independent-validation evidence.

---

## 15. Contracts and Compatibility Boundaries

* [ ] Record the exact versions of every release-relevant independent contract.
* [ ] This includes, where applicable:

  * File formats.
  * Schemas.
  * Encoding contracts.
  * Protocol versions.
  * Vocabulary versions.
  * IR contracts.
  * CLI compatibility guarantees.
  * Package interfaces.
  * Probe interfaces.
  * External tool requirements.
* [ ] Avoid relying on implicit compatibility assumptions.
* [ ] Ensure contract/version information can be generated or validated automatically where possible.
* [ ] Document compatibility guarantees for the v0 release.
* [ ] Document explicitly unsupported compatibility scenarios.

---

## 16. Negative and Exclusion Validation

* [ ] All explicitly excluded behavior remains rejected.
* [ ] All unsupported behavior remains rejected.
* [ ] All forbidden syntax, formats, contracts, or compatibility paths remain rejected.
* [ ] Add automated tests for release-critical exclusions.
* [ ] Ensure cleanup/refactoring does not accidentally re-enable deprecated behavior.
* [ ] Record deliberate exclusions in the release documentation.

---

## 17. CI/CD Alignment

* [ ] CI uses the same formatting command used locally.
* [ ] CI uses the same lint command used locally.
* [ ] CI uses the same test commands used locally.
* [ ] CI uses the same validation commands used locally.
* [ ] CI uses the same build/package commands used locally.
* [ ] Avoid maintaining a second implementation of the release process inside CI configuration.
* [ ] Keep CI orchestration thin and move reusable logic into project-owned commands/scripts.
* [ ] Ensure a CI failure can be reproduced locally using the same command.
* [ ] Ensure CI starts from a sufficiently clean environment to detect missing dependencies and hidden local assumptions.

---

## 18. Contributor Experience

* [ ] A new contributor should be able to clone the repository and understand how to build it without studying internal stage history.
* [ ] Provide a short, obvious getting-started path.
* [ ] Document:

  * Environment setup.
  * Build.
  * Test.
  * Quality checks.
  * Coverage.
  * Fuzzing.
  * Validation.
  * Packaging.
  * Release preparation.
* [ ] Prefer a small set of stable commands over many specialized scripts.
* [ ] Ensure command names remain stable as the project grows.
* [ ] Avoid exposing internal implementation/stage terminology in normal developer workflows.
* [ ] Ensure failures explain what the contributor needs to fix rather than assuming project knowledge.

---

## 19. Future Development Maintainability

* [ ] Future stages must be able to extend the repository without modifying unrelated components.
* [ ] Avoid duplicated release logic between crates or future products.
* [ ] Keep platform-specific functionality isolated behind appropriate boundaries.
* [ ] Keep testing infrastructure reusable across future crates and components.
* [ ] Keep release/versioning logic centralized.
* [ ] Keep generated metadata logic centralized.
* [ ] Avoid architecture that requires manually synchronizing files across multiple crates.
* [ ] Make adding new crates, vocabularies, packages, probes, or platform integrations straightforward.
* [ ] Ensure future Windows/macOS support can reuse the same project-level development and release model.

---

# Final v0 Release Gate

The v0 release is approved only when all of the following are true:

* [ ] All previous stage gates pass from a clean release-candidate build.
* [ ] Repository structure and cleanup requirements pass.
* [ ] All required development tools can be installed or verified through the documented environment setup.
* [ ] All compiler, test, build, and release-relevant warnings are resolved or explicitly justified.
* [ ] Unit tests pass.
* [ ] Smoke tests pass.
* [ ] Integration tests pass.
* [ ] Required performance/regression tests pass.
* [ ] Required coverage gates pass.
* [ ] Required fuzzing/quality gates pass.
* [ ] All explicitly unsupported, excluded, deprecated, or forbidden behavior remains rejected.
* [ ] Generated files are current and reproducible.
* [ ] Version metadata is consistent across the entire project.
* [ ] Lock metadata is current, automated, and reproducible.
* [ ] The release can be reproduced from a clean checkout.
* [ ] CI and local development use the same validation/build workflow.
* [ ] The complete v0 release can be built, tested, validated, and packaged through the documented release command.
* [ ] All required release artifacts are generated successfully.
* [ ] All required validation evidence is retained.
* [ ] Independent probe validation passes against the release candidate.
* [ ] Exact versions of all independent contracts, schemas, protocols, tools, and compatibility boundaries are recorded.
* [ ] The release record documents:

  * Known limitations.
  * Residual risks.
  * Explicit exclusions.
  * Compatibility guarantees.
  * Unsupported behavior.
  * Deferred work.
* [ ] No known release-blocking issue remains unresolved.
* [ ] A new user can clone, set up, build, test, and validate the project using only documented commands.
* [ ] A future patch release such as `v0.1.0` → `v0.1.1` requires changing the version in one authoritative location and running one release/update command.
* [ ] Future development does not require reintroducing stage-specific commands, duplicated release logic, or manual cross-repository metadata synchronization.

#### Stage Gate

Stage 10 passes only when the project can be reproduced from a clean checkout, validated through the standard automated workflow, released with centralized version management, and extended in future versions without relying on repetitive manual project-wide edits.
