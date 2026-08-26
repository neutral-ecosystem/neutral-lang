# Section 1: governing boundaries

## SYN-GOV-001 — Representation boundaries

The public direction is:

```text
.neu -> private compiler models -> Neutral IR -> consumer-private model
```

Concrete source and public IR are separately versioned contracts. Tokens,
parser recovery, syntax trees, resolution tables, and lowering models are
private. A consumer uses only the public reader API and does not parse source.

## SYN-GOV-002 — Lowering and provenance

Every accepted construct has an abstract meaning, deterministic IR lowering or
explicit non-semantic source-map treatment, and an origin containing captured
source identity plus a half-open byte span.

Comments and formatting are trivia. Value reuse and defaults affect provenance
but do not become distinct logical value kinds. The compiler emits no
authoritative IR after any semantic error.

## SYN-GOV-003 — Acceptance is not authority

Parsing, validation, and IR production perform no external operation and grant
no authority. Source syntax cannot request ambient filesystem, environment,
network, command, credential, or runtime access. A vocabulary is data, never
executable compiler code.

## SYN-GOV-004 — Completion evidence

A syntax decision is complete only with:

- normative lexical/grammar and semantic prose;
- valid, invalid, boundary, and misleading-lookalike examples;
- stable diagnostic codes and source spans;
- deterministic lowering and provenance;
- resource-limit treatment;
- formatter behavior where applicable; and
- source-to-IR plus public-reader conformance fixtures.

The generic probe must enumerate accepted output without compiler-private AST
access.
