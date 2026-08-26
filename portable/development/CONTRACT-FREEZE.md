# Neutral v0 normative contract-freeze gate

Status: mandatory pre-implementation gate

This gate closes the gap between design exploration and production compiler
behavior. Repository scaffolding and disposable experiments may happen before
this gate. No production parser acceptance, stable diagnostic, public IR type,
identity algorithm, vocabulary decoder, or public API may be implemented until
the gate is approved.

The gate freezes v0 behavior, not implementation internals. Private data
structures and algorithms may evolve as long as all frozen observations remain
conformant.

## Inputs and authority

The review uses these sources in precedence order:

1. [Neutral v0 architecture](../spec/v0/architecture.md)
2. [Neutral v0 requirements](../spec/v0/requirements.md)
3. [Accepted v0 decisions](../spec/v0/decisions/README.md)
4. [Master v0 syntax checklist](../spec/v0/syntax.md)
5. [Proposed syntax guide](../spec/v0/proposed-syntax-guide.md), once promoted from
   proposal to the accepted author-facing guide
6. [Identity and vocabulary encoding contract](IDENTITY-AND-VOCABULARY.md)
7. [Conformance fixtures](../spec/v0/fixtures/README.md)

The roadmap and development documents order work but cannot add language
behavior. Contradictory accepted documents fail this gate.

## Freeze artifact

Create `spec/v0/freeze.toml` in the future implementation repository. It must
record:

- freeze identifier and approval date;
- exact source revision of every governing document;
- accepted language behavior version;
- logical IR schema version;
- source-map, provenance, and derivation versions;
- vocabulary schema and bundle-encoding versions;
- digest/transcript profile version;
- compiler, reader, and resolver API contract versions;
- diagnostic catalogue revision;
- structural-limits profile revision;
- fixture corpus revision and manifest digest;
- approvers and unresolved blocking issues; and
- superseding freeze identifier, if replaced.

The freeze manifest identifies contracts; it is not itself a language input and
does not enter logical IR equality.

## Gate A: scope and document coherence

- [ ] Every v0 requirement has one stable `NL-*` identifier.
- [ ] Every syntax decision has one stable `SYN-*` identifier.
- [ ] Document precedence is accepted and linked from the repository entry point.
- [ ] Every explicit v0 exclusion is listed consistently in architecture,
      decisions, guide, grammar, and fixtures.
- [ ] No Flow, Neux, runtime, authority, filesystem, network, command, secret, or
      application-specific meaning appears in Neutral core.
- [ ] Later-version placeholders contain no implied v0 commitment.
- [ ] Open questions are classified as blocking, implementation-only, or
      deliberately deferred.
- [ ] Every blocking question is resolved before approval.

### Validation

- [ ] Document-link validation passes.
- [ ] Requirement/decision ID uniqueness validation passes.
- [ ] A generated scope report has no orphan or contradictory requirement.
- [ ] Reviewers confirm that implementation convenience has not expanded v0.

## Gate B: source, lexical, layout, and grammar contract

- [ ] UTF-8, BOM, malformed bytes, NUL, original-byte spans, CRLF, and lone-CR
      behavior are normative.
- [ ] Every raw token and trivia category is specified.
- [ ] Identifier regular languages and protected names are normative.
- [ ] String escapes, exact numeric spelling, Booleans, and `null` are normative.
- [ ] Comment and non-nesting behavior are normative.
- [ ] The raw-newline to semantic-`LINE_END` algorithm is normative for valid and
      malformed delimiter states.
- [ ] The complete context-free grammar is published independently from a parser
      implementation.
- [ ] Parser recovery boundaries, fatal conditions, and diagnostic-cap behavior
      are normative.
- [ ] Every accepted form and explicit exclusion has valid, invalid, boundary,
      and misleading-lookalike examples.

### Validation

- [ ] An executable grammar harness classifies every syntax fixture.
- [ ] Layout fixtures identify expected semantic token streams.
- [ ] No grammar production lacks a semantic and lowering owner.
- [ ] No temporary implementation limitation is represented as normative invalid
      Neutral syntax.

## Gate C: static semantic contract

- [ ] One-module scope, declaration collection, protected names, and duplicate
      rules are normative.
- [ ] Module-symbol identity and declaration revision are semantically distinct.
- [ ] `num`, `string`, `bool`, `T?`, `List<T>`, `Ref<T>`, user records, and
      vocabulary types have complete validity and compatibility rules.
- [ ] Contextual record/list typing and empty-list behavior are normative.
- [ ] Required/defaulted and nullable/non-nullable field axes are normative.
- [ ] Closed-constant defaults and final-value behavior are normative.
- [ ] Ordinary immutable reuse, forward resolution, dependency construction, and
      cycle ownership are normative.
- [ ] `ref(name)` target kind/type checks and identity-only behavior are
      normative.
- [ ] Embedded record recursion and the `Ref<T>` recursion boundary are
      normative.
- [ ] Semantic error ownership and canonical ordering are defined for ambiguous
      multi-error cases.

### Validation

- [ ] Every parsed fixture has one accept/reject result independent of
      declaration order.
- [ ] Every accepted fixture has one expected resolved semantic projection.
- [ ] Decision tables cover compatibility, field state, defaults, and reference
      target classes.
- [ ] Value and identity graphs are demonstrably distinct.

## Gate D: logical IR, identity, and companion contracts

- [ ] Logical payload and artifact envelope are separate.
- [ ] Every declaration, type, value, default, reuse, and reference has a complete
      logical representation.
