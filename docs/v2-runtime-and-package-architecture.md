<!-- SPDX-License-Identifier: Apache-2.0 -->

# After v2: runtime, packages, editor and self-hosted CI

Date: 13-10-2026. Status: **architecture proposal for review; not implemented or frozen**.

This document proposes the integration architecture to build **after the Neutral
v2 foundation is complete**. It complements the
[ecosystem-readiness audit](v2-ecosystem-readiness-audit.md): that document asks
what is missing; this one recommends how the pieces fit together.

The desired outcome is one extensible ecosystem in which installing a package
adds discoverable typed operations, editor choices and compatible implementations.
A CI/GitHub package and an embedded/ESP package follow the same package and host
contracts, without pretending their operations or execution lifecycles are alike.

The first complete proof is deliberately concrete: install the CI packages,
construct Neutral's own CI in the editor, save ordinary `.neu`, export a GitHub
workflow and observe GitHub running the repository's actual quality command.
No application, package loader, editor or workflow is implemented by this document.

## Contents

- [1. Architectural choice](#1-architectural-choice)
- [2. What runtime means](#2-what-runtime-means)
- [3. System boundaries](#3-system-boundaries)
- [4. One common package contract](#4-one-common-package-contract)
- [5. One package layout](#5-one-package-layout)
- [6. Discovery and command resolution](#6-discovery-and-command-resolution)
- [7. Dynamic editor and source round trips](#7-dynamic-editor-and-source-round-trips)
- [8. Planning, protocols and execution](#8-planning-protocols-and-execution)
- [9. First vertical slice: Neutral CI on GitHub](#9-first-vertical-slice-neutral-ci-on-github)
- [10. Full acceptance test](#10-full-acceptance-test)
- [11. Self-hosting without a bootstrap trap](#11-self-hosting-without-a-bootstrap-trap)
- [12. Delivery order and approval boundaries](#12-delivery-order-and-approval-boundaries)
- [References and verification](#references-and-verification)

## 1. Architectural choice

### Options considered

| Approach | Benefit | Cost | Decision |
| --- | --- | --- | --- |
| Shared package model and host SDK; domain-specific planning/execution | One installation/discovery/editor boundary, with honest domain semantics. | Requires explicit contracts between shared and domain components. | **Recommended.** |
| One universal executor and node graph for every domain | Initially looks like one runtime and one plugin API. | CI export, device sessions and system reconciliation acquire misleading common states and privileges. | Do not start here. |
| Entirely separate plugin/package/editor systems in each application | Each application can optimize locally. | Repeats installation, schema discovery, mapping, diagnostics and compatibility work. | Avoid duplicated infrastructure. |

The common abstraction is a **package describing typed intent and its supported
interpretations**, not a shell command and not a universal execution node.

A CI step may resolve to a GitHub `uses` entry. A memory inspection may resolve
to a device protocol request. A cloud resource may resolve to an API operation,
and a Linux resource to a checked reconciliation plan. None must be flattened
into a command string merely to share infrastructure.

### Recommended decisions

1. Keep compilation, pure evaluation and authoring free of ambient acquisition
   and effects. Installing a package never extends the compiler with vendor code.
2. Standardize the package envelope, exact contract references, discovery and
   adapter negotiation. Make optional roles explicit rather than inventing a
   different package format for every product.
3. Separate portable domain contracts from provider implementations. Provider
   packages depend on the contracts they implement; they do not redefine them.
4. Build one generic editor over public authoring APIs and package data. Packages
   cannot inject JavaScript, arbitrary UI components or executable metadata.
5. Share a small host SDK and checked protocols, not every scheduler or state store.
6. Make the first CI backend an exporter. GitHub is its execution environment;
   there is no need to build a Neutral-hosted CI runner first.
7. Require an end-to-end, externally observable proof before claiming that the
   ecosystem is ready. Valid source or attractive editor nodes are insufficient.

These are proposed choices, not retroactive changes to frozen v1/v2 profiles.
New package, mapping or host protocols require their own reviewed versions.

## 2. What runtime means

There are three distinct meanings; using one word for all three obscures design.

| Layer | Responsibility | Effects? |
| --- | --- | --- |
| Pure evaluator | Evaluate the language's typed expressions with explicitly supplied bindings and bounds. | No filesystem, network, clock, environment or credential access. |
| Integration host | Install/lock packages explicitly, discover descriptors, validate plans, resolve implementations and launch authorized adapters. | Only through explicit host operations and policy. |
| Domain execution environment | Schedule CI work, manage device sessions, reconcile resources, collect observations and recover failures. | Yes; owned by the application/provider. |

The shared integration host is needed. A single shared domain executor is not.
The host can initially be a library with a CLI and a bounded local editor service,
not a permanently running daemon or cloud platform.

| Consumer | Initial domain runtime | What is shared |
| --- | --- | --- |
| Flow / CI export | GitHub schedules and runs the generated workflow. Flow validates and lowers the graph. | Package registry, type/value checks, mapping descriptors, diagnostics and export receipts. |
| Embedded application | Host application owns sessions and authorization; a vendor adapter/tool/optional device agent performs operations. | Same package/discovery/protocol boundary and checked input/results. |
| Cloud application | Consumer owns credentials, API lifecycle, state, drift and recovery. | Same exact contracts, binding decisions and adapter host primitives. |
| Linux automation | Consumer owns inventory, privilege admission, resource locks and reconciliation. | Same packages, safe process boundary and explicit results/effects. |

An application can embed the host SDK rather than start another process. That
does not remove the trust boundary around third-party executable adapters.

## 3. System boundaries

```text
                         installed, locked packages
                        /            |             \
             semantic contracts   mappings     authoring metadata
                       |             |                 |
                  .neu source <-> generic editor/model
                       |
              Neutral compiler + independent reader
                       |
               complete immutable .nir artifact
                       |
               selected typed public entry
                       |
              domain validation and planning
                       |
        captured target facts + compatible implementation resolution
                       |
              immutable target-bound consumer plan
                       |
          export files                 authorize effects
               |                             |
        GitHub workflow              adapter / domain runtime
               |                             |
          GitHub runner              tools / devices / APIs
                       \             /
                    checked results and evidence
```

The editor opens original sources and the source-preserving authoring model.
The `.nir` artifact is for independent semantic consumption, not a replacement
for editable source, comments, aliases or templates.

### Ownership and dependency direction

| Component | Owns | Must not acquire by accident |
| --- | --- | --- |
| Neutral language SDK | Capture contracts, compiler, reader, evaluator, authoring and identities. | Device SDKs, provider credentials, package downloads. |
| Package/host SDK | Manifests/locks, safe content storage, compatibility discovery and adapter supervision. | Domain scheduling policy or compiler-private ASTs. |
| Domain SDK | Meaning of operations/resources/results, graph invariants and domain plans. | Backend-specific meaning disguised as portable semantics. |
| Provider package | Exact implemented contracts, mappings and provider-specific exporter/adapter. | Unilateral changes to portable guarantees. |
| Editor | Generic forms/graph/source editing, diagnostics, previews and explicit user actions. | Authority to execute merely because a project was opened. |
| Application | Authorization, domain lifecycle, credentials and persistence. | Reinterpretation of unknown language or operation features. |

These are logical boundaries, not a requirement to create six new repositories
or one crate per row. Start with the fewest components that preserve the direction.
Keep `xtask` as repository automation; consumer packages must not depend on it.

## 4. One common package contract

### Terminology

Use **package** for the installable unit. A package can supply a library of
source/types, an integration, an executable adapter, or several explicitly
declared roles. “Plugin” describes an executable extension role, not a special
second installation format. A vocabulary remains data, never executable code.

Package identity, semantic vocabulary identity, operation identity, executable
content identity and protocol version are separate. A friendly package name or
source alias is never the dispatch key.

### Mandatory envelope and optional roles

| Part | Required? | Contents and responsibility |
| --- | --- | --- |
| Manifest | Always | Package identity/release, package-schema version, required host/language features, exact exports, dependencies and file inventory. |
| Integrity inventory | Always | Digests, lengths and roles of distributed files; bound to package content identity and the host's publisher trust policy. |
| Semantic exports | When authorable data is supplied | Data-only vocabulary bundles and/or source modules, with exact nominal identity/revision. |
| Operation index | When operations are supplied | Exact input/result contracts, role, effect class, semantic specification and conformance profile. |
| Authoring metadata | Optional | Bounded presentation data targeting existing exact descriptors. No semantic overrides. |
| Mappings | When an implementation is supplied | Exact source/target contracts, applicability, required guarantees, transforms and implementation reference. |
| Executable components | Optional | Explicit planner/exporter/adapter roles, protocol/features, content and supported host platforms. |
| Documentation/license | Always | Responsibilities, installation/use, support limits, permission needs and legal provenance. |
| Examples/conformance | Required for supported integration claims | File-backed examples, literal expected plans/reports and contract behavior tests. |

Do not require a hardware adapter in a type-only package, or invent dummy
executables/directories so all packages look superficially identical. Structural
similarity means the same manifest and role contracts with meaningful omissions.

Executable validators/planners, where genuinely needed, are separate executable
components with explicit invocation policy. They are not callbacks smuggled into
vocabulary fields. Pure core type/default/restriction checks remain in Neutral.

### Portable contract packages versus provider packages

Illustrative distribution names, not reserved namespaces:

| Domain contract package | Provider implementation package | Shared relationship |
| --- | --- | --- |
| `neutral-ci` | `neutral-ci-github` | Provider implements exact CI operation/workflow contracts. |
| `neutral-embedded` | `neutral-embedded-esp` | Provider implements exact device operation/result contracts. |
| `neutral-cloud` | `neutral-cloud-provider` | Provider implements exact resource/action contracts. |
| `neutral-linux` | `neutral-linux-distribution` | Provider implements exact system resource/action contracts. |

A provider may expose additional explicitly vendor-specific types. The editor
labels them as provider-specific; using them changes portability requirements.
There is no inheritance merely because one package name ends in `.esp` or shares
a prefix with another.

Portable source normally uses the domain vocabulary. Host target configuration
selects GitHub, ESP or another implementation. Switching implementation should
not require rewriting portable source if both satisfy its exact contract.
Unsupported behavior is rejected, not approximated into apparent compatibility.

### Dynamic discovery does not make closed types open

Installing a package can add new nominal types and implementations. It cannot
silently add a tag to an existing closed variant or make `Ref<NewOperation>` a
`Ref<ExistingOperation>`. Similar roles and shapes do not establish subtyping.

Use two explicit composition levels:

- Domain-owned closed aggregates are useful for small, versioned contracts,
  including the first CI example below. A new alternative requires an explicit
  compatible contract update/migration, not an undocumented plugin insertion.
- For third-party operation composition, recommend a domain assembly contract
  whose links identify public operation bindings and exact expected contracts.
  Each operation retains its own fully typed vocabulary value. The independent
  consumer resolves every link through the reader and operation index before
  accepting a plan. Links must not be arbitrary JSON payloads or executable names.

The assembly links can be ordinary typed symbol/selector records; they are not
a new implicit language reference. Core validates their data shape, while the
consumer validates existence, public visibility, exact operation/result type,
graph membership and edge rules. A malformed link may therefore be core-valid
but consumer-invalid, and must fail before export or execution. The editor builds
links through typed selection, not free-form names, and reports the distinction.

Freeze this assembly extension only with real third-party composition examples.
The first closed five-operation CI package does not prove arbitrary third-party
step extensibility. This explicit seam avoids both a compiler plugin system and
an untyped universal `Any` value while preserving future package-driven graphs.

### Source `use` is not installation

In existing source grammar:

```neu
use NeutralCi as ci
```

`NeutralCi` is an illustrative canonical vocabulary identity, not a repository
URL. `ci` is a source-local alias. The host associates an installed package's
export with that exact identity and supplies the lock and captured bytes.

Typing `use` for an absent package produces an unresolved-dependency diagnostic
and may offer an explicit installation action. Parsing/opening the file never
downloads a package, starts an adapter or executes a post-install script.
Do not introduce a new `using` grammar merely for the editor.

### Locks and updates

The project host manifest records requested packages and target choices. Its
generated integration lock records exact resolved packages, semantic exports,
mappings, component digests/protocols and decision-affecting defaults. It can
contain provider components that are not semantic vocabulary dependencies.

The compiler's captured semantic locks still cover exactly the vocabularies
required by the source/dependency closure. Do not pass the entire integration
lock as extra semantic locks or include adapter metadata in project identity.

Latest-compatible requests are allowed during an **explicit update**. Normal
compile/export uses the lock and never silently changes provider implementation.
For deliberately moving runtime channels such as Rust `stable`, preserve the
channel request and record the actual tool version used by each CI attempt;
do not claim repeatable tool bytes merely because the source is unchanged.

Install data without activating executables. Use staged verified installation,
safe archive handling, atomic activation and an offline locked mode. Signature
verification identifies trusted content under a policy; it does not authorize
that content to touch a device, run privileged code or access credentials.

## 5. One package layout

### Source package

The manifest is authoritative; these directory names are a recommended layout,
not magic paths that the loader guesses.

```text
neutral-ci-github/
  neutral-package.toml
  README.md
  LICENSE
  contracts/
    operations.json
  mappings/
    github.json
  authoring/
    presentation.json
  components/
    github-exporter/
      component.toml
      src/
  examples/
    self-ci.neu
  tests/
    contracts/
    mapping/
    editor/
    golden/
    integration/
```

The portable `neutral-ci` package supplies its vocabulary under `contracts/`.
The GitHub provider refers to those exact contracts rather than copying another
independently maintained definition. `neutral-embedded-esp` uses the same layout
but may contain an adapter component instead of an exporter. An optional agent
is a separately identified firmware artifact, not a compiler dependency.

Empty directories are omitted. A pure-data package has no `components/`.
An executable component may be implemented in Rust, another language or a native
wrapper; the published protocol, not its internal folder layout, is the boundary.

### Installed package

Installed packages are immutable, content-addressed and separate from mutable
application state. The host maintains an index; applications cannot edit installed
files to change a lock's meaning. Keep these stores separate:

- Verified package data and executable artifacts.
- Project manifest, generated lock, source and editor layout.
- Ignored build/export outputs and caches.
- Runtime sessions, credentials and retained application evidence.

Distribution artifacts contain explicit supported-platform binaries where needed,
not a requirement to compile arbitrary third-party source during installation.
No automatic installation hooks. Development builds are an explicit trusted
developer operation, separate from end-user package installation.

### Illustrative manifest projection

This is a **proposed readable projection**, not an implemented schema or a
complete copy-paste package. The reviewed schema must additionally bind exact
contract references, file inventories and content digests before installation.
Protocol identifiers here are illustrative and independently versioned.

```toml
schema = "neutral.package/1"

[package]
id = "neutral-ecosystem/ci-github"
version = "1.0.0"
license = "Apache-2.0"

[requires]
host = ["package-discovery", "typed-mapping", "export-component"]
language = ["complete-project-reader", "consumer-contracts"]

[[dependencies]]
package = "neutral-ecosystem/ci"
role = "semantic-contracts"
version = "1.0.0"

[[mappings]]
file = "mappings/github.json"
component = "github-exporter"

[[components]]
id = "github-exporter"
role = "exporter"
protocol = "neutral.integration/1"
descriptor = "components/github-exporter/component.toml"
```

Actual projects should not repeat the package's own version in every file.
Generate inventory, catalogue and protocol bindings from their owning typed
models; keep one semantic schema definition, not several string-pattern validators.
Version/content values are required identity inputs, not constants to delete.

## 6. Discovery and command resolution

### What the editor discovers

The host loads a verified manifest, validates its data-only exports, and asks
Neutral's public authoring APIs for exact descriptors. The operation index then
associates those descriptors with domain roles.

For an operation, discover:

- Exact semantic operation identity and input/result type identities.
- Authorable fields, required/default/omitted/null states and restrictions.
- Typed result/output ports, explicit prerequisites and applicability.
- Effect classification and declared required capabilities.
- Safe plain-text documentation and presentation grouping.
- Compatible implementations and why others do not match, in a separate context.

Do not recognize an operation because a record is named `Command`, because a
field is called `needs`, or because metadata supplies a familiar icon. A reviewed
operation index/protocol establishes the role and exact association. If this
requires richer authoring descriptors than the foundation supplies, version the
extension; do not insert unknown members into frozen metadata.

### Resolution pipeline

1. Validate complete source/artifact and exact contract closure.
2. Select a public entry by exact type and explicit symbol, not first-match name.
3. Apply the domain's graph, presence, type and result-association rules.
4. Read the explicit target snapshot and locked candidate implementations.
5. Negotiate exact profiles/features and test applicability/required guarantees.
6. Validate bounded typed transformations and aggregate effects.
7. Produce one immutable bound plan or a structured rejection/ambiguity report.
8. For execution, refresh required safety facts and authorize separately.

Candidate enumeration order cannot change the decision. An explicit configured
preference may break a compatible tie; otherwise material ambiguity fails.
An alternative that weakens a requirement needs a new approved context and full
revalidation. Do not try installed adapters until something happens to work.

### The same mechanism, different resolutions

| Typed intent | Locked mapping resolution | Dispatch representation |
| --- | --- | --- |
| CI checkout | Compatible GitHub checkout mapping/action reference | Structured GitHub action step. |
| CI typed process request | Supported runner and argument-rendering contract | Safely rendered runner step, not arbitrary string concatenation. |
| Embedded flash inspection | Compatible device/security/transport facts | Typed adapter request, possibly a vendor tool or agent message. |
| Cloud resource change | Supported API/resource/reconciliation semantics | Domain change plan and scoped API requests. |
| Linux package requirement | Supported distribution/package capability | Domain transaction/reconciliation request under explicit privilege policy. |

Shell is an explicit implementation choice, not the operation model. Native
process adapters receive executable/argument arrays and controlled environment
bindings. Exporters targeting a shell must implement and test its quoting and
expression rules. A backend expression string cannot replace Neutral semantics.

## 7. Dynamic editor and source round trips

### The package-driven experience

The editor has a stable core: project/dependency view, catalogue, generic property
forms, typed connections, source view, diagnostics and plan/export preview.
Installing a new compatible package populates these through descriptors. Adding
an ESP operation must not require adding an ESP switch branch to the editor.

Recommended user journey:

1. Create/open a project and select an implemented language/authoring profile.
2. Explicitly install the portable domain package and chosen provider package.
3. Review required dependencies and executable roles before activation.
4. Add `use` through the source/GUI dependency action; capture exact locks.
5. See newly discovered operations and their fields in the catalogue.
6. Add a typed entry, configure values, and connect explicit typed outputs or
   prerequisite ports where the domain declares them.
7. Save canonical `.neu`; compile and independently compare intended meaning.
8. Preview target compatibility, permissions, selected mappings and export diff.
9. Export or separately authorize execution. Opening/editing never executes.

In the CI example, the catalogue presents checkout, toolchain setup, tool
installation, process invocation and artifact retention. In an embedded project,
it presents inspected device operations from that package. The editor code does
not contain either fixed command list.

The headless CLI uses the same catalogue for operation listing and typed help.
It does not generate arbitrary shell subcommands from record names. Static package
capability is shown separately from live target availability and authorization;
seeing an operation in the palette does not mean the connected target can run it.

### Generic rendering rules

| Contract fact | Generic editor control |
| --- | --- |
| Boolean/string/exact `num` | Typed field, retaining exact decimal text. |
| Finite choice/closed variant | Choice control with typed payload editor. |
| Record/list | Nested form or ordered collection editor. |
| Optional/nullable/defaulted field | Separate omit, explicit-null and supplied states; show default without silently inserting it. |
| Ref/reuse/output selector | Compatible typed connection control, preserving distinct edge kinds. |
| Pure expression/template | Governed structured source model plus supported visual control; no opaque executable escape hatch. |
| Capability/effect requirement | Readable requirement and compatibility/policy explanation, not proof of authorization. |

Use the existing plain-text authoring metadata where sufficient. Icons, layouts,
domain graph hints or richer port associations need a separately reviewed
presentation extension with sanitized local assets and explicit exclusions.
No remote executable assets, event handlers or arbitrary provider HTML.

Generic forms can cover every source-authorable type, but not every domain-specific
visualization equally well. Begin with forms and an explicit connection view.
Optional specialized host views may follow under separate trusted APIs; they
cannot become necessary for opening or validating ordinary portable source.

### Graphs are not all the same graph

Show structural references/reuse, potential guarded output dependencies and
explicit scheduling prerequisites separately. A valid core ref cycle does not
establish a valid CI schedule, and a potential output is not automatically an
unconditional prerequisite. Domain validation owns scheduling interpretations.

### Source is durable; layout is not semantic

The planned source surface is hybrid: typed tables for containment, explicit
variant constructors for choices, named typed bindings/references for shared
graphs, and pure expressions/templates for computation and reuse. Tables are
not mandatory for every value. See the [source syntax proposal](source-syntax-proposal.md)
for draft forms and required review gates; this architecture does not activate
them or change frozen profiles.

The editor must discover the selected grammar through a bootstrap-readable source
header and require captured metadata to agree. Construction labels are local
authoring addresses, not domain identifiers or reference targets. Moving a section
must preserve its explicit parent; moving ordered rows may change meaning.
Extracting nested data into named bindings/references is an explicit semantic
refactor, potentially changing identity and provenance, not automatic formatting.

Preserve original source, comments, aliases, templates, omissions and expression
forms through the public source-authoring model. `.nir` and expanded execution
plans cannot reconstruct those choices alone.

Round trip: `open source -> edit model -> project .neu -> compile -> independent
read/compare -> reopen`. Publish the new model/source atomically only on success.
Invalid edits can remain an explicit draft, never a falsely accepted artifact.
Stale source/lock/metadata revisions invalidate an edit; require explicit reopen
or reviewed merge rather than overwriting newer content.

Node coordinates, open panels and selection stay in an optional editor sidecar.
Removing it loses layout only. Headless tools must still compile and export the
same meaning. Existing unsupported authoring features remain visibly read-only
or rejected, never silently flattened into different source.

## 8. Planning, protocols and execution

### Share a small SDK surface

The common SDK supplies verified package lookup, public catalogue discovery,
typed value/selector checks, context-bound compatibility decisions, bounded
diagnostics and supervised protocol transport. It does not promise a shared
resource database, worker fleet or device scheduler.

Recommended component roles:

| Role | Input | Output | Effect policy |
| --- | --- | --- | --- |
| Domain validator/planner | Validated entry and explicit context | Validated domain plan or diagnostic report | No ambient discovery; planning inputs supplied. |
| Exporter | Validated bound plan and locked export settings | Complete output buffers, mappings and receipt | Host writes only explicitly approved paths. |
| Adapter | Authorized typed request, scoped handles and target session | Typed result, progress and effect/cleanup disposition | Explicit application/platform authority. |

One component may implement several negotiated roles. A component is not loaded
just to show its labels in the editor. Capabilities are advertised by data and
checked at explicit activation/handshake. A malicious executable can lie; real
enforcement must not rely on its declared role alone.

Keep the common host operations small: install/lock, catalogue, validate, resolve,
export and invoke. Component messages cover handshake, role-specific plan/export/
invoke requests, progress, terminal result and cancellation. Unsupported roles
return an explicit error; an ESP adapter need not pretend to export workflows,
and a GitHub exporter need not offer device sessions. Generate typed messages
and bindings from one reviewed protocol model instead of scattering method and
status strings across consumers.

### Protocol boundary

Use an explicitly versioned out-of-process protocol for independently distributed
executables. A bounded length-framed message stream with typed, pretty-printable
JSON payloads is a reasonable first host boundary; large artifacts use separately
checked content handles rather than embedding complete binary data in every frame.
The framing/schema needs review and hostile tests before it is a contract.

Reserve protocol output for framed messages; diagnostic process logs use a separate
bounded channel. Request IDs correlate interleaved events. A malformed message or
protocol mismatch terminates the request without accepting a partial result.

Keep exact decimals as canonical text in the value codec; do not route addresses
or large counts through JavaScript floating-point numbers. Canonical identities
come from governed typed projections, not JSON formatting. Probe inspection JSON
is not silently promoted into the runtime value protocol.

Negotiate protocol, roles, exact operation contracts, required features and
limits. Requests carry unique request/attempt IDs and context identities.
Cancellation, crash, malformed frame, timeout and lost acknowledgement have
distinct outcomes. A progress event is not a terminal successful result.
Validate all adapter outputs independently before using them as evaluator bindings.

Bound input/output bytes, frame counts, retained logs, work and elapsed time;
apply backpressure and aggregate bounds. Handle process trees and device cleanup
truthfully. A timeout after a write is an uncertain effect, not safe automatic
retry or evidence of rollback. Domain-specific recovery remains in the consumer.

An out-of-process adapter is not automatically sandboxed. Use real filesystem,
network and privilege controls where the trust model requires them, or explicitly
limit support to trusted executable packages. Embedded firmware protocols can
use a different bounded transport; they need not implement the entire host SDK.

### Plan identity, privacy and authority

Complete project identity excludes target selection, mappings and external
bindings. The consumer bound-plan identity includes every decision-affecting
input: selected entry, exact operation/mapping/component content, profiles,
target facts, relevant bindings and policy revisions.

Authorization is a separate expiring decision over the exact plan/target/effects.
No digest, compatibility report, public declaration or package signature grants
permission. Resolve credential handles only at the effect boundary; do not put
raw secrets or public hashes of low-entropy secrets in plans or caches.

Preserve source -> model -> plan -> export/runtime diagnostic chains, with
explicit unavailable/redacted states. Public visibility is not confidentiality.
Installed package paths, local user paths and sensitive device identifiers must
not leak into portable identities or default published reports.

## 9. First vertical slice: Neutral CI on GitHub

### Supported scope

The initial package pair implements one static CI job: checkout, install the
requested Rust toolchain/components, install Nextest, run the repository quality
command, and retain evidence even when the quality command fails. It supports
push-to-main and explicit manual dispatch, with read-only repository permissions.

This mirrors the responsibilities of the current
[repository CI](../.github/workflows/ci.yml), not a release/deployment engine.
Preserve the canonical `cargo xtask ci pr` composition; do not copy its Rust
format/lint/test policy into package mappings or an editor graph.

Excluded initially: arbitrary provider YAML, custom backend expression strings,
secret-bearing jobs, release publication, dynamic matrices and a local CI runner.
Reject unsupported constructs explicitly rather than dropping them during export.

### Package responsibilities

`neutral-ci` supplies public `Workflow`, `Job`, `Step` and the typed payloads
below, their exact semantics, operation index and authoring descriptors. CI domain
validation belongs to the CI SDK and can be supplied as an explicitly activated
planner component; data-only use does not start it. `neutral-ci-github` supplies
the exporter, compatible mappings and provider-specific conformance tests.
Initially the domain module and exporter may share one implementation build,
but portable validation must not acquire GitHub-specific behavior or be copied
into every later provider.

The minimum `Step` is a closed variant over `checkout`, `rust_toolchain`,
`install_tool`, `command` and `retain_artifacts`. These are library contracts,
not language keywords. Fields and constraints live in the one semantic schema;
the editor and exporter obtain validated typed facts instead of reparsing strings.

### Proposed `.neu` example

This source uses existing-style data syntax with **new proposed vocabulary
types**. It is a design target, not a fixture that currently compiles. A completed
v2 host selects its supported source profile; the `1.0` header below is not a
claim that a package release called v2 must introduce `neu "2.0"`.

This is the record-literal semantic baseline, not a recommendation to expose
wire-style wrappers in everyday authoring. The
[source syntax proposal](source-syntax-proposal.md) rewrites the same example
using typed tables and explicit variant selection, and audits optional TOML
capture. Its new syntax requires separate approval and is not implemented here.

```neu
neu "1.0"
module neutral::automation
use NeutralCi as ci

public ci::Workflow checks = {
    name: "Neutral CI",
    push_branches: ["main"],
    manual: true,
    jobs: [
        {
            id: "ci",
            runner: {
                os: "linux",
                architecture: "x86_64",
            },
            steps: [
                {
                    tag: "checkout",
                    payload: {},
                },
                {
                    tag: "rust_toolchain",
                    payload: {
                        channel: "stable",
                        components: ["rustfmt", "clippy"],
                    },
                },
                {
                    tag: "install_tool",
                    payload: {
                        tool: "cargo-nextest",
                    },
                },
                {
                    tag: "command",
                    payload: {
                        executable: "cargo",
                        arguments: ["xtask", "ci", "pr"],
                    },
                },
                {
                    tag: "retain_artifacts",
                    payload: {
                        name: "workflow-evidence",
                        paths: [
                            "test-results/workflows",
                            "test-results/tests",
                            "target/nextest/ci/junit.xml",
                        ],
                        always: true,
                        retention_days: 14,
                    },
                },
            ],
        },
    ],
}
```

These proposed fields are deliberately ordinary data. The contract defines that
jobs have unique IDs, steps are sequential within a job, later ordinary steps
stop on failure, and `always` retention is attempted independently without turning
a failed quality command into success. List order here is semantically meaningful.
Future job dependencies use explicit typed edges, not inference from source order.

Job/process outcomes are provider results, not values magically produced by
compilation. The proof needs no deferred expression or live result binding.

### Concrete mappings, not a generic `run(string)` escape hatch

| Semantic request | GitHub interpretation | Required check |
| --- | --- | --- |
| `checkout` | Locked checkout action, credentials persistence disabled where not needed. | Exact action binding and repository permission scope. |
| `rust_toolchain` | Locked toolchain installer action with requested channel/components. | Supported channel/components and actual installed version recorded. |
| `install_tool` for Nextest | Locked compatible installer action. | Known supported tool contract; no arbitrary package-name-to-shell interpolation. |
| `command` | Explicit supported shell step rendering executable/argv. | Argument fidelity, no context interpolation injection, nonzero exit propagates. |
| `retain_artifacts` | Locked artifact-upload action with provider always-condition. | Safe workspace-relative paths, supported retention, no accidental sensitive files. |

Provider action references and runner selection belong to the provider's reviewed
mapping/default configuration and resolved lock, not hardcoded throughout the
editor, host and compiler. Resolve action releases to immutable commit references
when locking; update explicitly. A user requesting a newer release changes the
lock/export context, not language meaning.

The runner contract specifies requirements, not an invented universal OS image.
Record the selected GitHub runner label/image policy in the bound plan and the
actual image/tool details in run evidence. Fail if architecture or required tools
cannot be provided. Moving hosted images/channels limit environmental repeatability.

### Export output and receipt

The exporter returns complete YAML buffers plus a bounded source mapping and
receipt; it does not write files itself. The host approves and atomically writes
the selected `.github/workflows/ci.yml`. GitHub requires workflow YAML under
that directory. See [workflow syntax](https://docs.github.com/en/actions/reference/workflows-and-actions/workflow-syntax).

The receipt binds source/capture and logical identity, selected entry, exact
package/mapping/exporter content, target/export settings and output digests.
It must not require maintaining a Markdown log or manually copying commit hashes.
Historical source attribution and logical identity remain separate.

Expected output preserves push-to-main, manual dispatch, `contents: read`, the
five ordered steps, a failing test exit and attempted evidence retention.
Only the provider exporter introduces `uses`, provider condition syntax and
artifact inputs. No YAML placeholders or unresolved action references may reach
the published output. Use a maintained structured YAML serializer and test YAML
key/scalar handling, including the `on` key, rather than concatenating strings.

Generated YAML is a reviewed, checked-in deployment artifact. `.neu` plus the
lock/settings is its maintained source; manual changes to owned YAML fail a
non-mutating freshness check. Never overwrite an unowned existing workflow:
initial migration requires an explicit diff/adoption action. Export twice with
the same inputs must produce byte-identical files.

Untrusted event strings must not be pasted into shell source. GitHub documents
this injection boundary in [script injection guidance](https://docs.github.com/en/actions/concepts/security/script-injections).
The first package has no privileged pull-request trigger or secret forwarding;
those need a separately reviewed extension and tests.

## 10. Full acceptance test

“Install in the language” means installing into the **project host environment**,
then supplying captured package exports to Neutral. The compiler itself never
installs anything. A complete proof has all of the following independently
observable stages.

### End-to-end sequence

1. Start with a clean project environment containing released compatible Neutral
   tooling and the editor, but no CI packages or provider-specific editor code.
2. Explicitly install the CI contract and GitHub provider packages. Verify content,
   compatibility and dependencies; create the generated integration lock.
3. Open the editor and confirm that CI operations appear dynamically, without
   rebuilding it. The headless catalogue returns the same exact contracts.
4. Model the five-step workflow above using GUI controls, not by pasting finished
   YAML or a prebuilt `.neu` file. Verify invalid connections/fields are rejected.
5. Save `.neu`, close/reopen, and independently compile/decode/compare its meaning.
   Removing only the editor layout file must not change the compiled/exported result.
6. Select `neutral::automation::checks` explicitly and the locked GitHub target.
   Produce a compatibility report, complete bound plan, YAML and receipt.
7. Compare the exported workflow with literal expectations; inspect the diff and
   approve adoption into the repository. A separate parser validates the YAML.
8. Run the actual workflow in an explicitly authorized proof repository/ref, then
   adopt it in Neutral's repository through normal reviewed changes.
9. Verify GitHub really runs `cargo xtask ci pr` at the recorded source revision,
   observes its exit status and retains the expected evidence. Record run ID/URL,
   source revision, export receipt and result; a green exporter test is not enough.
10. Deliberately introduce a test failure in an isolated proof revision. The job
    fails while the always-retention step is attempted; source-linked evidence
    identifies the failure. Restore through a normal new change, not history rewrite.
11. Re-export from unchanged inputs and require no diff. Change a semantic input
    or package/mapping lock deliberately and verify the intended output/context changes.

External pushes, workflow dispatch, package activation and device access require
explicit authority when this proof is eventually executed. This document does
none of them.

### GitHub trigger and credential details

Manual dispatch has repository/default-branch prerequisites; do not assume a
new workflow existing only on an arbitrary branch can already be dispatched.
Use an authorized disposable proof repository or first land a reviewed manual
workflow. See [manual workflow runs](https://docs.github.com/en/actions/how-tos/manage-workflow-runs/manually-run-a-workflow).

Do not promise that a workflow-generated push automatically starts another
workflow: GitHub suppresses most event recursion from its built-in token.
Use an explicitly authorized human/app workflow or supported dispatch mechanism,
with least privilege. See [workflow triggering](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/trigger-a-workflow).

Export does not require a GitHub credential. Publishing/dispatch and private
repository access are separate explicit operations. No token belongs in `.neu`,
semantic bundles, generated receipts or default logs.

### Required test layers

| Test | Required result |
| --- | --- |
| Package schema/integrity | Unknown/duplicate fields, missing files, digest mismatch and unsupported features reject. |
| Installation safety | Traversal, symlink escape, oversized archives and interrupted activation leave no accepted partial installation. |
| Offline/locked use | Supplied locked content supports compile/export without network or version drift. |
| Semantic isolation | Provider binaries cannot run while opening source, discovering types or compiling. |
| Dynamic discovery | Removing a package removes its available catalogue; reinstall adds it without an editor rebuild. Existing unresolved source stays visible. |
| Generic form generation | Required/default/null/choice/list/variant facts are derived from the exact schema. |
| Source/model round trip | Projected source recompiles to intended meaning; comments/omissions/expressions remain editable. |
| Stale edits | Changed source/lock/metadata refuses stale projection without overwriting the last accepted generation. |
| Core versus domain errors | Wrong type is a core error; graph cycle/unsupported provider is a domain/compatibility error. |
| Mapping resolution | Shuffled candidates are deterministic; ambiguity or unmet guarantee is explicit. |
| Command rendering | Spaces, quotes, metacharacters and hostile event input cannot change argv/meaning or inject workflow code. |
| Result checks | Wrong nominal revision/tag/restriction fails before binding or accepted result publication. |
| Export determinism | Same complete inputs yield identical YAML/receipt projections; layout-only edits do not change semantics. |
| YAML correctness | Independent parser/provider schema checks plus literal golden structure; no unknown/unresolved features. |
| Output ownership | Export refuses unowned-file overwrite and path escape; failure leaves previous files intact. |
| Failure/retention | Quality failure stays failure; retention policy cannot mask it. |
| Permissions/privacy | Minimal permissions, no secret embedding, no raw private paths or unintended files in retained evidence. |
| Live GitHub proof | Recorded run executes the real repository command at the intended revision and reports truthful results. |
| Protocol/fault/bounds | Hostile frames, allocation failure, cancellation, crash, output flood and exact/one-over limits fail boundedly. |
| Closed-type integrity | Package installation cannot append a variant tag, spoof a role or bypass exact nominal typing. |
| Open assembly extension, when shipped | An independently typed operation can be selected through checked public links; dangling/private/wrong-contract links fail consumer validation. |
| Cross-domain structural proof | A fake embedded provider installs/discovers/resolves through the same package/host API without changing generic code. |

Keep most tests offline, headless and no-effect. Editor automation exercises the
real form/model/source route; fixtures alone do not substitute for it. Live
GitHub tests are explicit integration gates and record failure instead of quietly
skipping when credentials or required services are absent.

A fake embedded package proves structure only. Real ESP support requires device,
tool/security/transport and safety conformance; a second MCU family later tests
the portability claim. Do not call the CI milestone complete embedded support.

## 11. Self-hosting without a bootstrap trap

The intended claim is precise: **the first application built on Neutral's
integration kit generates and maintains Neutral's own CI**. It is not a claim
that the Rust compiler implementation becomes written in `.neu`, or that Neutral
replaces GitHub's runner or Cargo/Nextest.

Recommended repository assets at adoption time, not files created here:

```text
automation/
  ci.neu
  neutral-project.toml
  neutral.lock
.github/workflows/
  ci.yml
```

Host project/lock names are provisional, not competing copies of `Cargo.toml`
or new mandatory directories in every project. Cargo continues to own Rust
workspace configuration. The integration manifest owns package/target capture.

### Bootstrap rule

Use a previous compatible released Neutral integration tool and locked CI packages
to generate/check the workflow. CI can then build/test the new language and
integration tools as its workload. It must not need the new broken compiler to
generate the only workflow capable of reporting that it is broken.

Initial adoption keeps the existing hand-maintained workflow as a reviewed
bootstrap guard while the generated candidate is proven independently. Avoid
two uncontrolled push workflows doing the same work indefinitely. After proof,
adopt one generated CI workflow and retain a documented repair/bootstrap route.
Changes to workflow ownership or required branch-protection check names require
explicit review; do not silently break merge requirements.

Add a non-mutating generation check using released compatible tooling. It rejects
drift without committing or pushing a regenerated workflow. New tooling is also
tested against the same fixtures as a candidate; changing the release used for
generation is an explicit lock update with compatibility qualification.

When a package exposes features unsupported by the bootstrap tool, upgrade the
tool first through a separately working release route. Never silently downgrade
the source or fetch an unverified latest binary to escape the cycle.

### Success criteria

- A developer can install the packages and reconstruct the editor catalogue
  using published tooling, without copying workspace test helpers.
- The editor creates the maintained `.neu`; headless compilation/export agrees.
- The generated GitHub workflow runs Neutral's real CI, including failure cases.
- Provider/library additions need package registration and exact contract
  implementations, not vendor-specific compiler/editor branches. Extending a
  closed domain aggregate still follows its explicit schema compatibility rules.
- Package updates, source edits and generated output have one inspectable,
  deterministic maintenance path; routine work needs no manual Markdown evidence.
- CI remains repairable when the new compiler/editor/package is broken.

This is the smallest convincing self-hosted vertical slice. Broader CI providers,
device execution and cloud/Linux engines follow as independent integrations over
the same proven host/package contract.

## 12. Delivery order and approval boundaries

### A. Accept and qualify the post-v2 integration boundaries

Confirm the completed language/reader/evaluator/authoring availability, then review
the package envelope, operation index, mapping/host protocol and trust model.
Reuse v2 SDK surfaces rather than adding a competing authoring or value engine.
Choose one maintained source of truth for every schema and generate bindings.

Exit: an independent consumer can use the public foundation, and every new
protocol has literal positive/negative/boundary oracles and explicit ownership.

### B. Prove data installation and dynamic authoring

Implement the smallest verified local-package store and CI semantic package.
Render generic controls from descriptors, project source and prove round trips.
No registry, cloud service, arbitrary UI plugin or executable adapter is needed
to demonstrate this phase.

Exit: installing a previously unknown CI type adds usable controls without
editor/compiler changes; headless APIs expose identical facts.

Source-surface qualification must also cover multi-job CI, shared embedded
resources and cloud/Linux dependency graphs, not only nested forms. Test grammar
selection, interleaved construction scope, list order, reuse/ref distinctions and
editor round trips against independent semantic reading. Keep closed-domain
variants separate from checked third-party operation assembly. The syntax proposal
is not frozen until these examples and its compatibility amendment are reviewed.
Package/mapping work can use existing supported source meanwhile; optional TOML
capture and construction aliases are not prerequisites for this vertical slice.

### C. Prove binding and offline GitHub export

Implement the CI domain checks, locked GitHub mappings and one exporter through
the host protocol. Test exact argument handling, permissions, source mappings,
ownership and deterministic output against literal fixtures.

Exit: the source above produces complete supported workflow YAML and a verified
receipt; unsupported behavior refuses export. No live execution claim yet.

### D. Run the full editor-to-GitHub self-CI proof

Execute the authorized acceptance sequence, including the deliberate failure and
retention case. Adopt reviewed generated CI and its non-mutating freshness gate
only after the bootstrap path is known to work.

Exit: evidence links package installation, editor projection, artifact/plan,
export and actual GitHub run. The first usable Neutral application exists.

### E. Challenge generality with another domain

Install a fake embedded integration through unchanged generic APIs, then implement
one real read/inspect operation under explicit safety policy. Use the experience
to refine optional protocol roles without changing already shipped semantics.
Cloud/Linux integrations follow their own domain-specific requirements.

Exit: structurally common packaging/discovery/binding works, and shared code is
based on observed reuse rather than speculative universal execution.

### Approval and scope

Review this architectural proposal before writing a detailed implementation plan.
The important decisions are the package-role model, host/domain boundary,
non-executable editor discovery, exporter-first CI and observable self-hosting
acceptance test. Manifest field names and physical repository placement are not
yet frozen release interfaces.

Do not copy this proposal wholesale into the current v1 operational checklist.
After acceptance, register the necessary post-v2 requirements in one authoritative
plan with executable tests. No implementation approval, package activation,
external publication or CI replacement is implied by this document.

## References and verification

- [Ecosystem-readiness audit](v2-ecosystem-readiness-audit.md): requirement gaps,
  proposed owners and cross-domain evidence.
- [Authoring contract](../portable/specs/contracts/AUTHORING.md): descriptors,
  separate metadata, source-preserving projection and stale revisions.
- [Consumer protocol](../portable/specs/contracts/CONSUMER-PROFILE.md): exact
  contracts, entries, typed selectors, safe attribution and context separation.
- [Vocabulary contract](../portable/specs/contracts/VOCABULARY.md): source `use`,
  exact captured closure and data-only semantics; later composition profiles
  extend types without granting acquisition or effects.
- [Evaluator contract](../portable/specs/contracts/EVALUATOR.md): bounded explicit
  bindings and separation from execution.
- [Current CI](../.github/workflows/ci.yml) and
  [developer workflow](development.md): the existing workload to preserve, not
  already a package-driven/self-hosted implementation.
- Official GitHub syntax, injection, manual-run and triggering documentation is
  linked beside the specific provider constraints above. These constraints must
  be revalidated when the exporter implementation is qualified.

Document verification: `cargo xtask check` passed, including workspace compilation,
Markdown links, traceability, dependency boundaries and repository hygiene.
Whitespace checks passed. This verifies the documentation change, not the proposed
package, editor or live CI acceptance tests.

**Not checked:** No proposed package/example was implemented, no editor interaction
or GitHub/device execution was performed, and no new runtime protocol was fuzzed.
The architecture and examples require review and contract qualification before
they can become working or supported features.
