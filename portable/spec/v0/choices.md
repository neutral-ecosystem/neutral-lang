# Neutral language v0 architectural choices

Status: proposed

These choices answer only what is needed to implement and validate Neutral v0.
They do not pre-design later versions.

## Decision summary

| ID | Decision |
| --- | --- |
| C1 | Prove one complete source → IR → generic probe path. |
| C2 | Use one source unit and one module; export every declaration. |
| C3 | Keep a small closed core plus at most one captured data-only vocabulary. |
| C4 | Use immutable explicit bindings, values, and identity-only references. |
| C5 | Separate acquisition from pure semantic compilation. |
| C6 | Separate logical IR from envelope, provenance, derivation, and encoding. |
| C7 | Define equality modulo graph-local element-ID renaming. |
| C8 | Require an untrusted-input reader with host-supplied vocabulary contracts. |
| C9 | Put deterministic structural limits in public APIs. |
| C10 | Defer every feature not exercised by the v0 boundary proof. |

## C1. Smallest complete journey

### Question

What proves that Neutral is a public language/IR boundary rather than only a
parser experiment?

### Decision

Compile one captured `.neu` source into immutable Neutral IR, read it through
the public API, and let a generic effect-free probe enumerate declarations,
values, references, vocabulary payloads, and provenance. A probe diagnostic must
map back to source.

### Rejected alternatives

- Parser-only success does not prove the IR or consumer boundary.
- A runtime demonstration mixes language validity with external behavior.
- A rich language before a probe creates unvalidated abstractions.

## C2. One source unit and exported surface

### Decision

v0 accepts exactly one source unit with one `module` header. All declarations are
exported. Namespace declarations, `pub`/`private`, imports, and multi-unit merging
are deferred because they add no evidence to the first compiler/IR proof.

This also removes projection/redaction complexity from v0.

## C3. Closed core and one vocabulary

### Decision

Core contains only headers, records, immutable bindings, scalar/list/null values,
ordinary value reuse, and `Ref<T>`. v0 may use zero or one exact captured
vocabulary to prove that namespaced nominal data crosses the compiler boundary.

The bundle uses a closed Neutral-owned schema. It is data-only and cannot execute
code or redefine core rules. `use Fixture` names a requirement; host lock data
selects the exact bundle.

### Why a vocabulary remains in v0

Without one tiny vocabulary fixture, the public IR could accidentally be usable
only for core records. The fixture tests extension framing without introducing
application semantics.

## C4. Immutable values and identity references

### Decision

Bindings are immutable, explicit, and type-first. Ordinary binding names reuse
logical values. `ref(name)` creates `Ref<T>` and links declaration identity.

Value edges participate in static cycle detection. Identity edges do not.
`Ref<T>` implies no containment, dependency, ownership, order, or execution.

Mutation, override, and composition are deferred until a later concrete need is
documented.

## C5. Capture and pure compilation

### Decision

The public architecture exposes:

```text
capture(request) -> CapturedCompilation
compileCaptured(captured) -> CompilationResult
compile(request) -> CompilationResult
```

The host-supplied resolver performs acquisition. `compileCaptured` is
deterministic and performs no external I/O. The convenience `compile` operation
composes the two stages.

This boundary supports offline tests, replay, fuzzing, and safe semantic caching.

## C6. Logical payload and companion artifacts

### Decision

The logical IR payload contains versions, logical identities, declarations,
types, final values, reference edges, and required structural features.

Producer/build data, concrete encoding, integrity evidence, source-map identity,
provenance identity, and derivation identity belong to an artifact envelope or
companion records. They do not change logical equality.

Source maps answer where. Provenance answers why/how. Derivation records captured
inputs and policies. These contracts must not be collapsed.

## C7. Identity and equality

### Decision

- module-symbol identity is logical module + name;
- declaration fingerprint commits to kind, type, and logical definition;
- `ElementId` is an opaque document-local graph label; and
- logical equality is graph alpha-equivalence under a consistent one-to-one
  `ElementId` mapping.

Consumers cannot persist an `ElementId` as durable external identity.

Canonical bytes are deferred until signing or content addressing creates a
concrete need.

## C8. External IR validation

### Decision

An in-process typed compiler result is already validated. Any encoded or
external artifact is untrusted and passes `decodeAndValidate` before typed views
are exposed.

The reader receives exact captured vocabulary contracts and reader capabilities
from its host. It never fetches schemas. Missing contracts, unsupported versions,
unknown required structural features, invalid payloads, and broken references
fail closed.

## C9. Limits and diagnostics

### Decision

Compiler and reader APIs take explicit structural budgets for bytes, nesting,
declarations, lists, references, numeric digits/scale, diagnostics, and IR size.
Checks occur before proportional work.

Wall-clock deadlines and physical memory ceilings are named deployment controls,
not language semantics. Completed compilation is deterministic; cancelled or
timed-out partial diagnostic subsets are not claimed reproducible.

Diagnostics use stable codes and canonical ordering by logical source identity,
byte range, code, and safe parameters.

## C10. Defer unproven surface area

v0 deliberately excludes:

- namespaces, visibility, imports, and multi-unit compilation;
- secrets;
- static/member selection;
- additional collection and algebraic types;
- operators and symbolic expressions;
- functions and control structures;
- mutation, composition, override, templates, and macros;
- executable plugins and external effects;
- IR transformation/migration APIs; and
- application-specific constructs.

A later proposal must start from a demonstrated authoring or consumer need and
define semantics, lowering, invalidity, provenance, resource cost, and
conformance evidence. Absence from v0 is not approval for a later release.

## v0 acceptance evidence

The choices close only when:

1. the source grammar and logical IR are normative;
2. the generic probe uses only the public reader API;
3. source-linked diagnostics work without the private AST;
4. repeated and concurrent captured compilation is deterministic modulo
   `ElementId` renaming;
5. malformed source, bundle, and encoded IR fail safely; and
6. the documented resource boundaries have adversarial tests.
