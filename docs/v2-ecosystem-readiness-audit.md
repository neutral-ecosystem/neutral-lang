<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v2 ecosystem-readiness audit

**What this repo does:** Neutral compiles host-captured `.neu` descriptions and
data-only vocabularies into independently validated, portable typed artifacts.
Its completed v1 design also provides bounded pure value evaluation, finite
description expansion, public reader interfaces and generic authoring; consumers
decide what those descriptions mean operationally and perform effects.

Date: 13-10-2026. Status: **requirements proposal, not an approved architecture**.

Assumed use: independent teams building headless CI/CD tooling and a host-side
embedded-device application, initially on a developer workstation or controlled
CI runner. Third-party packages, hostile artifacts, disconnected operation,
concurrent requests and real device failures matter. No production fleet size,
latency target or microcontroller memory budget is assumed without measurement.

This audit is a proposed basis for the next stable ecosystem milestone, called
**v2** here. It does not implement EmbeddedHealth, approve its architecture, add
an executor to Neutral, activate a profile, or amend the current v1 checklist.

## Contents

- [Conclusion](#conclusion)
- [Basis and limits of the assessment](#basis-and-limits-of-the-assessment)
- [What completed v1 already supplies](#what-completed-v1-already-supplies)
- [Can it cover Neutral Flow?](#can-it-cover-neutral-flow)
- [Can it cover the embedded application?](#can-it-cover-the-embedded-application)
- [Prioritized missing requirements](#prioritized-missing-requirements)
- [Which changes really belong in the language?](#which-changes-really-belong-in-the-language)
- [End-to-end acceptance corpus](#end-to-end-acceptance-corpus)
- [Delivery order and definition of done](#delivery-order-and-definition-of-done)
- [Decisions not to inherit from the drafts](#decisions-not-to-inherit-from-the-drafts)
- [Evidence and references](#evidence-and-references)

## Conclusion

**Completed v1 is a credible foundation for starting both projects. It is not,
by itself, a complete reusable integration ecosystem.**

There is no demonstrated reason to turn Neutral into a general-purpose language
with mutable state, unrestricted functions, loops, vendor SDK calls or a built-in
scheduler. Most of the requested information can already be expressed as v1
records, variants, references, restrictions, pure expressions and static
composition. Actual CI/CD behavior, device diagnostics and their safety guarantees
cannot be supplied merely by accepting those records.

The missing bridge is concrete: somebody must define operation meanings, acquire
and lock packages, validate mappings, select compatible implementations, authorize
effects, marshal inputs/results and prove adapters honor their contracts. V1
deliberately leaves most of this to consumers. Without shared infrastructure,
each consumer can build it privately, but the ecosystem goal becomes repeated
integration work rather than installable interoperability.

Two narrower **language-extension candidates** also deserve v2 review:

- Typed, bounded projection from structured values inside pure expressions.
  Public reader selectors are not automatically expression-level field access.
- Stronger declarative numeric validation for integral counts, addresses and
  alignment, while keeping `num` and the accepted decimal arithmetic semantics.
  A minimum/maximum restriction alone does not establish integrality.

Neither candidate justifies postponing a simple consumer prototype. A consumer
can perform checked projection and domain validation explicitly. They become
important when reusable typed descriptions must carry these guarantees through
compilation, binding, evaluation and independent reading without custom rules
being rewritten by every application.

The recommended v2 outcome is therefore **a supported language foundation plus
a small, tested integration kit**, not a universal executor or a promise to
support every platform. Its credibility should come from two different consumer
domains and genuinely different implementations of a named portable contract.

### Three different meanings of “enough”

| Question | Assessment under completed v1 |
| --- | --- |
| Can an application describe its configuration and operation graph? | Largely yes, using exact domain vocabularies and ordinary typed data. |
| Can an application implement useful behavior with its own planner and adapters? | Yes. It still has to write that application; Neutral does not execute it. |
| Can independent authors install interoperable packages and reuse mappings safely without changing compiler or consumer internals? | Not established by v1. This is the main v2 ecosystem gap. |
| Does a valid artifact prove a CI provider or MCU can perform the requested behavior? | No. Compatibility, authorization and observed conformance are separate. |
| Will completing v2 make all Flow and embedded features exist automatically? | No. Their runtimes, integrations, hardware tests and interfaces remain separate product work. |

## Basis and limits of the assessment

### The baseline is completed v1, not the current package number

The main comparison assumes every approved task in
[the v1 implementation checklist](../portable/development/07-IMPLEMENTATION-CHECKLIST.md)
has been completed, qualified and activated, including Stage 8 expressions,
evaluator, expansion and authoring. An unchecked implementation task is not
reclassified as a new v2 requirement.

The inspected workspace currently declares package version `0.8.0`. Its standalone
[profile catalogue](../crates/neutral-core/src/profile.rs) still reports source
profile `1.0` unavailable. Explicit composition library APIs already compile
multi-source projects and the standalone probe reads their artifacts; that is not
the same as activating standalone v1 source compilation. The
[compiler README](../crates/neutral-compiler/README.md),
[probe README](../crates/neutral-probe/README.md) and operational checklist explain
this distinction.

The current implementation also has a real
[typed consumer boundary and selectors](../crates/neutral-reader/src/consumer/README.md).
They validate reports, descriptions, member paths and safe origins over decoded
composition projects. Entry discovery, complete consumer compatibility discovery,
expressions, expansion and authoring remain implementation work in the inspected
checklist. This audit assumes that work is finished when assessing completed v1.

Keep these version axes separate:

| Axis | What it selects |
| --- | --- |
| Cargo/application release | A distributed implementation and its support policy. |
| Source profile, such as `neu "1.0"` | An exact source-language behavior. |
| Capture, logical IR and wire profiles | The accepted input and artifact shapes. |
| Expression, evaluator, expansion and arithmetic profiles | Exact computational semantics and required features. |
| Identity profile | A particular canonical transcript and digest meaning. |
| Vocabulary identity/revision | A particular nominal semantic contract. |
| Package, mapping, adapter and protocol versions | Independently evolving distribution and integration contracts. |

Calling the next milestone v2 does **not** automatically require a `neu "2.0"`
header or changing every `/1`, `/2` or `/3` identifier. Ecosystem libraries can
evolve without changing language meaning. New language or wire semantics require
explicit new profiles; historical profiles and vectors remain immutable.

### Inputs were assessed, not adopted

The supplied **EmbeddedHealth — Architecture** and the two **Neutral Mapping
System — Architecture Review** texts are scenario sources. Their proposed crate
names, package layout, `.nmap` extension, agent messages and version numbers are
not treated as accepted contracts. Their examples are not assumed to be valid
Neutral syntax.

The current [Flow requirements](https://github.com/neutral-ecosystem/neutral-roadmap/blob/main/neutral-flow/REQUIREMENTS.md)
and [proposed architecture](https://github.com/neutral-ecosystem/neutral-roadmap/blob/main/neutral-flow/ARCHITECTURE.md)
were read as a second problem corpus, not as authority over language semantics.
Some older descriptions assign source resolution to the compiler or conflate
logical IR identity with derivation identity. Current Neutral contracts instead
put acquisition in the host and distinguish those identities. Integration must
follow that distinction, rather than copying old wording into code.

This is a feature/requirements and ownership audit. It is not a line-by-line
security audit, a fresh conformance run or a claim that the proposed applications
already work. Source inspection supports statements about available surfaces;
the acceptance tests below are **proposed evidence**, not passing results.

## What completed v1 already supplies

An older review saying “missing” does not establish a current language gap.
The following features belong to the completed-v1 baseline.

| Capability | V1 coverage | Important limit |
| --- | --- | --- |
| Immutable structured data | Scalars, nominal records, ordered typed lists, reuse and identity references. | A reference is not a function call, device pointer or scheduling instruction. |
| Modules and public interfaces | Explicit logical imports, private-by-default declarations, public type closure and stable module-symbol identity. | Imports cannot locate files or download packages; the host supplies the complete closure. |
| Vocabulary composition | Both source and vocabulary records/closed variants; nested list/nullable/reference contracts. | Nominal identity and exact revisions matter; similar field shapes do not imply substitutability. |
| Presence and defaults | Required, optional, explicit null and materialized default distinctions with retained origin facts. | Missing runtime output must not be disguised as null or a source default. |
| Declarative restrictions | Finite scalar choices, inclusive numeric bounds and string/list length bounds. | No integrality, alignment, arbitrary predicates, uniqueness or cross-field rules are implied. |
| Cross-vocabulary types | Explicit public dependencies, exact transitive lock closure, diamond handling and conflicting/private/missing/cyclic rejection. | Distribution, version selection and installation happen outside compilation. |
| Portable locations | Distinct inert `url` and `path` data. | They grant no access and establish no content identity or platform path interpretation. |
| Complete artifacts | Complete project IR, companions, resource facts, required contract closure and independently validated binary transport. | A `.nir` is not an executable, firmware image or JSON document. |
| Reader/probe boundary | Compiler-independent validation, public views, typed member selectors, entry discovery and source attribution. | Public visibility does not mean confidentiality; source attribution does not authenticate a producer. |
| Layered identities | Captured, logical, derivation and artifact identities, plus public interface facts. | None proves authorization, publisher trust, hardware identity or execution success. |
| Consumer data | Typed descriptions, symbolic handles, capabilities, requirements and reports. | Generic envelopes do not define a universal operation catalogue or mapping algorithm. |
| Pure expressions | Typed comparisons/Boolean operators, value-producing `if`/exhaustive `match`, arithmetic and string/list construction. | No effects, mutation, ambient inputs or general callable functions. |
| Numeric evaluation | Exact stored decimals; exact bounded terminating division; deterministic 34-significant-digit, ties-to-even nonterminating division. | Rounded intermediates cannot be reassociated as exact algebra or narrowed silently to a device integer. |
| Deferred evaluation | One shared pure evaluator with explicit typed bindings, guarded demands, restrictions and distinct required/unavailable/failure states. | It does not acquire outputs, wait, schedule, authorize or resolve secrets. |
| Reuse and matrices | Hygienic typed compile-time templates and finite static expansion. | Deferred values cannot determine cardinality, generated names or topology. |
| Generic authoring | Descriptor catalogue, separate presentation metadata, revision-bound editable model, source projection and round trips. | An Editor UI, IDE integration and application-specific visual semantics are not delivered merely by the bridge. |
| Bounded processing | Independent limits, cancellation, fallible retention, deterministic diagnostics and atomic publication. | A compiler work budget does not bound device execution time, subprocess output or a consumer's own graph expansion. |

These conclusions come from
[consumer readiness](../portable/specs/contracts/CONSUMER-READINESS.md),
[consumer protocol](../portable/specs/contracts/CONSUMER-PROFILE.md),
[vocabulary composition](../portable/specs/contracts/VOCABULARY-COMPOSITION.md),
[expressions](../portable/specs/contracts/EXPRESSIONS.md),
[evaluator](../portable/specs/contracts/EVALUATOR.md),
[expansion](../portable/specs/contracts/EXPANSION.md) and
[authoring](../portable/specs/contracts/AUTHORING.md).

### What v1 intentionally does not promise

It does not define the meaning of a field called `needs`, an operation called
`flash`, a credential handle, a deployment gate or a diagnostic status. Those
belong to an exact consumer/domain contract. Ordinary typed data can represent
them; compiler acceptance cannot verify their effectful interpretation.

The [consumer profile](../portable/specs/contracts/CONSUMER-PROFILE.md) explicitly
leaves capability taxonomy and mapping interpretation to consumers. Its
`accepted`, `rejected` and `alternative-required` statuses are validation
outcomes, not device health outcomes or workflow execution states.

## Can it cover Neutral Flow?

**Yes for describing and independently reading a bounded portable workflow;
not alone for binding/exporting it faithfully or running a reliable CI system.**
The distinction holds even if every v1 implementation gate passes.

| Flow user outcome | What completed v1 contributes | What remains outside it |
| --- | --- | --- |
| Define a finite graph with fan-out/join | Typed operation values, refs, public entries, exact contract closure. | Flow graph membership, cycle rules, readiness, join and skip semantics. |
| Pass earlier results into later work | Result contracts, typed output selectors, retained availability/restriction obligations. | Association of an operation with its actual result type, result collection and scheduling. |
| Use result-dependent conditions | Pure guarded expressions and explicit deferred bindings. | Flow failure/skip policy and faithful provider lowering or controlled evaluator invocation. |
| Reuse components and finite matrices | Source modules, typed templates and static hygienic expansion. | Package distribution, component compatibility and Flow concurrency/aggregation rules. |
| Run arbitrary project tools | A vocabulary can describe a command/tool invocation as data. | Safe argument handling, tools, environments, isolation, permissions and effects. |
| Trigger from commits, requests or schedules | Typed input declarations and contextual data. | Event ingress, clocks, authentication, deduplication and causal history. |
| Export to different CI providers | Provider-independent IR and separately context-bound reports. | Graph-level mapping, provider schema, limits, semantic loss detection and export provenance. |
| Retry, cancel and recover | Types can represent policy and outcomes; language processing itself is cancellable. | Durable attempts, fencing, external-effect reconciliation and truthful cleanup states. |
| Promote a verified artifact | Records can name digests, evidence and approval subjects. | Retrieval verification, policy, signing authority, protected delivery and retention. |
| Keep secrets out of plans | Symbolic handles and no ambient secret lookup. | Credential broker, authorization, isolation and secret-safe output handling. |
| Explain failures visually | Safe field attribution, stable codes, authoring descriptors and identities. | Mapping-to-provider locations, runtime causation, GUI and domain explanations. |
| Support caching/change-aware work | Stable logical identities and explicit inputs help build keys. | Workload/material identities, tools/environment/policy context and safe reuse rules. |

### A concrete failure that more syntax does not solve

Suppose a workflow compares two strings and conditionally consumes a previous
output. V1 can specify exact typed equality and lazy evaluation. A naive exporter
could translate the expression to a provider's expression string and change its
meaning. GitHub Actions documents case-insensitive string comparison, loose
equality/coercion and string-valued step outputs. Those rules are not Neutral's
exact typed expression rules. See
[GitHub's expression semantics](https://docs.github.com/en/actions/reference/workflows-and-actions/expressions).

The adapter must prove faithful lowering for its supported subset, use a
versioned helper running Neutral's evaluator where appropriate, or reject the
unsupported profile. Shipping a helper is an explicit execution/package
dependency, not a license to change the meaning of an exported workflow.

Likewise, an output mentioned only in an unselected branch is not automatically
a prerequisite. Public views retain all possible interpretive dependencies;
evaluation demands only selected ones. Flow must define how this interacts with
readiness, skips and failed attempts, and each provider adapter must honor that
contract or report incompatibility.

### Flow capabilities that are not language blockers

Durable queues, worker fleets, approvals, deployment strategies, cache stores,
notifications, test reporting, cost analysis, historical replay and predictive
scheduling do not need Neutral keywords. They need Flow-owned contracts and
implementations. A static-export-only Flow release can begin without a
Neutral-owned runtime; it cannot claim runtime recovery or execution portability
from golden exported YAML alone.

Dynamic matrices determined by live outputs exceed v1's static-topology model.
They are a separate future consumer/profile decision, not a prerequisite for
the first useful CI/CD system. A new compilation with explicitly captured inputs
is possible, but is a new artifact/plan, not mutation of the original one.

## Can it cover the embedded application?

**Yes for portable descriptions and typed interpretation on the host. No
language can supply the entire diagnostic product or prove hardware health.**
The draft's features are addressed below without adopting its proposed module,
transport, firmware or UI architecture.

| Embedded user outcome | V1 representation | Missing integration or product responsibility |
| --- | --- | --- |
| Discover devices and connections | Typed device/connection descriptors and availability variants. | USB/serial/probe discovery, permissions, live inventory and identity confidence. |
| Identify MCU, board and security state | Nominal records, exact revisions, optional/unknown states. | Vendor interrogation, board evidence and distinction between session, connection and physical asset. |
| Inspect RAM/PSRAM/flash and partitions | Region/partition records, numbers, tagged properties and lists. | Address-space/width/unit/integrality checks, vendor parsing and authoritative measurements. |
| Execute a portable diagnostic | Operation/argument/result records and explicit requirements. | Stable diagnostic semantics, compatible mappings and adapter implementation. |
| Test controlled memory regions | Typed region/algorithm/preservation requests. | Alignment/overlap rules, exclusive ownership, agent safeguards and preservation evidence. |
| Verify or program firmware | Inert image references and typed requests. | Content verification, target compatibility, consent, secure-boot restrictions and safe programming. |
| Report CPU, clocks, reset and power observations | Typed measurement and evidence records. | Instrumentation, calibration, units, uncertainty and causal limits. |
| Test GPIO, ADC and peripheral buses | Typed operation and prerequisite descriptions. | Board wiring, fixtures, reference signals and target-specific guarantees. |
| Test Wi-Fi/Bluetooth links | Peer/environment requirements and typed results. | Actual peers, radio conditions, measurements and bounded execution. |
| Diagnose physical board components | Structured requirements and observations. | External equipment and defensible interpretation; software alone cannot cover every fault. |
| Reuse SDKs and vendor tools | Data can describe implementation and tool requirements. | Versioned wrappers, safe subprocess invocation, output parsing and tool compatibility. |
| Use an optional diagnostic agent | Protocol/version/capability descriptions as data. | Firmware, framing, request IDs, bounded messages, disconnects, cancellation and recovery. |
| Show progress, logs and live monitoring | Types can describe individual events. | Streaming protocol, rate limits, backpressure, retention and dropped-event reporting. |
| Manage profiles and batch devices | Static templates/matrices, refs and selected public entries. | Inventory snapshots, scheduling, per-device limits and session isolation. |
| Persist, compare and export reports | Typed records, schema versions and linked identities. | Storage, migration, redaction, statistical comparison and reproducible evidence. |
| Offer CLI, desktop and IDE interfaces | Reader APIs and generic authoring/projection. | The applications, inter-process APIs, accessible visualizations and user consent flows. |
| Add a genuinely different MCU family | New data-only vocabularies and exact dependencies, without compiler keywords. | A new adapter and measured operation-level conformance, not just another chip name. |
| Package for Linux, macOS and Windows | Portable core contracts and host-independent artifacts. | Supported host binaries, native tool/driver setup, packaging and platform testing. |

### Examples that must not be flattened into “supported”

- A device supports flash writes, but the current transport or security state
  prevents them. Capability, current availability and authorization differ.
- A memory test succeeds on a reserved scratch region. That does not mean it
  tested all RAM, preserved every byte or proved the board healthy.
- A diagnostic agent disconnects after a write. The operation may have happened;
  timeout is not proof of rollback or safe automatic retry.
- A vendor tool can identify a device only by resetting it. “Inspection” is not
  necessarily passive; that effect needs explicit disclosure and policy.
- A MAC, serial identifier or USB path is supplied. None alone proves a stable,
  authenticated physical identity or authorizes using a prior session's handle.
- A result references a dump or firmware image. A path or URL is not proof of
  its content, permission to retrieve it or permission to publish it.

### Host-side integration versus running Neutral on an MCU

The supplied use cases can start with compilation and evaluation on the host,
and a small device agent implementing native diagnostic operations. They do not
establish a requirement to put the compiler, complete artifact reader or evaluator
inside microcontroller firmware.

If on-device evaluation is later required, it needs its own measured profile:
target architecture, allocation/no-allocation behavior, stack/heap/code size,
numeric cost, wire subset, cancellation model, protocol and security boundary.
The current Rust library implementation and language-level resource limits do
not establish `no_std`, real-time or firmware suitability. Making LLVM a backend
would not solve that integration question by itself.

## Prioritized missing requirements

The numbered findings are proposed requirements, not discovered exploitable
vulnerabilities or claims of failed tests. **Must fix** means required before
claiming the stable, reusable ecosystem outcome described here. It does not mean
every item blocks the first application prototype. **Should fix** items reduce
real integration friction or become mandatory when the stated feature is shipped.

### Must fix — stable integration foundations

#### 1. V2-01 — Publish a supported consumer adoption path

**What this is:** A developer needs one documented route from a small local
project to a complete artifact, independent reader and selected typed entry.
**Problem:** The inspected CLI compiles single-source artifacts; the complete
composition path is a library API, and standalone v1 availability is still gated.
**Fix:** Finish v1 activation and expose a supported host adapter/SDK workflow
with exact profile selection and complete project/vocabulary capture.
**If we skip it:** Each consumer writes custom bootstrap code and users confuse
a recognized source header with an available implementation.

Owner: Neutral toolchain/SDK. Classification: **v1 completion and adoption**, not
a new language feature. Evidence:
[CLI interface](../crates/neutral-cli/README.md),
[composition compilation](../crates/neutral-compiler/README.md) and
[current gates](../portable/development/07-IMPLEMENTATION-CHECKLIST.md).

Acceptance must cover a fresh example outside this workspace: acquire inputs
explicitly, capture a multi-module project, select the exact features, compile,
encode, independently decode, select an exact public contract and report a
field-level error. No tests, private ASTs, `xtask` internals or archived portable
directory may be a runtime dependency. A documented Rust API is a valid first
path; a stable host manifest/CLI should remove hand-written digest/lock arguments
from normal authoring without hiding the resolved facts.

#### 2. V2-02 — Define operation contracts, not merely record shapes

**What this is:** An operation contract states observable promises: accepted
inputs, outputs, prerequisites, effects, failure meanings and guarantees.
**Problem:** Two adapters can accept the same record and both call their work
“verify,” while one checks a region and the other merely checks that a tool ran.
**Fix:** Publish a small domain-owned contract catalogue plus a shared contract
reference/description convention, validated independently of executable adapters.
**If we skip it:** Portable type compatibility will be mistaken for portable
behavior and incorrect implementations will look interchangeable.

Owner: domain vocabulary maintainers; shared envelope/conformance tooling in the
ecosystem. V1's [consumer envelope](../portable/specs/contracts/CONSUMER-PROFILE.md)
is transport and structural validation, not this behavioural specification.

Contracts need exact input/result nominal identities, applicability, required
guarantees versus preferences, preconditions, observable postconditions, side
effects, failure categories and evidence obligations. Cancellation, retry and
cleanup guarantees must be qualified, not implied by a method name. Some rules
will remain prose plus conformance tests; do not pretend schema validation can
prove arbitrary physical or remote behavior. Governance must assign an owner,
review process, deprecation policy and conflict procedure to each contract.

Acceptance: a flash verifier that does not read/verify the promised bytes and an
exporter that weakens a required isolation guarantee both fail the relevant
domain suite even if their result records are structurally valid.

#### 3. V2-03 — Establish publisher-qualified package identity and compatibility

**What this is:** Independent publishers need unambiguous package/contract
references and rules for upgrading them.
**Problem:** A source-friendly name such as `Embedded` or a dotted prefix does
not identify a trusted publisher, imply inheritance or establish compatibility.
**Fix:** Define host-side distribution identities and their explicit association
with exact Neutral vocabulary identities; validate namespace ownership and revisions.
**If we skip it:** Independently developed packages collide or users bind the
wrong contract because names happen to look related.

Owner: package ecosystem. Do not silently broaden the frozen vocabulary identity
grammar. If the existing semantic identity space cannot represent the required
independent ownership, specify a new profile and migration rather than repurposing
old strings. WIT is a useful precedent for separating namespace, package and
version, not a mandate to copy its type or execution model. See
[WIT package names](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md#package-names).

Keep distribution version, semantic vocabulary revision, mapping revision and
adapter protocol distinct. Package ranges may be convenient during explicit
resolution; compilation still receives exact resolved content and revisions.
V1 permits one revision per canonical vocabulary identity in a project. Either
retain that rule with clear conflict diagnostics or propose separately reviewed
multi-version semantics; do not introduce accidental structural coercion.

Acceptance: two publishers' similarly named packages cannot be confused;
diamond dependencies deduplicate correctly; conflicting contract revisions give
an actionable dependency path. A vendor namespace does not automatically claim
every contract in its broad domain.

#### 4. V2-04 — Provide an explicit, offline-capable package/capture workflow

**What this is:** Hosts acquire package contents and construct the exact inputs
Neutral already requires.
**Problem:** Exact captured locks exist in v1, but they do not discover, download,
install or safely update reusable source/vocabulary/mapping distributions.
**Fix:** Start with local packages and a manifest/lock workflow, separating
data installation from executable adapter installation and activation.
**If we skip it:** Every application invents a package loader and reproducibility
depends on manual file and lock maintenance.

Owner: host package tooling, outside the no-I/O compiler. A lock should identify
the complete resolved closure, content digests, package sources, exported logical
modules/vocabularies and relevant feature requirements. Installation is explicit;
importing a type cannot run a post-install script, load a plugin or acquire code.
Semantic-only users must not need vendor binaries or credentials.

Start with a reviewed directory/archive format and local content store. Validate
archive paths, symlinks, sizes, file counts and extraction work before activation;
reject traversal, duplicate entries, digest mismatch and partial installation.
Support frozen-lock operation with no network. A registry is optional at first.
For network updates, a checksum is not publisher authentication or rollback/freeze
protection; use an established trust/update design rather than inventing signature
semantics. See [TUF's threat model](https://theupdateframework.io/docs/security/).

Acceptance: a clean disconnected machine reconstructs identical captured inputs
from the lock and supplied packages. Malformed packages cannot escape installation
roots or activate an executable. Updates show changed contracts and require an
explicit new lock/capture.

#### 5. V2-05 — Specify a small typed mapping contract

**What this is:** A mapping connects an exact semantic operation or domain plan
to a concrete implementation without redefining the portable source.
**Problem:** V1 can carry mapping descriptions as data, but does not define how
to validate a binding or prove its argument/result transformations preserve contracts.
**Fix:** Build one shared mapping model for direct bindings and bounded pure
transformations, with domain-specific validators for graph or stateful planning.
**If we skip it:** Consumers duplicate incompatible mapping formats or hide
behavior-changing conversions inside opaque adapters.

Owner: a separately versioned mapping library/contract. No permanent file suffix,
crate count or repository split is required yet. Begin with one format that can
be inspected without starting its adapter; standard serialization is preferable
to an invented parser until the data model is proven.

Retain exact source/target contracts, mapping/content identity, adapter/protocol
requirements, applicability, required capabilities, effects, input/output
transformations, and diagnostic/source links. Transformations must preserve types,
presence, units, restrictions and failure states. They must not silently insert
defaults, round addresses, stringify arbitrary structures, drop fields or weaken
guarantees. Reuse the shared pure evaluator for its supported semantics instead
of embedding shell, JavaScript or a second expression language in mapping strings.

Direct mapping is not enough for whole-DAG CI export. Allow a Flow-owned lowering
implementation behind a declared contract. Composite mappings, if needed, require
finite expansion, aggregate bounds, cycle rejection, effect aggregation and
failure/cleanup semantics; a sequence is not automatically a transaction.

Acceptance: wrong result types, missing outputs, invalid conversions, undeclared
effects and recursive/unbounded mapping composition are rejected before dispatch
or publishing an accepted export.

#### 6. V2-06 — Make resolution deterministic and explainable

**What this is:** Given a request and an explicit target/candidate snapshot, a
resolver chooses a valid implementation or explains why it cannot.
**Problem:** Selecting the first installed adapter can ignore a required guarantee,
hide ambiguity or use a stale device/provider claim.
**Fix:** Specify bounded matching over captured facts, exact contracts, explicit
selection policy and domain validators; reject unresolved material ambiguity.
**If we skip it:** Installation order and live environment drift become hidden
inputs to behavior.

Owner: shared matching machinery plus consumer-specific interpretation. Discovery
and device/provider probing are effectful host operations. The deterministic
resolver accepts their snapshots; it does not secretly perform discovery.

Represent capability, availability, prerequisites, qualifiers, quantitative limits,
claim issuer/evidence and freshness separately. Technical compatibility does not
mean permission. Unknown or insufficient evidence must be explicit; the current
three-status consumer report may carry a domain reason, but its meaning must not
be silently extended. Required guarantees reject if unmet. Preference loss is
visible, and a behavior-changing alternative requires explicit approval and a
new decision/context.

Acceptance: shuffled installation order produces the same decision; equal
material candidates produce ambiguity; stale/disconnected/wrong-security-state
targets cannot reuse a previously accepted decision without the host's specified
refresh and authorization checks.

#### 7. V2-07 — Bind plan identity to all decision-affecting context

**What this is:** A target-bound plan is an immutable consumer record, separate
from complete Neutral logical identity.
**Problem:** A key containing only project identity or an adapter version can
reuse a decision after target facts, selected entry or external values change.
**Fix:** Define a canonical versioned plan/decision projection binding all
relevant inputs, with typed identities and explicit invalidation.
**If we skip it:** Stale plans and cache hits can select the wrong implementation
or conceal why a protected action was approved.

Owner: consumer planning and shared integration helpers. V1 already requires
this separation; v2 must supply usable conventions and tests, not replace the
[project identity contracts](../portable/specs/contracts/PROJECT-IDENTITY.md).

Bind selected entries, exact operation/vocabulary and mapping identities, planner
behavior, adapter binary/protocol, target/capability snapshot, non-secret binding
generation, evaluator profiles and decision-affecting policy. If time or randomness
affects a decision, the host must capture the relevant input explicitly. Record
authorization separately with its validity and subject; a plan digest grants no
authority. Do not put raw secrets or publicly correlatable hashes of low-entropy
secrets in plan keys. Use opaque protected generation references where appropriate.

Acceptance: root, target, mapping, relevant binding or behavior-version changes
invalidate the appropriate consumer decision while complete project identity
stays unchanged. Rejected/cancelled requests cannot replace a successful cache
generation. Export byte identity remains separate from plan meaning.

#### 8. V2-08 — Enforce effects, safety and credentials outside descriptors

**What this is:** The component performing effects must authorize the exact
operation against fresh target and actor facts.
**Problem:** A data field saying `read_only` or a signed adapter package cannot
stop native code from resetting hardware, exposing credentials or writing files.
**Fix:** Define a narrow host authority boundary, effect declarations and policy
checks, with real process/platform enforcement appropriate to the claimed risk.
**If we skip it:** A well-typed profile can become an unintended privileged action.

Owner: application runtime/adapter host. Shared effect and permission descriptors
can help, but Neutral's compiler must not become a policy engine or secret broker.
Treat compatibility, authorization, consent, execution and observation as distinct
facts. Dynamic safety facts require rechecking before effect admission.

Embedded cases include reset/halt/write/erase/irreversible effects, exclusive
memory regions and conditional backup/restoration. Flow cases include protected
artifact promotion, untrusted repository code and destination-scoped credentials.
Handles must identify scope/session and cannot be reused as ambient authority.
Bound execution must not forward arbitrary bearer tokens through general plans,
queues or logs. Separating a plugin into a process helps protocol/failure control;
it is not itself a filesystem, network or privilege sandbox.

Acceptance: unsafe actions are refused through the engine even if a UI/profile
claims consent; a stale device session or changed protected target refuses action;
undeclared effects cannot be silently accepted. Tests must verify the actual
enforcement boundary, not just that a descriptor contains a permission list.

#### 9. V2-09 — Supply a versioned adapter protocol and a small SDK

**What this is:** Independently released adapters need discovery, negotiation,
invocation and failure handling that do not depend on compiler internals.
**Problem:** Ad-hoc Rust trait implementations work within one build but do not
define a safe, cross-language, independently distributed plugin boundary.
**Fix:** Begin with an explicitly launched, versioned out-of-process protocol
and a reference adapter SDK; keep built-in trusted adapters possible.
**If we skip it:** Every application designs another plugin lifecycle and vendor
integration leaks into core APIs.

Owner: integration SDK and host. Define handshake/protocol/features, exact contract
advertisements, request/attempt IDs, bounded payloads, progress/result separation,
timeouts, cancellation acknowledgements, cleanup/disconnect outcomes and structured
errors. Planning-only adapters must not receive execution credentials. No adapter
may bypass independent type/result validation by returning a “validated” label.

Do not promise a stable arbitrary Rust dynamic-library ABI. Rust only guarantees
the documented layout/ABI properties; the default representation is not a durable
plugin serialization contract. See the
[Rust representation rules](https://doc.rust-lang.org/reference/type-layout.html#the-rust-representation).
Terraform's separate provider processes are a useful interface precedent, not a
reason to copy its resource/state model. See
[Terraform's plugin boundary](https://developer.hashicorp.com/terraform/plugin/how-terraform-works).

Acceptance: a deliberately separate adapter process can negotiate, handle a
request and fail safely on unknown versions, malformed messages, output overflow,
crash and cancellation races. No fallback runs a failed untrusted adapter with
broader in-process privileges.

#### 10. V2-10 — Define runtime value/result and event transport boundaries

**What this is:** Runtime values, progress and outcomes must be validated before
they become evaluator bindings or authoritative application results.
**Problem:** V1 defines evaluator states and consumer validation reports, but
neither is a universal device/result/stream protocol or a runtime value codec.
**Fix:** Provide a shared checked value transport/validation surface, with
domain-owned lifecycle/result schemas and bounded streaming envelopes when needed.
**If we skip it:** Adapters can smuggle mistyped results into evaluation or turn
interrupted/unsupported work into an apparent success.

Owner: language value/reader SDK for structural checks; consumers and adapter
protocols for lifecycle and transport. Do not reinterpret `ConsumerStatus::Accepted`
as workflow success or a diagnostic `PASS`.

Preserve exact nominal revisions, decimal values, restrictions, presence, closed
variant tags, and output-to-operation association. Specify bindings for externally
produced records and references: a host-created result must not need a fake
source declaration, private IR constructor or whole-project recompilation. Ref
targets remain validated project identities or explicitly separate domain handles,
never arbitrary foreign pointers. JSON numbers cannot silently force a large
integer through a lossy host number representation.

For streams, define correlation, sequence, aggregate limits, rate/backpressure,
dropped-event indicators and a terminal outcome separate from progress. A consumer
may report partial observations after an interrupted effect; that is not partial
successful compilation or a partially accepted evaluator value. The domain must
distinguish lifecycle, outcome, failure reason, tested coverage and cleanup/effect
disposition rather than collapse them into one status string.

Acceptance: malformed/oversized/wrong-revision results fail before binding;
unavailable, missing, explicit null, failed and cancelled remain distinct. A
disconnect after a flash write reports uncertainty and cannot trigger an automatic
retry based only on timeout.

#### 11. V2-11 — Preserve numeric, unit and address fidelity

**What this is:** Device addresses, byte lengths, retry counts, durations and
measurements have requirements beyond “is a number.”
**Problem:** V1 numeric bounds allow `1.5` inside an integer range; approximate
division and unchecked host casts are unsafe ways to derive addresses or counts.
**Fix:** Keep `num`, require checked integral/range/unit conversion in the SDK,
and review a minimal declarative integral/alignment refinement extension.
**If we skip it:** A valid numeric value may be truncated, use the wrong units or
refer to the wrong address space at a safety-critical boundary.

Owner: shared value/domain validation first; language/vocabulary profile only for
new general refinements. The
[existing restriction set](../portable/specs/contracts/VOCABULARY-COMPOSITION.md#restrictions)
does not include integrality or multiples. A record named `UInt64` is not proof
that its `num` member is an integer. Domain wrappers can prevent accidental nominal
mixing, but their constraints must still be enforced.

Require exact conversion with no rounding or saturation for indices, sizes and
register values; reject fractions, negatives where forbidden and overflow.
Represent address-space identity, address width, units and alignment explicitly;
two equal numbers in different device address spaces are not interchangeable.
Keep measurement uncertainty/calibration separate from the decimal arithmetic
context. Endianness and wire integer widths belong to adapters/protocol codecs.

Portable domain validators must also handle region end overflow, containment,
non-overlap and reserved-region rules. These are relational/device facts; do not
put arbitrary executable validation callbacks in semantic bundles. Introduce
only proven common refinements under new profiles with reader/evaluator/oracle
coverage. V1's frozen arithmetic remains unchanged.

Acceptance: a value larger than a host's exact floating-point integer range
round-trips exactly; `1.5` as a byte count fails; misaligned and overflowed regions
fail; a rounded calculation is never silently narrowed to an address.

#### 12. V2-12 — Prove interoperability with consumer and adapter conformance

**What this is:** Stable ecosystem claims need tests of observable integration
behavior, not only compiler coverage or similarly shaped output.
**Problem:** A simulator or one vendor adapter cannot establish portability,
honest hardware results or faithful provider export.
**Fix:** Publish a shared harness, domain-owned contract suites and independent
second implementations, using the acceptance corpus below.
**If we skip it:** V2 becomes an architectural claim that fails when the second
real project or provider arrives.

Owner: Neutral integration tests plus each consumer/domain maintainer. Existing
[composition tests](../crates/neutral-test-suite/tests/composition_contract/pipeline/mod.rs)
and independent identity oracles are useful foundations, not evidence of effectful
adapter correctness. A language suite must not need a connected MCU or cloud
account; integration/HIL suites are explicitly separate.

Use two proof axes: two different domains expose accidental shared-framework
assumptions; two genuinely different target implementations test each portability
claim. Flow needs a small workflow on structurally different providers, including
unsupported cases. Embedded needs one portable operation on different MCU families,
with controlled evidence and honest limitations. Real hardware is necessary for
hardware claims, but destructive testing must be opt-in and policy controlled.

Acceptance includes negative conformance: a lying capability, weaker guarantee,
wrong result, expired session, lost acknowledgement and changed target must not
become success. Every public support claim names the contract/profile, adapter,
target/tool/protocol versions and evidence scope it actually tested.

### Should fix — full-product support and recurring integration friction

#### 13. V2-13 — Add bounded structured-value projection to pure expressions

**What this is:** Reusable descriptions need to read members of typed parameter
records, inspect optional values and build values from structured inputs.
**Problem:** V1 specifies public reader selectors and an output-selector intrinsic,
but its expression grammar does not define general record/list member extraction.
**Fix:** Review a small typed projection/presence model integrated with the one
pure evaluator, preserving exact types, visibility and guarded availability.
**If we skip it:** Authors flatten reusable inputs into many scalar parameters or
move otherwise pure transformations into every consumer's custom code.

Owner: language, evaluator and authoring; **a genuine post-v1 candidate**, not
permission to reinterpret the frozen expression model. Evidence:
[expression forms](../portable/specs/contracts/EXPRESSIONS.md#source-forms-and-precedence)
and [selector behavior](../portable/specs/contracts/CONSUMER-PROFILE.md#member-selectors--v1-cons-007).

Concrete case: a template accepts a `BuildOptions` record and must compute a label
from its `platform` member. Matching variants exposes their payload, but does not
establish general member access inside that payload. A consumer-side reader call
does not supply source syntax or evaluator semantics for this operation.

Start with typed record-field access, explicit presence/null handling, bounded
list access and `length` only where examples prove they are needed. Define missing,
wrong-tag and out-of-range results explicitly, without implicit ref dereference or
dynamic string paths. Key/value lists remain sufficient for basic map-like data;
lookup/duplicate rules can stay in a reusable domain contract. Bounded map/filter
or a native map type needs separate evidence, not automatic inclusion of a
general function system. Any new operation must cross source, IR/wire, reader,
identities, evaluator, authoring and independent vectors together.

Acceptance: the same structured-input example folds statically and evaluates with
explicit deferred bindings identically; skipped projections do not demand absent
values; exact/one-over work/output limits and hostile paths fail boundedly.

#### 14. V2-14 — Make non-Rust consumption and codecs a supported surface

**What this is:** Desktop TypeScript, agent C/C++ and other integrations should
consume stable contracts without implementing private Rust data layouts.
**Problem:** A binary `.nir`, a public Rust API and a human-readable probe JSON
projection are three different things, not interchangeable SDKs.
**Fix:** Choose a supported host protocol or bindings for the first real consumer;
publish exact value/diagnostic schemas and compatibility fixtures.
**If we skip it:** Each frontend writes a partial decoder or treats diagnostic
display strings as an authoritative API.

Owner: SDK/host tooling. The
[probe documentation](../crates/neutral-probe/README.md) explicitly distinguishes
inspection JSON from IR transport. Keep that distinction. A UI can use a bounded
local host protocol instead of decoding the complete artifact itself; an agent
usually needs only its domain operation protocol, not all Neutral IR.

Prioritize a Rust SDK and one real non-Rust boundary. Generate schemas/types from
one reviewed model where practical and test them against hostile/literal cases.
Do not generate five SDKs before one is used. Preserve large exact numbers,
unknown-required-feature rejection, stable diagnostic codes and safe attribution
across language boundaries.

Acceptance: a separate non-Rust client can discover compatibility, submit a
bounded request and inspect a typed result without parsing log lines or importing
compiler internals. Unsupported versions fail before accepted data is exposed.

#### 15. V2-15 — Standardize artifact/evidence references, not giant IR payloads

**What this is:** CI artifacts, firmware images, flash dumps and long logs have
content, confidentiality and retention properties separate from their locations.
**Problem:** Inert paths/URLs or huge lists of numbers do not provide safe binary
transport, content verification, bounded streaming or evidence authenticity.
**Fix:** Use a small shared artifact-descriptor convention and host-side verified
content handling; keep large bytes outside ordinary semantic IR by default.
**If we skip it:** Plans bind mutable locations or grow without useful bounds,
and “hash matches” is mistaken for trusted evidence.

Owner: ecosystem data conventions and consumer artifact stores. A descriptor needs
an algorithm-qualified content digest, byte length, media type and optional
retrieval locations, with confidentiality/access and retention handled separately.
Firmware adds target/format compatibility; memory dumps add sensitive-data policy.
Records/lists can represent this today; a new built-in `bytes` type is not required
until small inline-binary scenarios justify it.

Verify retrieved content before trusted use, enforce decompressed-size limits,
and distinguish missing, expired, inaccessible and digest-mismatched content.
An identity or signature is not authorization and does not prove a measurement
was truthful. Reuse established descriptor/evidence formats where appropriate;
the [OCI descriptor model](https://github.com/opencontainers/image-spec/blob/main/descriptor.md)
is a useful precedent for content versus location.

Acceptance: promoting an artifact or flashing an image cannot substitute bytes
at the same locator; a deleted payload remains a clearly unavailable historical
reference, not silently empty data. Large logs/dumps do not require whole-buffer
retention in the compiler or UI.

#### 16. V2-16 — Make schema evolution and support commitments executable

**What this is:** Independent language, package, mapping and adapter releases
need a supported compatibility matrix and explicit migrations.
**Problem:** Exact nominal revisions and closed variants correctly reject unknown
meaning, but “minor version update” alone cannot make changed contracts compatible.
**Fix:** Add contract-diff tooling, explicit migration paths and tested old/new
producer-reader-consumer combinations around v1's existing negotiation rules.
**If we skip it:** Installation succeeds while projects, stored reports or
providers become unusable without a clear diagnosis.

Owner: language release tooling plus package/domain maintainers. Freeze how added
required fields, changed defaults/restrictions, new variant tags, removed exports
and altered operation guarantees are classified. A new alternative can break an
exhaustive match. A tighter constraint can reject a previously valid deferred
result. Content pins and nominal revisions must remain explicit after migration.

Distinguish source migration, artifact conversion, runtime report migration and
provider binding changes. Each may have different safety and loss rules; none
silently rewrites historical evidence. State which profile combinations are
supported and for how long. Keep package installation metadata outside semantics
unless it actually affects project meaning.

Acceptance: a deliberately breaking bundle update is detected before use; an
explicit migration creates new identities and reports losses; retained old
artifacts remain readable under the stated policy. Unknown required features
never downgrade to a best-effort interpretation.

#### 17. V2-17 — Carry safe diagnostics through every transformation

**What this is:** A user needs to trace a provider, mapping or device rejection
back to the requested operation and relevant field without leaking private data.
**Problem:** V1 source attribution ends at its governed boundaries; generated
provider files, package sources and runtime observations add new transformations.
**Fix:** Compose bounded diagnostic mappings using stable public subjects and
explicit unavailable/redacted states, keeping execution evidence separate.
**If we skip it:** Users receive tool text or the wrong source span, and reports
can leak private source, device identifiers or credentials.

Owner: mapping/consumer SDK and application reporting. Preserve the distinction
between where a value was supplied, where its default contract lives, where it
was transformed and where a runtime failure occurred. A generated provider line
is not an original source occurrence. Host paths are private acquisition state,
not portable identity. No hop may invent a precise span it did not retain.

Public visibility is not a secrecy mechanism: exported values can contain text
reused from private declarations. Redacting the private origin does not make the
public value confidential. Domain schemas need safe handle/evidence conventions;
logging and raw export require separate policy. Do not claim automatic secret
detection in arbitrary strings or perfect masking after transformation.

Acceptance: a rejected converted argument maps to its safe source field and exact
mapping contract; private reuse origins stay redacted; missing evidence stays
unavailable; diagnostics/logs do not expose local paths or raw sensitive values.

#### 18. V2-18 — Support explicit configuration composition and realistic budgets

**What this is:** Applications combine author requests, package defaults,
environment bindings, protected policy and live observations at different phases.
**Problem:** Treating them as one last-write-wins dictionary hides authority and
can change behavior without a new plan or valid source attribution.
**Fix:** Publish a typed host workflow with explicit composition/precedence,
validation checkpoints and budgets for each owned phase.
**If we skip it:** Users rely on hidden overrides and a bounded compiler is
surrounded by unbounded package, mapping, subprocess or report work.

Owner: host/domain tooling. No automatic source overlay, inheritance or merge
syntax is required. Existing source/template composition can remain explicit.
Policy may constrain or refuse a request; a behavior-changing rewrite must be an
inspectable new input/plan, not silent mutation. Retain the origin and authority
of each effective value without copying credentials into evidence.

Measure package extraction/closure, mapping candidate count and expansion, plan
graph size, adapter message/output rates, subprocess/device time, history and
artifact retention separately. Establish release targets only from named workloads
and runners. Nested operations share aggregate work budgets; resetting a local
limit must not bypass the overall bound. Support Linux first if that is the real
product promise; claim macOS/Windows only after host/tool/driver tests exist.

Acceptance: conflicting defaults/bindings/policy produce a deterministic result
or explicit rejection; effective changes create the correct new context. A
slow adapter, huge tool output, decompression bomb or nested mapping expansion
fails within the owning bound without corrupting the last accepted state.

## Which changes really belong in the language?

The rule is: first identify the information or invariant a consumer needs, then
ask whether ordinary v1 data plus a public validator already expresses it.
Do not turn every application requirement into a compiler primitive.

| Candidate | Recommended disposition | Reason |
| --- | --- | --- |
| Pure conditions, exact/rounded arithmetic, templates, finite matrices | Finish v1; do not reschedule as missing v2 features. | Already approved and frozen in v1's extension contracts. |
| Typed entry discovery, selectors, safe origins, compatibility discovery | Finish v1 and expose a usable SDK. | Existing contracts; full availability remains gated. |
| Structured expression projection/presence | Review for v2 with concrete templates and deferred inputs. | Reader selection does not supply evaluator/source operations. |
| Hybrid source authoring: typed tables, explicit variants, named graph bindings | Review the [syntax proposal](source-syntax-proposal.md) with cross-domain examples and a versioned authoring/source amendment. | Readability is valuable, but nested tables alone do not model shared graphs or supply expression projection and package interoperability. |
| Integral/multiple-of refinements | Checked SDK/domain validation immediately; minimal general extension if adopted. | Bounds alone cannot ensure safe integer/alignment semantics. |
| Units, durations, addresses, digests and artifact descriptors | Shared nominal domain vocabulary and checked codecs first. | Their meaning is domain-specific; no new scalar is automatically necessary. |
| Cross-field validation, region overlap, graph cycles | Reusable domain validators; only proven general declarative rules enter core. | These relationships are not established by per-field schema validation. |
| Native map type, generic records, re-exports, broad collection functions | Defer until real packages show that wrappers/entry lists/explicit modules fail. | Useful ergonomics are not yet demonstrated blockers. |
| Arbitrary runtime functions, mutation, `while`, SDK calls | Do not add to the current core purpose. | They mix portable intent, evaluation and effects and undermine bounded reasoning. |
| Dynamic graph creation from live results | Separate consumer/profile decision after static use cases work. | Changes planning, bounds, identity, authorization and recovery. |
| Plugin callbacks in vocabularies or metadata | Reject. | Installation/validation must remain inspectable and non-executing. |
| Native code generation, LLVM or an MCU interpreter | Separate measured requirement, not a v2 prerequisite. | Mapping/protocol/safety gaps are not compiler-backend problems. |
| A universal operation executor | Do not make it mandatory. | CI providers and diagnostic devices have materially different lifecycle and authority models. |

### Minimum logical boundaries

These are responsibilities, **not a directive to create one crate or repository
per row**. Start with the fewest components that enforce them.

| Boundary | Owns | Must not silently own |
| --- | --- | --- |
| Neutral language and evaluator | Types, pure semantics, complete IR, independent validation, identities and authoring. | Package acquisition, provider calls, policy or scheduling. |
| Host package/capture tooling | Explicit acquisition, exact locks, safe installation and complete supplied closures. | Running code because a semantic import was encountered. |
| Shared mapping library | Typed descriptors, bounded matching helpers, pure conversions and explainable decisions. | Proving hardware truth or authorizing effects. |
| Domain vocabulary and validator | Operation meanings, relationships, invariants, failure/evidence semantics. | Replacing the language parser or weakening nominal type rules. |
| Consumer planner/exporter | Domain plans, graph interpretation, target-specific lowering and plan context. | Mutating immutable Neutral meaning or claiming untested backend equivalence. |
| Adapter host/application runtime | Effects, credentials, session lifecycle, retries, cancellation and cleanup. | Treating a compatible descriptor as authority. |
| Adapter implementation | Concrete tool/API/device protocol and observations. | Defining portable meaning unilaterally or returning unchecked values as trusted. |

The resulting route is intentionally small:

```text
Explicit host acquisition + exact locks
                    |
              .neu compilation
                    |
        immutable, validated Neutral artifact
                    |
          independent reader + pure evaluator
                    |
             domain validation/planning
                    |
       captured mapping/target compatibility decision
                    |
            immutable consumer bound plan
                    |
        separate fresh authorization/effect admission
                    |
         adapter or provider export -> observations
```

The evaluator can be used during planning and later with explicitly supplied
bindings. It has no connection that acquires live values. Static export may stop
before effect admission; execution is not mandatory for every consumer.

## End-to-end acceptance corpus

These are proposed test requirements, **not implemented fixtures or approvals**.
Promote them into a machine-readable corpus only when scope and contracts are
reviewed. Inputs should be file-backed `.neu` and captured vocabulary/package
data, with literal expected types, reports, plan projections and identities.

| ID | Scenario | Required evidence and owner |
| --- | --- | --- |
| ADOPT-01 | New external project uses two modules and a public vocabulary. | Supported SDK/host path produces an artifact a separate reader selects and validates; no workspace-private imports. Neutral tooling. |
| PKG-01 | Reconstruct locked inputs offline. | Identical captured closure; missing/conflicting/modified packages fail before accepted capture. Package tooling. |
| PKG-02 | Install a malicious or interrupted archive. | Traversal, symlink escape, duplicate entries, oversized expansion and partial activation fail safely. Installer. |
| MAP-01 | Same operation, two independently developed direct adapters. | Exact typed results and named observable guarantees agree; shuffled installation order is irrelevant. Shared mapping/domain suite. |
| MAP-02 | Two candidates remain equally suitable. | Material ambiguity is explicit; no first-installed choice. Resolver. |
| MAP-03 | Adapter offers a weaker alternative. | Required behavior rejects; explicit approval produces a new revalidated context rather than silent replacement. Consumer. |
| MAP-04 | Capability changes after planning. | Stale decision cannot authorize dispatch; refreshed incompatible facts refuse action. Host/runtime. |
| FLOW-01 | Finite fan-out/join workflow exported to two different providers. | Deterministic graph-level binding, exact supported profile, provider limits and source-linked unsupported reports. Flow adapters. |
| FLOW-02 | A conditional references an unavailable output only on the skipped branch. | Shared evaluation demands only the guard/selected path; provider export preserves those semantics or rejects. Evaluator and Flow. |
| FLOW-03 | Provider equality/number rules differ from Neutral. | Faithful lowering/helper or explicit incompatibility; no raw backend-expression substitution. Flow exporter. |
| FLOW-04 | An operation loses acknowledgement after an external effect. | No assumed safe retry or exactly-once claim; attempt and reconciliation evidence remain distinct. Flow execution profile, if shipped. |
| FLOW-05 | Promote bytes that were previously checked and approved. | Digest substitution, stale policy or wrong scope refuses delivery; no rebuild masquerades as the approved artifact. Protected delivery. |
| EMB-01 | Same inspection contract on two different MCU families. | Comparable normalized results with actual unsupported/setup/unknown states and named board/tool/security matrix. Embedded adapters and HIL. |
| EMB-02 | Memory test requests fractional, overlapping or misaligned regions. | Checked numeric/domain rejection before hardware access; addresses retain their space/width/unit. Domain validator. |
| EMB-03 | “Read-only” inspection needs reset or debug halt. | Effect is disclosed and independently authorized; profile/UI cannot suppress it. Embedded engine. |
| EMB-04 | Agent disconnects during a write or cancellation. | Result preserves partial observation, uncertain effect and cleanup state; never hardware `PASS` or automatic restoration claim. Agent/host protocol. |
| EMB-05 | Live inventory changes between two batch sessions. | Explicit input/session generations prevent device-handle leakage; finite planning stays bounded. Embedded runtime. |
| VALUE-01 | External result has a wrong nominal revision, tag, ref target or restriction. | Public value boundary rejects before evaluator binding or dispatch. Reader/value SDK. |
| VALUE-02 | A large exact integer crosses Rust and a non-Rust client. | Decimal and range fidelity is preserved; no floating-point detour. SDK codecs. |
| EXPR-01 | Structured input projection is static, deferred, absent or skipped. | Independently checked semantics and limits under a new approved profile, if V2-13 is accepted. Language/evaluator. |
| VIEW-01 | Public value reuses private implementation data. | Public meaning remains inspectable, private origin stays redacted, and confidentiality is not inferred from visibility. Reader and consumer. |
| DIAG-01 | Mapping failure points into generated provider material. | Safe chain back to original request or explicit unavailable attribution; no invented spans or raw local paths. Mapping/reporting SDK. |
| COMPAT-01 | Breaking default/constraint/variant or feature update. | Explicit compatibility failure or reviewed migration with new identities; historical artifacts/vectors unchanged. Release/domain tooling. |
| LIMIT-01 | Hostile package, nested mapping, result flood or allocation failure. | Exact/one-over controls, cancellation and atomic recovery at every owned boundary, not just compilation. Each owner. |

### What counts as independent proof

Two MCU models using the same unchanged vendor stack are useful coverage, but
weak evidence of multi-vendor abstraction. Two CI wrappers over the same substrate
are similarly weak. Choose a second implementation that challenges the shared
assumptions, and include deliberately incompatible examples.

Compiler/reader/evaluator semantic vectors should continue to use an independent
oracle, not compute expected results by calling the production implementation.
Adapter conformance compares observable promises, not JSON field order alone.
Hardware evidence records the tested region, algorithm, prerequisites and limits;
successful static export records only export fidelity unless controlled provider
execution has also been tested.

Keep the no-effect language test suite runnable offline. Hardware, vendor-tool and
provider tests have explicit dependencies and permissions; they cannot silently
skip and still establish an approved support claim. Simulator passes are useful
but cannot substitute for the live evidence a claim requires.

## Delivery order and definition of done

### A. Finish and qualify the existing v1 foundation

Close the current Stage 8/9 implementation and activation gates. Reconcile status
summaries before publishing support: several contract introductions still describe
earlier partial implementation or pending review while the operational checklist
records later progress. Do not edit frozen semantic text opportunistically; update
the authoritative availability/status material and preserve approved byte pins
through the repository's review process.

Deliver one supported external-project example and exact profile discovery.
This is the prerequisite, not evidence that packages or mappings already exist.

### B. Build the smallest no-effect integration kit

Use local packages, exact locks, one direct mapping descriptor, one deterministic
resolver and a fake adapter. Validate all descriptors and transformations without
executing the adapter. Keep package sources, mapping metadata and target facts
explicit. Do not start with a global registry, seven crates, a daemon or cloud
infrastructure.

Exit: ADOPT-01, PKG-01/02, MAP-01/02/03 and the relevant value/limit cases pass
through supported public boundaries. New integrations require package/adapter
registration, not compiler source edits or hardcoded vendor branches.

### C. Challenge the kit with Flow and embedded vertical slices

For Flow, use a small static graph and one learning exporter, then a structurally
different target. For embedded, start with identification/inspection and a fake
device, then real read-only operations on two different families. Test safety
refusals before adding opt-in destructive diagnostics.

Use these cases to decide the minimal shared contract and whether V2-11/V2-13
require new language profiles. Do not stabilize a broad mapping abstraction solely
from two implementations copied from one another. Consumers can develop these
prototypes before a final v2 release; early feedback is part of the evidence.

Exit: the same source/meaning remains unchanged across target binding; valid
differences and incompatibilities are explicit, source-linked and reproducible.
No claim extends beyond the tested contract/targets.

### D. Add only the proven language and transport extensions

If accepted, review numeric refinements and structured expression projection in
the normal contract-first process. Include source/IR/wire/evaluator/reader/identity/
authoring compatibility, literal vectors, hostile tests and migrations. No
historical source or artifact is silently reinterpreted.

Review the [hybrid source surface](source-syntax-proposal.md) independently:
tables for containment, typed bindings/references for shared graphs, contextual
variants and retained pure expressions/templates. Before freeze, specify a
bootstrap-readable grammar selector with matching capture metadata, interleaved
construction scope, default/duplicate rules and identity-aware editor refactors.
Prove multi-job CI, shared embedded resources, cloud/Linux graphs and third-party
operation composition with literal oracles and editor round trips. Dotted field
construction is not expression-level projection; shorthand does not extend a
closed variant. Neither optional TOML capture nor lexical construction aliases
should block the first package/mapping kit. Existing supported source remains a
valid adoption route while a successor syntax profile is reviewed.

Choose the first non-Rust boundary, define runtime result transport, and add
streaming only for a real monitoring/progress case. Harden actual executable
adapters and package updates before independent third-party distribution.

### E. Qualify a supported ecosystem release

V2 is ready to make its stable integration claim when:

- Every shipped capability has an owner, a supported API/profile and explicit
  exclusions; current availability agrees with documentation and runtime discovery.
- An external developer can acquire/lock/validate/compile/inspect a project using
  published artifacts and documentation without copying internal test helpers.
- Domain packages, mappings and adapters can be added without compiler or generic
  UI changes; semantic-only use does not start executable plugins.
- Compatibility, authorization and execution outcomes remain distinct and unknown
  required behavior fails closed.
- Two-domain and second-target proof cases pass, including negative and failure
  cases; support matrices identify what is actually tested.
- Numeric/result transport preserves exact types and restrictions; a large value,
  stale session, changed target or malformed result cannot cross unchecked.
- Source-to-plan/export/runtime diagnostics remain useful and privacy-safe.
- Package/protocol/profile updates and retained old artifacts obey the stated
  compatibility policy; historical identity vectors remain immutable.
- Per-phase bounds and measured quality gates match final supported workloads,
  host platforms and release inputs.

This is not a requirement to finish all CI/CD delivery strategies, all MCU
diagnostics, a global registry or a universal runtime before application teams
can start. Those remain separately scoped releases. Stable means an honest,
usable, extensible contract with evidence, not an unlimited feature list.

### Keep tracking small and executable

Use this report as discovery input, not another permanent copy of the operational
checklist. After review, move accepted requirements into one future portable plan
with stable IDs, owner, dependent profile, fixture family and acceptance gate.
Keep decisions and necessary semantic prose; generate inventories and compatibility
reports from machine-readable registries/tests. Do not require developers to write
a new Markdown progress log for every change.

Suggested everyday adopter workflow, **not implemented command syntax**:

```text
initialize example -> explicitly resolve/lock inputs -> validate/compile
                   -> inspect/select -> domain plan and compatibility report
                   -> export, or explicitly authorize and run
```

Repository contributor commands such as `cargo xtask` remain repository tools.
They should not become an undocumented package manager or mandatory production
dependency of Flow or the embedded application.

## Decisions not to inherit from the drafts

- **No mandatory universal runtime.** Share descriptions, validation and useful
  matching utilities; let consumers own scheduling and effects.
- **No automatic `.nmap` or package-layout commitment.** Prove the minimal model
  before choosing a permanent syntax, suffix or seven-crate workspace.
- **No compiler plugin to call vendor SDKs.** Package installation and host
  adapter registration add support; compilation remains no-I/O and data-only.
- **No dotted-name inheritance.** Namespaces organize ownership. Exact dependencies
  and conformance establish compatibility.
- **No promise of universal conversion.** Some providers, transports, security
  modes or devices cannot satisfy a requested guarantee; refusal is correct.
- **No unqualified composite transaction.** Multi-step mapping may leave effects
  behind; compensation and uncertain outcomes remain visible.
- **No complete hardware diagnosis from software.** External instrumentation,
  trusted references and physical limits remain part of the product contract.
- **No health percentage invented from pass counts.** Coverage, uncertainty and
  prerequisites matter; any aggregate score needs a justified model.
- **No treating signing as permission.** Authentic package content may still be
  unsafe or unauthorized to execute.
- **No automatic MCU deployment of Neutral.** Host-side use satisfies the initial
  scenarios; firmware execution needs separate evidence.
- **No v2 rewrite for its own sake.** Preserve v1 contracts, use existing records,
  variants, evaluator and selectors, and add only what real integrations prove missing.

## Evidence and references

### Repository evidence

The reviewed baseline is the checked-out repository on the audit date; no commit
identifier is needed to define a new language contract. This report is not release
evidence and must be reassessed as the implementation and consumer scope evolve.

- [Operational v1 checklist](../portable/development/07-IMPLEMENTATION-CHECKLIST.md)
  and [contract checklist](../portable/specs/contracts/v1-checklist.md): completion
  and qualification boundaries, separate from design acceptance.
- [Capture/project contract](../portable/specs/contracts/PROJECT.md),
  [project IR](../portable/specs/contracts/PROJECT-IR.md) and
  [composition profile](../portable/specs/contracts/COMPOSITION-PROFILE.md):
  no-I/O inputs, complete artifacts, companions and public closure.
- [Vocabulary composition](../portable/specs/contracts/VOCABULARY-COMPOSITION.md)
  and [variants](../portable/specs/contracts/VARIANTS.md): exact type/default/
  restriction/dependency semantics and both declaration origins.
- [Consumer profile](../portable/specs/contracts/CONSUMER-PROFILE.md) and
  [example consumer](../portable/specs/contracts/EXAMPLE-CONSUMER.md): typed transport
  and examples are not universal domain interpretation or effectful execution.
- [Expressions](../portable/specs/contracts/EXPRESSIONS.md),
  [shared evaluator](../portable/specs/contracts/EVALUATOR.md),
  [expansion](../portable/specs/contracts/EXPANSION.md) and
  [successor compatibility](../portable/specs/contracts/EXPRESSION-PROFILE.md):
  planned completed-v1 pure semantics, profile separation and immutable history.
- [Authoring contract](../portable/specs/contracts/AUTHORING.md): generic data
  descriptors, source-preserving model, revision safety and compiler authority.
- [Compiler APIs](../crates/neutral-compiler/README.md),
  [IR contracts](../crates/neutral-ir/README.md),
  [consumer APIs](../crates/neutral-reader/src/consumer/README.md),
  [CLI](../crates/neutral-cli/README.md) and
  [probe](../crates/neutral-probe/README.md): actual integration surfaces and limits.
- [Consumer/selectors tests](../crates/neutral-test-suite/tests/composition_contract/pipeline/selectors.rs),
  [consumer envelope tests](../crates/neutral-test-suite/tests/composition_contract/pipeline/consumer.rs)
  and [quality policy](../config/quality-gates.toml): existing validation coverage
  and configured measurements, not fresh results from this audit.

Links into the active portable plan are evidence locators, not production runtime
dependencies. When that plan is archived, preserve or update these documentation
links to its maintained archive without making consumers depend on its directory.

### Scenario sources

- Supplied **EmbeddedHealth — Architecture**: discovery, diagnostics, agents,
  capability differences, safety, CLI/GUI, reporting and multi-vendor proof cases.
- Supplied two **Neutral Mapping System — Architecture Review** texts: candidate
  packaging, mapping and adapter concerns, reviewed critically rather than adopted.
- [Neutral Flow requirements](https://github.com/neutral-ecosystem/neutral-roadmap/blob/main/neutral-flow/REQUIREMENTS.md)
  and [architecture proposal](https://github.com/neutral-ecosystem/neutral-roadmap/blob/main/neutral-flow/ARCHITECTURE.md):
  domain scenarios and boundaries, not authority to redefine Neutral.

The supplied local documents are not copied into the repository and their private
filesystem locations are not retained here.

### Engineering precedents, not selected dependencies

WIT, Terraform's plugin protocol, OCI descriptors and TUF were consulted only for
the specific namespace, process-boundary, content-reference and update-security
ideas linked above. None supplies Neutral's complete mapping semantics. CWL's
[requirements versus hints](https://www.commonwl.org/v1.2/Workflow.html#Requirements_and_hints)
is another useful precedent for distinguishing mandatory behavior from optional
preferences; adopting it does not mean importing CWL's language or runtime.

Document verification: `cargo xtask check` passed, including workspace compilation,
repository Markdown links, dependency boundaries, traceability and repository
hygiene. Whitespace checks passed for the documentation changes. These checks do
not establish the proposed consumer or hardware acceptance results.

**Verdict:** Finish and expose v1, then prove a small package/mapping/adapter kit
against both consumers. Completed v1 can express most intent; stable interoperability
requires the missing contracts and evidence, not a general-purpose executor.

**Not checked:** No new implementation, full test/fuzz/mutation/soak campaign,
provider execution, hardware-in-the-loop test or on-device resource measurement was
performed. Not every production function was read. The proposed requirements and
acceptance corpus need review before they become approved v2 scope.
