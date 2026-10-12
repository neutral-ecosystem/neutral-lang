<!-- SPDX-License-Identifier: Apache-2.0 -->

# Project-neutral consumer fixture convention

Profile: `neutral.example-consumer/1`.
Status: registered literal fixture convention, not a universal vocabulary or an
implemented consumer. Inputs and expected reports are pinned independently from
the [Consumer/1 boundary oracles](CONSUMER-PROFILE.md). No execution is involved.

## Captured example vocabulary

The self-contained `ExampleConsumer` schema-2.0 bundle defines public Resource,
Entry, Output, Deferred, Input, Equals, Condition, Operation and Graph types.
Exact version/digest locks and explicit capture /2 selection apply. No network
registry, mapping callback or executable vocabulary member exists. Source aliases
have no semantics. Resource URLs are inert, including `example.invalid` URLs.

An Operation has a closed kind (`emit`, `transform`, `join`), typed string
arguments as key/value Entry lists, typed Deferred inputs, explicit `needs` refs,
Condition and resource refs. Missing collection fields default to empty lists;
missing Condition defaults to literal true. Every kind's result contract in this
example is a record with exactly one string member `value`; this is consumer
convention, not compiler magic. A Graph contains explicit operation and entry refs.
Graph entries must be in its operation set; names/refs are exact stable symbols.

Consumer rules are deliberately separate from core validation:

- Duplicate argument keys and input names reject; list representation alone is
  core-valid. Map keys are exact case-sensitive UTF-8 strings, not normalized.
- Explicit prerequisite cycles reject. Output connections do not synthesize
  prerequisite edges; their member/result-type and availability checks are separate.
- Output member lookup permits `value` only. Validate all structural connection
  contracts, including disabled operations, before demanding any output value.
- Conditions are literal Boolean or exact string equality against a named output.
  Evaluate a condition before its operation inputs; disabled operations do not
  demand their inputs. Demanded output dependency cycles reject even when values
  are supplied; ordinary ref cycles remain core-valid data. Skipped operations
  are not executed and never implicitly produce a value.
- Inputs are literal strings, named external strings, named operation outputs,
  or symbolic `(scope,name)` handles. The supplied planning snapshot states
  availability only. It never resolves credentials or proves authorization.
- Capability checking rejects unsupported kinds. An offered alternative produces
  `alternative-required`, not success. Only explicit approval, a new exact adapter
  context and full revalidation can accept the alternative; do not silently rewrite
  operation meaning or mutate project identity.

Canonical traversal uses stable operation symbols. A reported cycle rotates to
its lexicographically smallest symbol; choose the lexicographically first cycle.
Topological ties use stable symbols. A diagnostic may identify the original
offending edge separately from the canonical cycle tuple. Conditions and deferred
values here are data conventions; this is not activation of expression/evaluator
syntax. Future language expressions use the shared pure evaluator instead.

## Request/report fixture boundary

`cases.json` carries capture/lock facts, file pins, complete default request facts
and 31 named cases. Each case replaces named top-level defaults; `controls` merge
individual positive counters; `bindings` merge named output/input/handle sections.
Relative files resolve within the fixture directory, never through acquisition.
`request_file` replaces the entire request and is parsed strictly, before defaults:
unknown members and duplicate keys reject. Defaults are harness policy, not a
rule that makes missing/invalid user requests acceptable.

The closed example request has `schema,entry,target,bindings,controls,alternative`;
schema is `neutral.example-consumer-request/1`. Target has adapter/target revisions,
supported kinds and exact alternative descriptions. Bindings have output values
indexed by public operation name, external string inputs and symbolic handle
availability. Fixture `cancelled` and `input_revision` are outer harness context,
not extra request members. Unknown/duplicate keys and invalid binding shapes fail.

Literal example reports have `schema,phase,status,entry,context,diagnostics`;
schema is `neutral.example-consumer-report/1`, phase is `consumer`, status is
accepted/rejected/alternative-required, and context has exact adapter/target/input
revisions. Diagnostics have `phase,code,subject,path,primary,related`. Subjects
and member paths use Consumer/1 stable symbols/segments; locations are checked
original-byte occurrences or explicitly unavailable. A core rejection has its
literal NEU-COMP code and no consumer report. Never mislabel it a consumer cycle.

These are literal report projections, not fabricated complete Consumer/1 artifact
receipts. The future Consumer/1 harness must additionally bind the envelope to
the independently validated project logical identity and exact report contract;
that identity is not replaced with a source digest or invented placeholder hash.
`facts` are separately asserted typed inspection outcomes, not extra report fields.
Report/private-view invariants prohibit private provenance, host paths, backend
execution, credential resolution, partial acceptance and context-driven identity
changes. Public materialized text reused from a private binding remains visible;
the private origin name/span/digest does not. Public use-site attribution remains.

## Diagnostics and bounds

| Code | Consumer meaning |
| --- | --- |
| EXC-001 | Prerequisite or demanded-output cycle |
| EXC-002 | Demanded external/output/handle value unavailable |
| EXC-003 | Unknown output member/result-contract mismatch |
| EXC-004 | Unsupported capability; explicit alternative may be offered |
| EXC-005 | Duplicate map key or input name |
| EXC-006 | Malformed/unknown/duplicate request member |
| EXC-007 | Invalid/exceeded resource control or allocation failure |
| EXC-009 | Alternative not explicitly approved |
| EXC-010 | Missing/nonpublic/wrong-type entry |
| EXC-011 | Cooperative cancellation |

Codes are owned by this example consumer, not NEU-CONS compiler/reader codes.
Check strict request shape, controls and pre-cancellation; independently validate
the core artifact; then entry/closure, counters/map uniqueness/member contracts,
prerequisite and demanded-output cycles, capabilities, condition/input availability.
Later cancellation/allocation failure terminates atomically at its checkpoint.
Sort ordinary diagnostics by stable subject, typed path, code and safe byte range.
Core-invalid inputs never reach this consumer phase. No backend command is run.

Positive independent hard ceilings: 1,000,000 operations/prerequisites/output
connections/map entries each, 1,000,000 entries/diagnostics, 100,000,000 work visits,
67,108,864 report bytes. Intersect Consumer/1 reader/capture limits. Counter scope
is the complete selected Graph closure, not only enabled operations. Preflight
retention/edge counts before proportional allocation. Exact/one-over literals
cover operation, prerequisite and output counts; further exhaustive controls and
fault tests belong to the unchecked implementation/validation gates.

## Completion

Registration establishes reviewed input bytes and literal expected consumer
reports, not a consumer implementation or full-pipeline passing evidence.
The current explicit composition compiler and independent reader can validate
the core-positive source files and reject the four core-negative families.
The future consumer adapter must compare unchanged reports, verify all safe
locations and invariants, and exercise hostile/boundary/fault cases before
required-active status. No separate product repository is needed.
