<!-- SPDX-License-Identifier: Apache-2.0 -->

# Human-readable Neutral source: typed tables and explicit variants

[< Documentation hub](README.md)

Status: proposal for review, 13-10-2026. No parser, source contract, fixture pin,
compatibility promise or implementation checklist is changed by this document.

Future planning direction: hybrid typed authoring, reviewed against the runtime
and ecosystem plans. The grammar remains a draft; the review gates below must
pass before specification freeze or implementation activation.

This answers the readability problem in the
[self-hosted CI example](v2-runtime-and-package-architecture.md#proposed-neu-example).
The goal is a pleasant hand-written language and a stable editor projection, not
another serialization of `.nir` and not a CI-specific DSL.

All new `.neu` forms below are **proposed, not runnable syntax**. The retained
`neu "1.0"` header illustrates the semantic context, not automatic activation.
Acceptance requires a separately selected source-syntax profile, reviewed
capture/compatibility rules and new literal fixtures. Existing capture profiles
must continue to interpret or reject the same source bytes as before.

## 1. Recommendation

Add a **TOML-inspired typed table surface to `.neu`**, with explicit variant
selection. Keep ordinary expressions and compact literals where useful. Make
typed TOML ingestion an optional, separately specified host frontend, not a
compiler-side `include` that opens arbitrary files.

Tables are the default for configuration, not the mandatory form of every value.
Use tables for containment, named typed bindings/references for shared graphs,
variants for choices, and pure expressions/templates for computation and reuse.
Do not force all future applications into a nested configuration tree.

The central changes are:

- `[public ci::Workflow checks]` starts one typed record binding.
- `name = expression` supplies a field; dotted keys construct nested records.
- `[[checks.jobs as quality]]` appends a record to an ordered list, with an
  optional authoring-local label for addressing its children.
- `[[checks.jobs.quality.steps @command]]` appends the `command` alternative of
  the expected variant type. Its body supplies that alternative's record payload.
- `@success("done")` supplies a variant in an expression without spelling its
  wire-oriented `tag` and `payload` fields.

The vocabulary still owns `Workflow`, `Job`, `Step`, field names, restrictions
and alternatives. Neutral owns the generic construction syntax. Installing a
library does not add parser keywords or execute anything.

### Options audited

| Option | Benefit | Cost / limitation | Recommendation |
| --- | --- | --- | --- |
| Keep braces; split values into named bindings | Small implementation impact; uses the existing model. | Still nested literals and variant wrappers; extra bindings can affect complete logical identity and provenance. | Useful today where supported, not the main authoring solution. |
| Add only variant shorthand | Removes most `tag`/`payload` noise without changing record layout. | Deep records and lists still require nested braces. | Useful part of the table proposal. |
| Typed tables in `.neu` | Flat readable sections, retained types, references and expression semantics; no wire wrappers. | New parsing, attribution, formatting and authoring-model work; repeated qualified paths. | Preferred direction for review. |
| Plain TOML as captured typed data | Familiar format and existing editing tools. | Not a replacement for Neutral expressions, references, nullability or numeric rules; needs a capture adapter. | Optional interoperability path, not the primary language. |
| YAML-style indentation or package-defined block grammar | Fewer visible delimiters. | Introduces another nesting/scope model or makes source grammar depend on installed libraries. | Do not introduce for this requirement. |

This is a source ergonomics change. It does **not** require redesigning the
vocabulary schema, making records untyped, or changing the binary artifact into
TOML. Keep one semantic type/value model, regardless of input surface.

## 2. The complete CI example, without nested record braces

Proposed `.neu`:

```neu
neu "1.0"
module neutral::automation
use NeutralCi as ci

[public ci::Workflow checks]
name = "Neutral CI"
push_branches = ["main"]
manual = true

[[checks.jobs as quality]]
id = "ci"
runner.os = "linux"
runner.architecture = "x86_64"

[[checks.jobs.quality.steps @checkout]]

[[checks.jobs.quality.steps @rust_toolchain]]
channel = "stable"
components = ["rustfmt", "clippy"]

[[checks.jobs.quality.steps @install_tool]]
tool = "cargo-nextest"

[[checks.jobs.quality.steps @command]]
executable = "cargo"
arguments = ["xtask", "ci", "pr"]

[[checks.jobs.quality.steps @retain_artifacts]]
name = "workflow-evidence"
paths = [
    "test-results/workflows",
    "test-results/tests",
    "target/nextest/ci/junit.xml",
]
always = true
retention_days = 14
```

There is still one public `checks` binding. Jobs and steps remain ordered list
elements, not new module-level declarations. The five selected alternatives,
payload values, empty checkout payload and artifact paths are unchanged in
intended materialized meaning.

`quality` is a local construction label. `id = "ci"` is actual domain data.
They are deliberately different: labels must not secretly fill a field, become
a job identifier or alter consumer scheduling.

An empty `@checkout` section means an explicitly supplied empty record payload.
It succeeds only if the selected payload's required/defaulted-field rules allow
that record. It is not an omitted payload, unit value or implicit null.

This still describes a workflow. Compilation must not check out a repository,
install Rust, run Cargo or upload anything. A compatible consumer validates and
exports or executes it under separately granted authority.

## 3. Table construction rules

### 3.1 Typed roots and nested records

The proposed header shapes are small and package-independent:

```text
binding_table = "[" ["public"] type binding ["@" tag] "]"
child_table   = "[" construction_path ["@" tag] "]"
array_row     = "[[" construction_path ["@" tag] ["as" label] "]]"
field_entry   = field_path "=" expression
```

Names, types and expressions retain their own grammar. Tags and row labels are
literal identifiers, not arbitrary expressions. Header variants with `@tag`
construct a record payload of the exact expected variant; other table headers
construct an ordinary record. A scalar-payload variant uses expression syntax.

The root header retains Neutral's type-before-name order and optional `public`:

```neu
neu "1.0"
module example

public record Service {
    string image,
    List<string> labels = [],
}

[public Service api]
image = "example.invalid/api:1"
labels = ["portable", "typed"]
```

Type declarations remain small, explicit schema definitions. Their braces are
not a problem in the same way as five levels of configuration nesting. Do not
invent a second table-based type-definition language just to remove every brace.

Nested record values may use dotted keys or a dedicated section. Given a
proposed `Service` contract with a `resources` record field:

```neu
[public app::Service api]
image = "example.invalid/api:1"
resources.cpu = 2
resources.memory_mib = 256
```

Alternatively, in a separate source example:

```neu
[public app::Service api]
image = "example.invalid/api:1"

[api.resources]
cpu = 2
memory_mib = 256
```

Paths are rooted in a table-constructed binding in the current module. `.`
addresses construction fields/row labels; `::` continues to qualify logical
modules and nominal types. Neither is a filesystem path.

This construction-path notation does not define expression-level member access.
For example, `resources.cpu = 2` supplies a field; reading `settings.cpu` on a
right-hand side requires the separately reviewed typed projection semantics in
the [ecosystem audit](v2-ecosystem-readiness-audit.md#13-v2-13--add-bounded-structured-value-projection-to-pure-expressions).

The compiler knows each field's expected type. A table is not an untyped bag of
keys followed by a best-effort conversion. Unknown fields and type mismatches
remain errors, with the original field/header span.

### 3.2 No hidden parent selection for nested lists

Every array header appends one element. A label is required only when later
sections need to address that particular element:

```neu
[[checks.jobs as test_job]]
id = "test"
runner.os = "linux"
runner.architecture = "x86_64"

[[checks.jobs as lint_job]]
id = "lint"
runner.os = "linux"
runner.architecture = "x86_64"

[[checks.jobs.test_job.steps @command]]
executable = "cargo"
arguments = ["xtask", "test", "all"]

[[checks.jobs.lint_job.steps @command]]
executable = "cargo"
arguments = ["clippy", "--workspace"]
```

The test step still belongs to `test_job`, even though `lint_job` was declared
more recently. Every traversal through a list requires a declared label. Reject
`[[checks.jobs.steps @command]]`; do not attach it to the most recent job.

This intentionally differs from TOML's most-recent-array-element rule, which is
convenient but makes a section move capable of changing its parent. The ordering
and parent-selection rules are specified in the official
[TOML arrays-of-tables section](https://toml.io/en/v1.1.0#array-of-tables).

Labels are unique within their containing list, not globally. They are not
runtime keys, do not turn a list into a map, and cannot be used as `ref` targets.
Changing a label together with its construction paths changes source evidence,
not materialized logical values. Changing row order changes the ordered list.

If a variant row needs a nested payload record section, give the row a label:

```neu
[[checks.jobs.quality.steps @command as verify]]
executable = "cargo"
arguments = ["xtask", "ci", "pr"]
```

The proposed suffix order is fixed: selected tag first, optional `as` label
second. An ordinary record row has only optional `as label`.

### 3.3 Single assignment, not imperative mutation or configuration merging

`field = expression` contributes to one immutable construction. It is not an
assignment statement executable later.

Reject all of the following:

- The same field defined twice, even if its values are equal.
- `runner = { ... }` combined with `runner.os = ...`.
- A list assigned with `steps = [...]` or `steps = []` and then appended with
  `[[...steps]]`.
- Reopening the same explicitly declared record table/header.
- Defining a child row before its labelled parent.
- Constructing fields inside an imported value, reused value or reference target.
- A path continuing through a scalar, unknown field or unlabelled list element.

Multiple dotted fields may build the same otherwise-unassigned nested record.
An explicit child table may do the same, but do not allow both forms to reopen
that record. The rule must be mechanical, independent of evaluation order.

Required fields and defaults are resolved after the whole binding's construction
has been assembled. A parent header need not be complete before its child
sections. No partial binding, artifact or cache generation is published.

Construction scope must be frozen with these rules and literal examples:

- A binding's child sections may be separated by unrelated declarations. Their
  explicit paths retain ownership; entering another table does not reparent them.
- Ordinary binding-name resolution remains independent of declaration order.
  The earlier-parent requirement applies to construction headers/row labels, not
  to ordinary binding references.
- Value reuse and expression dependencies cannot form value cycles. Typed
  reference cycles retain their separate language rules; they do not establish
  a valid consumer scheduling graph.
- Moving independent record sections preserves materialized values, while moving
  array rows can change ordered lists. Moving a section does not rename its parent.

If a nested record field has a record-valued default, supplying one of its
dotted children supplies a **new record value**, not an override merged into the
old aggregate default. Missing children follow the nested record type's ordinary
field defaults/requiredness. This avoids a second, hidden defaulting algorithm.

A table constructs a non-null record. It may inhabit a nullable record position
under the same ordinary type rule as a non-null record literal. It cannot
traverse or amend an already assigned `null`. Explicit null values use `null`;
omission remains omission.

### 3.4 Parsing boundaries and comments

At declaration level, `[Type name]` and `[[path ...]]` are table headers. On the
right of `=`, `[a, b]` remains an ordinary list expression. Headers must occupy
their own statement, outside every expression delimiter.

A table body ends at the next header, module-level declaration or end of file.
Line breaks end complete field entries; explicit expression delimiters permit
multiline values. Indentation improves readability but never determines scope.
Header targets are explicit; they do not depend on indentation or the previous
table. Initially prohibit table headers inside `if`, `match` and template bodies.

Retain Neutral's existing comments and string rules. Do not import TOML's
comments, dates or string normalization accidentally merely because table
headers look familiar. Do not require SPDX comments in `.neu` examples.

## 4. Variants should look like typed choices, not transport envelopes

The existing [variant contract](../portable/specs/contracts/VARIANTS.md) uses
`tag` and `payload`. The proposed source shorthand should lower to the same
nominal alternative and typed payload, not create another variant model.

Proposed expression forms:

```neu
public variant Outcome {
    string success,
    num failure,
    string? absent,
}

Outcome ready = @success("complete")
Outcome failed = @failure(3)
Outcome missing = @absent(null)
List<Outcome> results = [ready, @failure(7), missing]
```

The expected type must uniquely resolve the variant owner. `@success` is not a
free-standing global function and its spelling does not infer a library/type.
Two unrelated variants with a `success` tag remain distinct nominal types.
An untyped or ambiguous constructor is an error; do not add `Any` to make it fit.

For a record payload, table headers provide the same choice without a nested
`payload` table, including at a binding root:

```neu
[public ci::Step verify @command]
executable = "cargo"
arguments = ["xtask", "ci", "pr"]
```

In a separate expression-based example, a compact literal is still useful:

```neu
ci::Step verify = @command({
    executable: "cargo",
    arguments: ["xtask", "ci", "pr"],
})
```

For a scalar, list or nullable payload use `@tag(expression)`, not a special
table field called `value`. A record-payload table is not valid for those payload
types. No missing payload is silently converted to `{}` or `null`.

Unknown tags, missing arguments, extra arguments, wrong payload types and
duplicate fields remain errors. Vocabulary restrictions, public type closure,
recursive-type rules and independent variant/value limits still apply.

This syntax does not make a closed variant extensible by installing a plugin.
A new operation must belong to a compatible declared contract or to an explicitly
designed domain composition boundary. See the
[closed-type extension boundary](v2-runtime-and-package-architecture.md#dynamic-discovery-does-not-make-closed-types-open).

## 5. Keep the features that make this Neutral, not just TOML

The vocabulary names in this section are illustrative package contracts, not
implemented libraries. Examples describe source behavior, not executable tests.

### Expressions and guarded evaluation

Table fields accept the same typed expressions as other values:

```neu
neu "1.0"
module example
use ExampleDomain as domain

input num replicas
input bool maintenance

[public domain::Deployment app]
name = "api"
replicas = if maintenance { 0 } else { replicas }
enabled = !maintenance && replicas > 0
share = 1 / 3
```

`share` follows the separately frozen deterministic decimal arithmetic contract,
including bounded approximation of nonterminating division; do not reinterpret
it as a binary float or make `1 / 3` a new error. All branches type-check, but only
selected branches evaluate or demand external bindings. A table must not evaluate
deferred fields early just to assemble its structure.

`if`, exhaustive `match`, interpolation, typed selectors, templates and matrices
keep their separately specified ownership and semantics. Table syntax does not
turn deferred values into static ones. Static table paths/labels/tags cannot be
computed from runtime inputs; table topology remains compile-time bounded.

For example, a selected string can remain an ordinary expression:

```neu
string status = match outcome {
    success(message) => message,
    failure(code) => $"failed: ${text(code)}",
    absent(reason) => "absent",
}
```

This is an expression example using the proposed `Outcome` above, not a complete
module. Braces here delimit branch bodies, not dozens of nested configuration
records. Their removal is not needed to solve the original problem.

### Reuse and references

```neu
neu "1.0"
module example
use ExampleDomain as domain
import shared::settings as settings

[public domain::Deployment app]
name = "api"
labels = settings::labels
```

Reuse stays ordinary immutable reuse. A typed reference stays `ref(binding)`
where the declared field type permits it. Neither should be encoded as a magic
string such as `"$app"` or automatically resolved from a TOML key.

Construction labels are deliberately not bindings. If a consumer needs stable
cross-job references, its contract must expose explicit domain identifiers or
ordinary typed referenced bindings. Do not let a formatter's local row labels
accidentally become part of a public API.

### Shared graphs instead of deeper construction paths

For embedded, cloud and Linux applications, shared resources are often clearer
as named values than as nested rows. This hypothetical vocabulary explicitly
declares `Sensor.bus: Ref<Bus>` and `Project.sensors: List<Ref<Sensor>>`:

```neu
neu "1.0"
module example::shared_device
use ExampleEmbedded as embedded

[public embedded::Bus control_bus]
kind = "i2c"
frequency_hz = 400000

[public embedded::Sensor temperature]
bus = ref(control_bus)
address = 72

[public embedded::Project board]
name = "room-sensor"
sensors = [ref(temperature)]
```

Several sensors can share `control_bus`; a formatter must not inline/copy that
binding or silently replace references with reuse. All exposed targets must
satisfy public closure. Domain validation still owns address, frequency, wiring
and scheduling constraints; a `num` field alone does not prove integrality.

Extracting an existing nested value into a named binding is a semantic refactor,
not mere formatting: it can alter declarations, reference edges, complete project
identity and provenance. The editor must make that change explicit.

Try named bindings and short explicit roots before adding construction aliases.
Lexical path aliases remain deferred until real workloads prove they are needed;
do not introduce relative/current-parent shorthand in their place.

### Embedded configuration: same language, different vocabulary

```neu
neu "1.0"
module example::device
use ExampleEmbedded as embedded

[public embedded::Project sensor]
name = "room-sensor"
target.board = "sensor-board"
target.architecture = "riscv32"

[[sensor.operations @compile]]
optimization = "size"

[[sensor.operations @flash]]
device = "bench-device"
verify = true
```

The hypothetical vocabulary owns the target constraints and operation meanings.
The device name is an inert symbolic handle, not permission to open a device.
The board name is illustrative; the selected package must validate the actual
board/architecture combination rather than infer it from a string prefix.

### Linux configuration: descriptions, not privileged statements

```neu
neu "1.0"
module example::machine
use ExampleSystem as system

[public system::Plan workstation]
name = "developer-tools"

[[workstation.resources @package]]
name = "git"
state = "present"

[[workstation.resources @directory]]
path = "/opt/example"
mode = "0755"
```

`package` and `directory` remain vocabulary alternatives. A consumer chooses a
compatible mapping and validates privileges/target applicability. Parsing this
file does not install Git or create a directory. The path spelling above follows
the illustrative contract's string field, not a new implicit host-path primitive.

## 6. Could the configuration be a real TOML file instead?

Yes, for static typed data, through an explicit captured-input adapter. It should
be an alternative entry frontend, not a second mandatory file for every program.

### 6.1 An ordinary TOML representation of the CI data

This block is actual TOML, unlike the typed `.neu` headers:

```toml
name = "Neutral CI"
push_branches = ["main"]
manual = true

[[jobs]]
id = "ci"

[jobs.runner]
os = "linux"
architecture = "x86_64"

[[jobs.steps]]
tag = "checkout"
[jobs.steps.payload]

[[jobs.steps]]
tag = "rust_toolchain"
[jobs.steps.payload]
channel = "stable"
components = ["rustfmt", "clippy"]

[[jobs.steps]]
tag = "install_tool"
[jobs.steps.payload]
tool = "cargo-nextest"

[[jobs.steps]]
tag = "command"
[jobs.steps.payload]
executable = "cargo"
arguments = ["xtask", "ci", "pr"]

[[jobs.steps]]
tag = "retain_artifacts"
[jobs.steps.payload]
name = "workflow-evidence"
paths = [
    "test-results/workflows",
    "test-results/tests",
    "target/nextest/ci/junit.xml",
]
always = true
retention_days = 14
```

The wrappers remain in this direct representation because TOML has no native
nominal-variant constructor. Keeping those fields is preferable to guessing a
variant from a payload's shape or interpreting arbitrary dotted keys as tags.
The native TOML frontend must honor TOML's own parent rules; the `.neu` label
extension must not be smuggled into a file advertised as standard TOML.

### 6.2 How `.neu` could use it without compilation-time acquisition

An explicitly invoked future host capture adapter would:

1. Read a declared `automation/ci.toml` under the host's path/security policy.
2. Resolve its exact expected `NeutralCi` revision and `Workflow` type from the
   project lock; do not discover a type from untrusted file fields.
3. Parse a specified TOML subset while retaining original number tokens and
   checked source spans; validate one typed value with ordinary Neutral rules.
4. Supply a deterministic captured module such as `generated::workflow` with a
   public `workflow` binding. Generated bytes and relevant adapter/profile facts
   belong to capture/derivation evidence; preserve the original TOML byte digest.
5. Let the `.neu` file import the resulting logical module normally:

```neu
neu "1.0"
module neutral::automation
use NeutralCi as ci
import generated::workflow as config

public ci::Workflow checks = config::workflow
```

The compiler receives captured inputs; it does not open the TOML path, fetch a
package, read an environment variable or retry missing imports through a loader.
The manifest association between file, logical module, binding and exact type
must be reviewed as part of the host package/capture contract before implementation.
No new installer command or manifest field is defined by this proposal.

The data binding is **not** an external runtime `input`. Its values are project
meaning. Calling it an input merely to bypass capture would put its identity and
validation in the wrong layer. Module packaging may add declarations to complete
project meaning; do not promise that this wrapper has the same complete identity
as an unrelated one-binding source program.

### 6.3 Necessary restrictions on the TOML boundary

TOML has no native null, reference or Neutral-expression value; it does have
date/time values and floating-point special values outside Neutral's core data
model. Its duplicate-key and numeric rules are not interchangeable with a
Neutral type checker. These format facts come from the
[official TOML specification](https://toml.io/en/v1.1.0).

Consequently, the adapter needs explicit rules:

- Reject unsupported date/time and non-finite numeric values; never guess strings
  or silently normalize them into other Neutral types.
- Preserve decimal token precision. A generic parse through `f64` followed by a
  decimal conversion is not acceptable for `num`; validate representability and
  bounds before accepting the value. Unsupported numeric forms must reject
  explicitly rather than truncate or silently round.
- Reject heterogeneous values when the expected Neutral list element type does
  not permit them. TOML acceptance is not Neutral acceptance.
- Initially use omission/defaults only where the exact Neutral contract permits
  them. Do not invent `"null"`, `"$null"` or an empty table as a null sentinel.
  Required explicit nullable/reference/expression values may simply be outside
  this frontend's initial scope.
- Do not treat strings beginning with `$`, `ref(...)` or a URI as executable
  expressions, references or acquisition instructions.
- Retain actual original-byte attribution, including escapes and multiline
  strings. Generated `.neu` spans are not automatically TOML spans. If a checked
  mapping is unavailable, report attribution unavailable rather than invent it.
- Bound parsing, key/path depth, numbers, strings, rows, defaults, validation work
  and generated output before proportional allocation; honor cancellation.
- Keep the resulting artifact self-contained. An independent reader must not
  need to reopen the TOML file to understand a value or enforce its restrictions.

Start with native `.neu` authoring. Add this adapter only when a concrete package
needs TOML interoperability; do not build two new frontends in the first slice.

## 7. Semantic and usability audit

| Concern | Decision / risk control |
| --- | --- |
| Too much punctuation | Table bodies have `key = value`; record/list nesting moves to headers, variant wrappers disappear. Ordinary list brackets remain. |
| Very long paths | Use short root names and local row labels, editor breadcrumbs and completion. No relative-header or implicit-current-parent shorthand in the first design. Review a multi-job example before freeze. |
| Hidden schema behavior | Header names, fields and tags resolve against exact public types. No inferred discriminator, field injection or plugin-defined grammar. |
| Ordered lists | Array-header order is value order. Do not sort jobs, steps, arguments or paths while formatting. Labels are not ordering keys. |
| Maps | A list of entries remains a list unless its contract explicitly provides checked map semantics. Do not infer map semantics from named rows or silently discard duplicate domain keys. |
| Defaults and null | Omitted field, supplied null, supplied empty record and nested defaults retain distinct origin facts. No aggregate-default deep merge. |
| Reuse and references | Keep ordinary language rules; construction paths cannot mutate a reused/imported value or dereference a target. |
| Public/private closure | A public root closes over all required types/alternatives and referenced bindings. Table syntax cannot expose a private imported member. |
| Expressions | Field RHS uses the shared typed expression model. Names resolve/type-check in every branch; selected branches alone evaluate. |
| Expansion | Table paths, row labels and selected tags are static. Templates/matrices use the approved hygienic finite-expansion boundary, not header interpolation. |
| Source attribution | Every header, field, selected tag, label, omitted/defaulted value and generated construct needs bounded checked attribution. A label is authoring information, not semantic origin. |
| Identity | Compare equivalent forms under the same new profile. Preserve logical values/declarations, ordered lists, nominal owners and restrictions; never equate capture/source evidence. Do not promise cross-profile digest equality. |
| Artifact format | Construction syntax is lowered away. Independent decoding still uses a governed self-contained binary contract; changing source spelling alone is not permission to rewrite `.nir`. |
| Editor round trips | Preserve table/literal choice, comments, labels, omissions and section placement in the authoring model. Semantic IR alone cannot reconstruct that choice. |
| Host safety | `use`, table construction and variant selection remain data-only. File acquisition, command mapping, credentials and execution stay with explicit consumers/hosts. |
| Compatibility | New syntax is selected explicitly. Older profiles reject it; no parse-failure fallback or silent reinterpretation. New source/transcript facts need reviewed profile boundaries. |
| Resource exhaustion | Check header/path/label/row/value/work/output limits and cancellation before expansion/allocation. Exhaustive failure sweeps must cover builder finalization and cache publication. |

### Do not solve a schema problem with syntax alone

The CI example also needs a well-designed vocabulary: typed payloads, meaningful
defaults, clear identifiers, explicit edges and reusable domain types. A readable
table cannot make a provider's arbitrary shell text portable or a closed variant
open. Conversely, do not flatten a correctly nested domain model merely to make
its transport-shaped source shorter.

With a tiny configuration, the current literal may remain clearer. Supporting a
table surface need not forbid compact record literals or expressions. The formatter
should preserve the author's chosen representation, not endlessly flip between
equivalent forms or create unnecessary top-level bindings.

## 8. Where this belongs in Stage 8 and Stage 9

Use Stage 8 for the **source-design and authoring-extension review**, before
freezing an authoring model that cannot represent tables. Implement there only
after the amendment and its execution plan are approved. Stage 9 qualifies the
finished profile; it should not be the first place its syntax is invented.

Keep the extension separate from the existing foundation's completion criteria.
Do not silently make all v1 delivery depend on the new surface. If adopted after
v1, select a successor source/authoring profile instead of retroactively adding
requirements to an already frozen gate. Optional TOML ingestion is a separate
milestone and must not block the package/mapping vertical slice.

Several existing Stage 8 contracts and fixtures are already frozen. Approval of
this proposal must produce a versioned source/authoring amendment and new pinned
inputs, not edits that reinterpret historical approved bytes. Keep the current
expression, expansion, composition, wire and identity vectors immutable.

The implementation reach is broader than adding `[` to one parser:

| Owner | Required work after approval |
| --- | --- |
| Source/capture contract | Specify header/body grammar, builder ownership, labels, duplicates, visibility, profile selection, resource controls and deterministic diagnostic order. |
| Lexer and all source/header consumers | Recognize the new punctuation/context without breaking lists, imports, headers, comments, recovery or cached header discovery. |
| Compiler parsing and semantic construction | Build one private typed construction representation; lower tables and shorthand into the shared value/expression model. No second record/type validator. |
| Defaults, companions and identity | Preserve supplied/default/reuse facts, exact field/tag spans and intended logical equivalence; bind source-format facts to the correct identity layer. |
| Formatter and repository source tooling | Preserve table styles and list order; update only maintained mutable inputs, never bulk-reformat archived/frozen fixtures. |
| Public authoring model and editor | Represent tables, labels, comments and construction paths; stale-revision checks, completion and safe round-trip projection. |
| IR, encoding, reader and probe | Reuse unchanged semantic models where sufficient. If retained authoring/derivation facts require new formats, version those facts explicitly rather than leak construction paths into logical values. |
| Cache and test infrastructure | Key parsing by exact syntax profile; clean/warm/changed equivalence, cancellation/failure recovery, independent decode, bounds and fuzz coverage. |
| Guides and packages | Update examples, syntax highlighting and documented commands; do not let libraries define their own incompatible table dialects. |

A minimal implementation sequence after design approval would be: typed record
roots and child records; ordered record/variant rows and labels; shorthand
variants; formatter/authoring/diagnostics; full compatibility and adversarial
qualification. Add the TOML adapter as a separate requirement only if selected.
Do not announce any slice as complete merely because one pretty CI file parses.

## 9. Required acceptance examples before activation

Register literal source, expected typed results, source origins and diagnostic
oracles before implementation. These are proposed future tests, not passing
results of this documentation change.

| Case | Required outcome |
| --- | --- |
| Full five-step CI example | Same intended `Workflow` value and list order as its record-literal counterpart under the new profile. |
| Core/source-declared record table | Works without a vocabulary or runtime package. |
| Vocabulary record table | Resolves exact owner/revision; private or missing types reject. |
| Two jobs with interleaved child sections | Explicit labels keep each step in its declared parent; row order remains the declared list order. |
| Missing parent label / forward row label | Deterministic construction error, not latest-parent inference. |
| Duplicate row label | Reject within its list; the same label in a different list is independent. |
| Label rename with all paths updated | Same normalized logical value under the same profile; different captured/source evidence. |
| Reordered list rows | Different ordered value/identity when order differs. |
| Reordered record fields/sections | Same normalized logical value when declarations and ordered rows are otherwise unchanged. |
| Duplicate key / repeated header / prefix conflict | Reject; no last-write-wins, implicit reopen or deep merge. |
| Literal list followed by array headers | Reject, including an explicitly supplied empty list. |
| Nested aggregate default plus supplied child | Apply the newly supplied nested record's ordinary field rules, not a merge with the aggregate default. |
| Omitted, null, empty and defaulted fields | Appropriate meaning equivalence where specified, with distinct checked source origins. |
| Record, scalar, list and nullable variant payloads | Exact expected owner, tag and payload rules; heterogeneous variant collections remain typed. |
| Empty record-payload variant table | Accept only if its payload contract permits an explicitly supplied empty record. |
| Unknown/wrong tag or missing/wrong payload | Reject with safe tag/payload attribution. |
| Same tag in unrelated nominal variants | No implicit conversion or global tag inference. |
| Existing `tag`/`payload` literal under historical profiles | Preserve old accepted/rejected bytes, values and frozen vectors. |
| Reuse and typed references | Preserve reference/reuse distinction and public closure; labels cannot become reference targets. |
| Unknown field / path through scalar, reused value or ref | Reject rather than create an untyped table or dereference a binding. |
| Multiline lists, comments, strings and `[` inside a string | No header confusion; exact original-byte spans and stable formatting. |
| Guarded deferred value with unavailable unselected branch | Type-check both branches; do not demand/evaluate the unselected branch. |
| Nonterminating division | Use the frozen decimal semantics and bounds, not TOML float conversion or a new arithmetic rule. |
| Runtime value in a header/tag/row label | Reject dynamic topology rather than secretly expanding at runtime. |
| Static finite template/matrix output | Existing expansion semantics and bounds; no accidental source-label identity leakage. |
| Exact/one-over header, path, label, row and aggregate controls | Bounded failure before proportional allocation; no partial accepted results. |
| Cancellation / deterministic allocation failure at every builder boundary | Safe error, prior cache generation intact, subsequent request succeeds. |
| Clean, warm-cache, changed-source and shuffled/concurrent construction | Equivalent intended results and diagnostics; ordered values never shuffled. |
| Independent encoding/reader/public-view traversal | Complete typed values and restrictions, correct privacy/closure; no source or TOML reopening. |
| Editor open/edit/project/reopen | Preserves intended semantics, source form, omissions/comments and checked attribution; stale edits fail safely. |
| Source header and captured syntax selection disagree | Reject before body interpretation; no grammar guessing or retry fallback. |
| Child sections interleaved with unrelated bindings | Explicit ownership and ordinary declaration-order-independent resolution remain intact. |
| Shared embedded bus / cloud or Linux resource graph | Retain reference sharing and public closure; do not duplicate resources into a containment tree. |
| Reuse cycle versus typed-ref cycle versus scheduling cycle | Apply the correct core or consumer rule, with distinct diagnostic ownership. |
| Third-party operation outside a domain's closed variant | No implicit tag extension; require the reviewed checked assembly contract or reject. |
| TOML frontend, if later selected | Exact numeric tokens, unsupported-value rejection, bounded capture, checked original spans and complete independent artifact. |

## 10. Review gates before freeze

Carry the hybrid direction into future planning, but resolve these gates before
touching implementation:

1. Standardize bootstrap-readable source grammar selection. Independent tools
   must discover it from the source header without parsing the body; captured
   metadata must agree. Exact header spelling and profile identifiers still need
   written review. The example `neu "1.0"` alone does not select table syntax.
2. Freeze construction scope, duplicate/default behavior, interleaved sections,
   ordinary binding dependencies and their distinction from row labels.
3. Prove nested multi-job CI and shared embedded/cloud/Linux resource graphs.
   Use tables, named bindings and references where each is appropriate; do not
   add path aliases merely to shorten the first example.
4. Prove closed variants and independently supplied operation assembly without
   `Any`, implicit subtyping or library-defined syntax.
5. Review structured expression projection separately from dotted construction
   keys, with the shared evaluator and compatibility requirements intact.
6. Prove editor open/edit/project/reopen for each source form, with source origins,
   ordered rows, identity-aware semantic refactors and stale-edit rejection.
7. Review the written grammar/amendment and implementation plan. Retain compact
   literals, keep historical profiles immutable, and schedule optional typed TOML
   capture independently of the first integration package.

The CI example is the first readability check, not proof that the surface suits
all future applications. Preserve the acceptance matrix as a required qualification
gate, not optional implementation guidance.

## References and verification scope

- [Source syntax](../portable/specs/contracts/SOURCE.md) and
  [project capture](../portable/specs/contracts/PROJECT.md): current logical
  module/import, binding and no-acquisition boundaries.
- [Variants](../portable/specs/contracts/VARIANTS.md): frozen nominal tag/payload
  meaning being preserved, not silently amended.
- [Expressions](../portable/specs/contracts/EXPRESSIONS.md),
  [expansion](../portable/specs/contracts/EXPANSION.md) and
  [successor compatibility](../portable/specs/contracts/EXPRESSION-PROFILE.md):
  typed semantics and historical-profile isolation to retain.
- [Runtime/package architecture](v2-runtime-and-package-architecture.md): package,
  editor, provider and execution ownership behind the CI example.
- [Stage 8 checklist](../portable/development/07-IMPLEMENTATION-CHECKLIST.md):
  existing contract approval and remaining implementation gates.

This document is a design audit, not parser conformance evidence. Standard TOML
example parsing and repository documentation checks can be verified now; new
`.neu` grammar, identity equivalence, authoring round trips and host integration
require the approved implementation and registered tests above.
