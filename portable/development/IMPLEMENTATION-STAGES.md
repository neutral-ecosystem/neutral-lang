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
neutral-probe         reader-only library and standalone probe binary
neutral-cli           capture/compile/validate/format host commands
neutral-test-support  reusable test-only builders and assertions
neutral-test-suite    cross-package smoke/integration/system/conformance tests
neutral-bench         benchmark harnesses and immutable corpora
xtask                 developer/CI/evidence automation
```

- [ ] Create virtual workspace manifest, lockfile, pinned toolchain, formatting,
      lint, dependency, and quality/test-profile configuration.
- [ ] Mark automation/test/benchmark packages non-published.
- [ ] Keep unit tests colocated; use the ownership layout in
      [TESTING.md](TESTING.md).
- [ ] Add meaningful crate/module documentation describing ownership and
      prohibited effects; do not enforce an exact line count.
- [ ] Add compilable shells without placeholder panics or fake language behavior.
- [ ] Keep package versions and public API stability at `0.x` until contract
      freeze/release policy says otherwise.

#### Step validation

- [ ] `cargo metadata`, workspace check, lint, tests, and docs pass.
- [ ] Every package has one owner and no duplicate test/fixture tree.
- [ ] No production package depends on automation/test/benchmark packages.
- [ ] Tracked files remain unchanged after checks.

### Step 2: enforce dependency and effect boundaries

- [ ] `neutral-core` has no compiler, reader, CLI, or host dependencies.
- [ ] `neutral-ir` depends only on core and reviewed value utilities.
- [ ] `neutral-vocabulary` depends only on core/public logical model contracts.
- [ ] `neutral-compiler` depends on core, IR, and vocabulary; its frontend,
      semantic model, and lowering remain private.
- [ ] `neutral-reader` depends on core, IR, vocabulary, and later the selected IR
      encoding implementation; it performs no acquisition.
- [ ] `neutral-probe` depends only on core/reader-facing contracts and approved
      output/argument utilities.
- [ ] `neutral-cli` owns filesystem/process-facing host behavior but does not
      become the independent probe artifact.
- [ ] Forbid filesystem, environment, network, command, locale, and clock access
      from `compile_captured` dependency closure.
- [ ] Forbid unsafe code in project-owned v0 crates; audit transitive dependency
      unsafe separately rather than claiming it is absent.

#### Step validation

- [ ] Automated package-graph policy rejects every forbidden edge.
- [ ] `cargo tree --package neutral-probe --edges all` matches the allowlist.
- [ ] A compile-time/dependency audit proves the pure compiler closure has no
      effectful host adapter.
- [ ] Deliberate forbidden edges fail Stage 1 CI.

### Step 3: establish environment, automation, and Stage 1 tests

- [ ] Implement [ENVIRONMENT-AUTOMATION.md](ENVIRONMENT-AUTOMATION.md) Layers
      0–2 and the stable `cargo xtask` interface.
- [ ] Add only the active Stage 1 tests defined by
      [TESTING.md](TESTING.md): automation, environment, workspace, dependency,
      package shell, and probe allowlist.
- [ ] Record the active stage in `config/development-stage.toml`.
- [ ] Create the conformance manifest with all known cases planned but none
      falsely active as compiler behavior.
- [ ] Configure Stage 1, PR, nightly, and release workflow shells; later profiles
      select only active suites.

#### Step validation

- [ ] `cargo xtask ci stage1` passes from a clean checkout.
- [ ] Every active suite is nonempty and every future suite is explicitly planned,
      not intentionally failing.
- [ ] A zero-test active suite fails discovery.
- [ ] A fresh supported host and development container pass Stage 1.

### Stage 1 validation

- [ ] Workspace, environment, automation, dependency boundaries, documentation,
      and active Stage 1 tests pass.
- [ ] No production source parser, semantic behavior, stable Neutral diagnostic,
      public IR payload, or vocabulary decoder has been implemented.
- [ ] The standalone probe package is independently buildable even though it has
      no language document to inspect yet.
- [ ] Stage 2 remains blocked by the normative contract-freeze gate.

---

## Mandatory contract-freeze gate

Complete and approve every gate in
[CONTRACT-FREEZE.md](CONTRACT-FREEZE.md) before Stage 2. The freeze includes
accepted identity/fingerprint and vocabulary bundle contracts from
[IDENTITY-AND-VOCABULARY.md](IDENTITY-AND-VOCABULARY.md).

- [ ] Freeze manifest exists and identifies every governing contract revision.
- [ ] Initial fixture/oracle manifest is reviewed and immutable.
- [ ] No blocking normative question remains.
- [ ] Production Stage 2 tasks link frozen requirements and expected evidence.

---

## Stage 2: implement the minimal atomic core

The first complete source path is:

```neu
neu "0.1"
module minimal