- [ ] Exact numbers have a host-independent normalized IR representation.
- [ ] All invalid logical IR states are enumerated.
- [ ] `ElementId` is document-local and logical equality is graph
      alpha-equivalence.
- [ ] Module identity, source content identity, module-symbol identity,
      declaration fingerprint, derivation identity, and byte identity are
      distinct.
- [ ] Digest algorithms, domain separation, transcripts, collision handling, and
      test vectors follow [IDENTITY-AND-VOCABULARY.md](IDENTITY-AND-VOCABULARY.md).
- [ ] Source maps use logical source identity plus half-open original-byte spans.
- [ ] Provenance represents explicit source, normalization, reuse, user defaults,
      and vocabulary defaults without creating accidental logical value kinds.
- [ ] Derivation partitions meaning, acceptance/resource, and diagnostic/output
      policy inputs.
- [ ] Resource accounting for logical IR and reader traversal is normative.

### Validation

- [ ] Expected IR graphs can be written without parser/private-model names.
- [ ] Golden comparisons ignore only explicitly nonsemantic details.
- [ ] Independent identity test vectors produce identical digests/fingerprints.
- [ ] Formatting-only changes may alter source maps while preserving logical IR.

## Gate E: vocabulary schema and captured bundle encoding

- [ ] The closed vocabulary logical schema is versioned and accepted.
- [ ] The JSON bundle encoding, duplicate-key behavior, unknown-field behavior,
      limits, and tagged exact-value representation follow
      [IDENTITY-AND-VOCABULARY.md](IDENTITY-AND-VOCABULARY.md).
- [ ] `Fixture::Metadata` and its closed defaults are frozen.
- [ ] Exact logical identity, bundle version, schema version, content digest, and
      required feature set are distinct.
- [ ] Validation order is size/integrity → JSON structure → closed schema →
      structural features → logical schema → source payload.
- [ ] Executable content and unknown required structural behavior fail closed.
- [ ] External readers receive the exact captured contract and perform no lookup.

### Validation

- [ ] Positive, missing, mismatch, duplicate, unknown, executable, limit, and
      hostile bundle fixtures have expected results.
- [ ] Two differently formatted valid bundles may have different content digests
      but equal logical vocabulary contracts.
- [ ] Bundle decoding is bounded before proportional allocation.
- [ ] No vocabulary field can redefine core syntax or semantics.

## Gate F: public APIs and failure contracts

- [ ] `CompilationRequest`, resolver, and capture authority are specified.
- [ ] `CapturedCompilation` contains every decision-affecting captured input.
- [ ] `capture`, Rust `compile_captured`, convenience `compile`, and
      `decode_and_validate` are specified.
- [ ] Authoritative IR exists only on complete success.
- [ ] Diagnostics, capture failures, cancellation, limits, invalid IR, and
      internal defects are distinct result classes.
- [ ] Public reader views are immutable, typed, bounded, and storage-independent.
- [ ] Reentrancy, concurrency, cancellation, ownership, and lifetime behavior are
      specified.
- [ ] No public AST or IR rewrite API exists in v0.
- [ ] The standalone probe can be implemented from reader contracts alone.

### Validation

- [ ] Public API examples compile against interface-only stubs.
- [ ] Probe design imports no compiler or private frontend contract.
- [ ] Cancellation and fatal errors cannot expose partial typed output.
- [ ] Thread-safety and ownership claims are testable on every supported host.

## Gate G: diagnostics, limits, fixtures, and traceability

- [ ] Every diagnostic has stable code, owner layer, severity, safe parameters,
      primary/related location policy, remedy policy, and truncation behavior.
- [ ] Canonical diagnostic ordering is complete.
- [ ] Every compiler/reader structural limit has unit, default profile,
      enforcement owner, and at-limit/one-over fixture.
- [ ] Positive fixtures have expected logical IR, source map, provenance, and
      derivation.
- [ ] Negative fixtures have expected failure class, stable code, safe
      parameters, and original-byte spans.
- [ ] Temporary milestone behavior is stored outside the normative conformance
      corpus.
- [ ] Requirement → decision → fixture → expected result → test traceability is
      complete.

### Validation

- [ ] No normative requirement lacks executable planned evidence.
- [ ] No normative fixture lacks an oracle.
- [ ] No diagnostic code exists only because a feature is not implemented yet.
- [ ] The fixture manifest is deterministically ordered and content-digested.

## Approval and change control

The gate passes only when Gates A–G pass and all blocking findings are closed.
Approval produces the freeze manifest and a review record.

After approval:

- [ ] Any semantic change updates the governing decision and affected
      requirement, grammar, fixtures, diagnostics, IR, API, and traceability.
- [ ] The change states compatibility impact and whether the freeze identifier
      must change.
- [ ] A test may reveal a specification defect but may not silently redefine the
      specification.
- [ ] Implementation-only refactors do not change the freeze manifest.
- [ ] Emergency changes still require retrospective contract review before a
      release candidate.

## Exit condition

Production Stage 2 may begin only when:

- [ ] all seven gates pass;
- [ ] the freeze manifest exists and is reviewed;
- [ ] the initial fixture/oracle bundle is immutable and identified;
- [ ] digest/fingerprint and vocabulary bundle test vectors pass independently;
- [ ] no blocking contract question remains; and
- [ ] the implementation issue tracker links every Stage 2 task to frozen
      requirements and expected evidence.
