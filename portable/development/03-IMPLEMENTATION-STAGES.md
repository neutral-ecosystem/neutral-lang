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
- [ ] Complete source, vocabulary, IR, formatter, and probe fuzz campaigns.
- [x] Test every structural limit at and one over boundary.
- [x] Inject cancellation/faults at every stage.
- [x] Complete dependency/build-script/proc-macro/native/unsafe review.
- [x] Complete cache poisoning/cross-request/stale-source-fact review.
- [ ] Complete controlled phase/end-to-end performance, growth, memory,
      concurrency, stress, and soak profiles.
- [ ] Complete coverage, mutation, static work-product reviews, threat model, and
      quality evaluation defined in [04-TESTING.md](04-TESTING.md).

### Stage 9 validation

- [ ] No known input causes unbounded work, panic, stack exhaustion, invalid
      typed IR, stale source facts, cross-request leakage, or partial success.
- [ ] Determinism holds under repeated/concurrent/adversarial execution.
- [ ] All approved quality gates and residual-risk reviews pass.

---

## Stage 10: qualify and release v0

Execute [05-RELEASE.md](05-RELEASE.md).

### Stage 10 — v0 Release Validation

- [ ] All previous stage gates pass from a clean release-candidate build.
- [ ] The complete v0 release can be built, tested, validated, and packaged using the documented release workflow.
- [ ] Versioning is centralized so that changing a release version, for example `v0.1.0` → `v0.1.1`, requires changing it in only one authoritative location.
- [ ] A single release/update command propagates the version and generated metadata across all required crates, packages, manifests, documentation, and artifacts.
- [ ] Repetitive development and release tasks are automated wherever practical instead of requiring manual edits across multiple files.
- [ ] Generated files and metadata have a clearly defined source of truth and can be regenerated deterministically.
- [ ] Future stages can extend the project without requiring changes to unrelated components or duplicating release logic.
- [ ] Common development operations such as formatting, linting, testing, validation, building, and release preparation are exposed through simple project-level commands.
- [ ] CI uses the same validation and build commands used by developers locally.
- [ ] A clean checkout can reproduce the release without relying on undocumented local state or manually prepared files.
- [ ] Required release artifacts are generated successfully and all required validation evidence is retained.
- [ ] Independent probe validation passes against the release candidate.
- [ ] All explicitly excluded, unsupported, or forbidden behavior remains rejected.
- [ ] Release validation records the exact versions of all independent contracts, schemas, protocols, and compatibility boundaries.
- [ ] The release record documents known limitations, residual risks, exclusions, compatibility guarantees, and deferred work.
- [ ] No known release-blocking issue remains unresolved.

#### Stage Gate

Stage 10 passes only when the project can be reproduced from a clean checkout, validated through the standard automated workflow, released with centralized version management, and extended in future versions without relying on repetitive manual project-wide edits.