num answer = 42
```

### Step 1: activate minimal fixtures and oracles

- [ ] Add one positive minimal fixture and final-v0-invalid malformed variants.
- [ ] Do not classify additional valid v0 declarations as a normative error.
- [ ] Assign expected stable diagnostics only for behavior invalid in final v0.
- [ ] Add expected logical IR, source map, provenance, derivation, resource facts,
      and standalone probe summary.
- [ ] Activate these cases from Stage 2 in the conformance manifest.

#### Step validation

- [ ] Every active case has requirement IDs and one complete oracle.
- [ ] No milestone-only implementation limitation appears in conformance.
- [ ] Fixture discovery is deterministic and nonempty.

### Step 2: implement foundational core, capture, and diagnostics

- [ ] Implement typed logical source identity, exact byte content digest, checked
      half-open spans, line/column derivation, diagnostics, limits, cancellation,
      and result classes.
- [ ] Implement `CompilationRequest`, resolver contract, immutable
      `CapturedCompilation`, `capture`, I/O-free `compile_captured`, and
      convenience `compile`.
- [ ] Apply the accepted digest/transcript contract and test vectors.
- [ ] Ensure any diagnostic/fatal/cancellation result exposes no authoritative IR.

#### Step validation

- [ ] UTF-8/CRLF/BOM span and digest vectors pass.
- [ ] Diagnostic ordering/rendering is deterministic, bounded, and safe.
- [ ] Capture never falls back to ambient authority.
- [ ] Recompiling one captured object is mutation-free and deterministic.

### Step 3: implement the minimal frontend slice

- [ ] Lex only tokens needed for exact headers and one `num` binding, while
      retaining physical newlines and original spans.
- [ ] Normalize layout into semantic line ends for those complete constructs.
- [ ] Parse exact language/module headers and one explicit scalar binding.
- [ ] Keep tokens/tree/recovery private and prevent recovered syntax from
      becoming authoritative.
- [ ] Reject malformed final-v0-invalid variants with frozen diagnostics.

#### Step validation

- [ ] Token/layout/parser fixtures agree with frozen oracles.
- [ ] LF/CRLF/lone-CR/trailing/no-trailing newline forms are logically equal.
- [ ] Malformed UTF-8/NUL/BOM/headers/numbers terminate safely within limits.
- [ ] Parser types cannot be imported outside compiler internals.

### Step 4: implement minimal semantics, IR, reader, and probe

- [ ] Validate exact `0.1`, one module scope, names, protected words, explicit
      `num`, and exact numeric value.
- [ ] Implement module-symbol identity and declaration fingerprint using frozen
      contracts.
- [ ] Lower module/declaration/type/value plus source map, explicit/normalization
      provenance, derivation partitions, and resource facts.
- [ ] Expose immutable in-process reader views.
- [ ] Implement probe library traversal and source-linked consumer diagnostic.
- [ ] Implement standalone probe binary shell for later encoded input without
      linking the compiler.

#### Step validation

- [ ] Minimal fixture compiles through reader/probe and matches all oracles.
- [ ] Probe dependency allowlist passes.
- [ ] Formatting-only source changes preserve logical meaning/fingerprint where
      specified and update source facts correctly.
- [ ] Repeated/concurrent results are equal modulo `ElementId` mapping.

### Stage 2 validation

- [ ] `cargo xtask ci pr` passes with newly active minimal smoke, unit,
      integration, system, conformance, property, security, and fuzz-smoke cases.
- [ ] Every minimal failure returns no authoritative IR.
- [ ] The end-to-end path remains runnable for all later stages.

---

## Stage 3: complete source-text and scalar vertical slices

Stage 3 extends shared lexical/layout behavior only as each scalar feature needs
it. It does not parse record, list, reuse, reference, or vocabulary productions.

### Slice 3.1: complete source text, identifiers, comments, and boundaries

- [ ] Add fixtures/oracles for identifiers, protected names, punctuation
      rejection, comments, strings' lexical boundaries, newline/comment
      ambiguity, and explicit unsupported symbols.
- [ ] Implement full ASCII identifier categories and protected names.
- [ ] Implement line and non-nesting block comments as nonsemantic trivia.
- [ ] Complete raw newline/layout behavior for currently accepted scalar
      declarations, including malformed delimiter recovery needed by them.
- [ ] Preserve trivia privately for later formatter work without lowering it.
- [ ] Carry every behavior through diagnostics, source facts, reader-observable
      unchanged semantics, limits, and conformance.

#### Slice validation

- [ ] Comment insertion/removal preserves logical IR.
- [ ] Identifier and boundary property tests match frozen grammar.
- [ ] Unterminated/misleading comments fail safely and deterministically.
- [ ] No future grammar production has become accepted.

### Slice 3.2: strings and Booleans

- [ ] Activate string/escape/Unicode/control and Boolean fixtures.
- [ ] Implement string and Boolean tokens/parser values.
- [ ] Type-check explicit `string`/`bool` bindings.
- [ ] Lower exact logical values, source maps, provenance, derivation, and limits.
- [ ] Expose values through reader and probe.

#### Slice validation

- [ ] Every escape, Unicode boundary, invalid surrogate/control, and limit case
      passes its oracle.
- [ ] Safe rendering never emits hostile control text unescaped.
- [ ] Reader/probe behavior uses typed values, not source parsing.

### Slice 3.3: complete exact numbers

- [ ] Activate sign, separator, fraction, exponent, equality, normalization, and
      limit fixtures.
- [ ] Implement full frozen numeric grammar and exact normalized representation.
- [ ] Use no host floating-point conversion.
- [ ] Apply NHT numeric fingerprint vectors.
- [ ] Expose normalized exact values and normalization provenance.

#### Slice validation

- [ ] Equivalent spellings normalize/fingerprint equally.
- [ ] Boundary/over-limit values fail before proportional allocation.
- [ ] Locale and host numeric types cannot affect output.

### Slice 3.4: nullable scalar and explicit null

- [ ] Activate `T?`, outer widening, and null fixtures for scalar types.
- [ ] Parse postfix nullability and `null` only in currently supported scalar
      contexts.
- [ ] Implement exact identity plus outer `T` → `T?` compatibility.
- [ ] Lower/read/probe typed null and nullable values.
- [ ] Keep null distinct from structural omission.

#### Slice validation

- [ ] Null without nullable expected type fails.
- [ ] Inner/generic widening is not accidentally accepted.
- [ ] IR/reader distinguishes null from absence.

### Stage 3 validation

- [ ] Every Stage 3 slice is complete through probe and conformance.
- [ ] No record/list/reuse/reference/vocabulary syntax is accepted yet.
- [ ] Stage 2 remains green.

---

## Stage 4: implement records, defaults, and lists as vertical slices

### Slice 4.1: nominal record declarations and contextual values

- [ ] Activate record declaration/value, field, nominal compatibility, duplicate,
      wrong-kind, and recursion fixtures.
- [ ] Add record/field/contextual-value grammar only now.
- [ ] Collect root declarations before resolution and enforce one scope.
- [ ] Resolve nominal types and validate contextual fields.
- [ ] Reject missing/unknown/duplicate fields, anonymous records, shorthand,
      structural compatibility, and embedded recursive cycles.
- [ ] Lower record declarations/values and source/provenance/derivation facts.
- [ ] Expose nominal records through reader/probe.

#### Slice validation

- [ ] Declaration order is nonsemantic.
- [ ] Every field failure has stable ownership/span.
- [ ] Public IR contains no parser/private semantic types.
- [ ] Record limits fail before proportional work.

### Slice 4.2: closed defaults and omission

- [ ] Activate all required/defaulted × nullable/non-nullable combinations.
- [ ] Add field-default grammar and closed-constant semantic validation.
- [ ] Permit only scalar/null and recursively closed currently supported record
      constants; lists join when Slice 4.3 activates them.
- [ ] Materialize final logical values for omitted defaulted fields.
- [ ] Record explicit versus user-default provenance without changing logical
      value kind.
- [ ] Reject names, `ref`, and expressions in defaults.

#### Slice validation

- [ ] Final values and provenance match frozen oracles.
- [ ] Omission is not represented as `null`, `none`, or `absent`.
- [ ] Defaults create no value/reference dependency edge.

### Slice 4.3: ordered homogeneous lists

- [ ] Activate `List<T>`, list values, empty context, nested/default list,
      invariance, order, and size/depth fixtures.
- [ ] Add list type/value grammar only now.
- [ ] Implement invariant generic resolution and contextual element typing.
- [ ] Extend closed defaults to lists.
- [ ] Preserve logical list order through IR/reader/probe/fingerprints.
- [ ] Enforce item/depth/traversal limits.

#### Slice validation

- [ ] Empty lists require expected type.
- [ ] Generic covariance remains rejected.
- [ ] Large lists fail before proportional allocation.
- [ ] Record/default/list combined fixture passes end to end.

### Stage 4 validation

- [ ] Records, defaults, nullability, and lists are complete vertical slices.
- [ ] Every newly accepted parser form has public reader/probe evidence.
- [ ] Stage 2–3 suites remain green.

---

## Stage 5: implement reuse and references as vertical slices

### Slice 5.1: ordinary immutable value reuse

- [ ] Activate forward/transitive/nested reuse, unknown/wrong-kind, cycle, and
      traversal-limit fixtures.
- [ ] Add unqualified name value grammar only now.
- [ ] Resolve after declaration collection and build the value-dependency graph.
- [ ] Detect every cycle deterministically with stable primary/related spans.
- [ ] Lower the final logical value and reuse provenance, not a reuse value kind.
- [ ] Expose final value/provenance through reader/probe.

#### Slice validation

- [ ] Forward reuse works independent of declaration order.
- [ ] Direct/indirect cycles fail with no IR.
- [ ] Deep chains are bounded.
- [ ] Fingerprints use final logical definitions as frozen.

### Slice 5.2: typed identity references and recursion boundary

- [ ] Activate `Ref<T>`, `ref(name)`, forward target, unknown/wrong-kind/type,
      recursion, and edge-integrity fixtures.
- [ ] Add reference type/value grammar only now.
- [ ] Require exact target binding type and exclude identity edges from value
      dependency.
- [ ] Permit nominal recursive cycles only through `Ref<T>`.
- [ ] Lower typed identity edges using graph-local `ElementId` plus provenance.
- [ ] Expose typed edge traversal through reader/probe.

#### Slice validation

- [ ] Field names/source position add no relationship meaning.
- [ ] Reader validates target existence/kind/type.
- [ ] Identity cycles do not become value cycles.
- [ ] Probe traverses IDs, not parsed strings.

### Slice 5.3: alpha-equivalence and graph identity

- [ ] Implement one-to-one whole-graph `ElementId` mapping comparison.
- [ ] Keep logical payload equality separate from companion/envelope comparison.
- [ ] Add property vectors for reflexivity, symmetry, transitivity, random ID
      renaming, changed edge/value/type, duplicate ID, and dangling edge.
- [ ] Prohibit cross-document persistence of `ElementId` in public docs/APIs.

#### Slice validation

- [ ] All alpha-equivalence properties pass.
- [ ] Fingerprints and structural equality agree on their documented scopes.
- [ ] Invalid graph states never produce validated reader views.

### Stage 5 validation

- [ ] Reuse and identity references remain semantically distinct end to end.
- [ ] Full core fixtures pass compiler/reader/probe and all graph adversarial
      cases fail closed.
- [ ] Stage 2–4 suites remain green.

---

## Stage 6: implement captured vocabulary as one vertical boundary

### Slice 6.1: strict captured bundle decoder and logical contract

- [ ] Implement the accepted JSON byte/schema contract and exact digest checks
      from [IDENTITY-AND-VOCABULARY.md](IDENTITY-AND-VOCABULARY.md).
- [ ] Activate duplicate/unknown/executable/malformed/limit/default/recursion and
      independent digest/transcript vectors.
- [ ] Decode into untrusted intermediate data, then validate closed schema,
      features, names, types, fields, defaults, and recursion.
- [ ] Expose only immutable validated logical vocabulary contracts.
- [ ] Perform no code loading or external I/O.

#### Slice validation

- [ ] All accepted/hostile bundle vectors pass.
- [ ] Duplicate keys are detected before map collapse.
- [ ] Raw JSON numbers and executable shapes fail closed.
- [ ] Allocation-before-validation review passes.

### Slice 6.2: captured `use` and qualified values

- [ ] Activate `use Fixture`, `Fixture::Metadata`, payload/default, lock mismatch,
      missing, collision, unknown feature/type, and reader contract fixtures.
- [ ] Add `use` and qualified-type grammar only now.
- [ ] Resolve exclusively from exact captured lock input.
- [ ] Validate bundle before source payloads.
- [ ] Type-check vocabulary contextual values using ordinary binding/value rules.
- [ ] Apply vocabulary defaults as final values with distinct provenance.
- [ ] Record exact identity/version/schema/encoding/digest/features in IR and
      derivation.
- [ ] Expose qualified typed data through reader/probe without interpretation.

#### Slice validation

- [ ] Minimal vocabulary fixture passes end to end.
- [ ] Missing/mismatch/unknown/executable cases fail with frozen diagnostics.
- [ ] Source cannot trigger registry/path/network acquisition.
- [ ] Probe has no `Fixture`-specific behavior.

### Stage 6 validation

- [ ] Vocabulary byte decoding, capture, source syntax, semantics, IR, reader,
      probe, diagnostics, provenance, derivation, and limits form one complete
      vertical boundary.
- [ ] External-reader contract fixtures are ready for Stage 7 encoded IR.
- [ ] Stage 2–5 suites remain green.

---

## Stage 7: implement one external Neutral IR encoding

### Step 1: accept the encoding decision

- [ ] Compare candidates for exact numbers, duplicate detection, unknown fields,
      bounded decoding, ecosystem tooling, and language bindings.
- [ ] Freeze framing, versions, capabilities, sizes, payload/companion/envelope
      sections, malformed behavior, and all invalid encoded states.
- [ ] State that bytes are noncanonical and logical equality remains structural.

#### Step validation

- [ ] The decision represents every frozen logical/companion contract.
- [ ] Exact numbers require no host floating-point conversion.
- [ ] Every unknown/malformed/version/capability case has a specified result.
- [ ] Security and allocation review approves the framing design.

### Step 2: encode validated documents

- [ ] Encode only fully validated in-memory documents.
- [ ] Keep producer/build facts in the envelope.
- [ ] Preserve all logical and companion contracts without making byte order
      semantic.

#### Step validation

- [ ] Every valid in-memory fixture encodes within configured limits.
- [ ] Encoding does not mutate validated input.
- [ ] Producer/envelope changes do not alter logical payload equality.
- [ ] Byte determinism, where provided, is documented as implementation behavior
      rather than logical identity.

### Step 3: decode and validate hostile input

- [ ] Validate framing/length/version/capability before allocation.
- [ ] Decode into untrusted intermediate data.
- [ ] Validate IDs, types, values, references, source maps, provenance,
      derivation, limits, and exact vocabulary contracts.
- [ ] Expose reader views only after complete validation.

#### Step validation

- [ ] Valid artifacts produce expected immutable reader observations.
- [ ] Every invalid encoded state returns a bounded classified error.
- [ ] No unchecked length controls proportional allocation.
- [ ] Missing/mismatched vocabulary contracts fail without lookup.

### Stage 7 validation

- [ ] Valid artifacts decode to alpha-equivalent logical IR.
- [ ] Corrupt/truncated/oversized/duplicate/dangling/unknown cases fail boundedly.
- [ ] Standalone probe inspects encoded artifacts without compiler linkage.
- [ ] Decoder fuzzing and allocation review pass.

---

## Stage 8: complete formatter, CLI, standalone probe, and traceability

### Step 1: reference formatter vertical tool slice

- [ ] Implement canonical header order, four-space indentation, field layout,
      spacing, commas, no semicolons, and deterministic comment placement.
- [ ] Prove idempotence and parse/format/parse logical equality.
- [ ] Keep formatted bytes separate from IR/source identity/signing.

#### Step validation

- [ ] Formatting is idempotent across the complete source corpus.
- [ ] Parse/format/parse preserves logical IR and accepted provenance categories.
- [ ] Comment placement is deterministic and comments remain nonsemantic.

### Step 2: CLI host tools

- [ ] Implement compile, validate, and format commands with explicit resolver,
      limits, disclosure, destinations, overwrite, atomic-write, and exit policy.
- [ ] Keep inspect proof in standalone `neutral-probe`; shared rendering may use a
      reader-only public library.
- [ ] Test child-process/filesystem/stdio/permission/cancellation behavior.

#### Step validation

- [ ] Every command has stable usage and exit classes.
- [ ] Failure/cancellation leaves no authoritative partial output.
- [ ] Host paths/credentials obey disclosure policy.
- [ ] System tests invoke built binaries, not CLI internals.

### Step 3: complete standalone probe

- [ ] Enumerate all metadata/declarations/types/final values/references/
      vocabulary/provenance through reader APIs.
- [ ] Map one consumer diagnostic to source.
- [ ] Compare in-process reader/probe library and external probe binary summaries.
- [ ] Enforce dependency allowlist in release CI.

#### Step validation

- [ ] Probe package builds/tests independently from compiler packages.
- [ ] In-process and encoded summaries match modulo envelope-only metadata.
- [ ] Probe traversal is bounded and safe for hostile validated graphs.
- [ ] Source-linked consumer diagnostic maps to the expected original span.

### Step 4: close documentation and traceability

- [ ] Complete requirement → decision → fixture → implementation → test mapping.
- [ ] Publish grammar, semantics, IR, identity, vocabulary, API, encoding,
      diagnostic, limits, formatter, and tool documentation.
- [ ] Check master syntax items only with complete evidence.

#### Step validation

- [ ] Every accepted `NL-*`/`SYN-*` ID maps to executable evidence.
- [ ] No fixture, diagnostic, public API, or implementation behavior is orphaned.
- [ ] Documentation examples compile and repository coherence checks pass.

### Stage 8 validation

- [ ] Formatter, CLI, probe, docs, traceability, and all active tests pass.
- [ ] No explicit v0 exclusion is accepted.
- [ ] No public consumer needs source/private compiler models.

---

## Stage 9: harden correctness, security, and performance

- [ ] Complete property/metamorphic suites.
- [ ] Complete source, vocabulary, IR, formatter, and probe fuzz campaigns.
- [ ] Test every structural limit at and one over boundary.
- [ ] Inject cancellation/faults at every stage.
- [ ] Complete dependency/build-script/proc-macro/native/unsafe review.
- [ ] Complete cache poisoning/cross-request/stale-source-fact review.
- [ ] Complete controlled phase/end-to-end performance, growth, memory,
      concurrency, stress, and soak profiles.
- [ ] Complete coverage, mutation, static work-product reviews, threat model, and
      quality evaluation defined in [TESTING.md](TESTING.md).

### Stage 9 validation

- [ ] No known input causes unbounded work, panic, stack exhaustion, invalid
      typed IR, stale source facts, cross-request leakage, or partial success.
- [ ] Determinism holds under repeated/concurrent/adversarial execution.
- [ ] All approved quality gates and residual-risk reviews pass.

---

## Stage 10: qualify and release v0

Execute [RELEASE.md](RELEASE.md).

### Stage 10 validation

- [ ] All prior stage gates pass from a clean release candidate.
- [ ] Required artifacts and complete retained evidence exist.
- [ ] Independent probe proof passes.
- [ ] All exclusions remain rejected.
- [ ] Release approval records exact independent contract versions and residual
      risks.
