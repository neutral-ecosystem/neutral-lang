<!-- SPDX-License-Identifier: Apache-2.0 -->

# 04 — Testing and conformance

Status: accepted operational plan

## Fixture-first rule

Before a stage's implementation is accepted, add exact request/source/
vocabulary/authoring inputs, expected accepted summaries or diagnostics, and
identity vectors where identity is claimed. Every core and authoring diagnostic
family has at least one named negative case.

## Required evidence

- inherited v0 corpus under `neu "0.1"`;
- v1 positive/negative source and multi-file project corpus;
- capture and vocabulary lock failures;
- canonical ordering, shuffled-input, concurrent, repeated, and clean versus
  incremental equivalence tests;
- resource ceilings, cancellation, malformed/hostile input, fuzz, and decoder
  tests;
- independent reader, generic authoring, and project-neutral consumer-boundary probes; and
- reviewed captured-closure/logical-project/artifact identity vectors.

Consumer-boundary evidence verifies that the public reader supplies resolved
typed data and safe attribution to an external consumer. It does not authorize
or execute a mapper. The scheduled
[consumer-readiness requirements](../specs/contracts/CONSUMER-READINESS.md)
require composite vocabulary/default/restriction cases before `v0.8.0` and
operation/resource/condition/output/capability/report and authoring cases before
`v0.9.0`. Freeze literal outcomes and schema versions before activation; keep
core rejection and consumer interpretation rejection as separate phases.

The approved Stage 8 expression/composition extension (V1-CONS-011..016)
requires literal operator/type/phase oracles, short-circuit and unselected-branch
errors, exhaustive matching, exact arithmetic/division and conversion boundaries,
typed parameter scope, and finite matrix ordering/expansion limits. Compare
production static evaluation with an independent test interpreter; verify
deferred missing/skipped outputs at the consumer boundary, not as compiler
execution failures. Include reader-only combined examples, required-feature
rejection, public-view redaction, cancellation/allocation faults, semantic identity
changes, and actual clean/cache equivalence. Stage 9 reruns the complete corpus
and independent interpretation against final inputs; Stage 8 demos do not replace
release qualification. No tests acquire credentials or perform backend effects.

Test each Stage 8 slice before its dependent slice begins. Independent pure
interpretation must not delegate operator evaluation or canonicalization to the
production evaluator; compare both with literal expected values/errors and
transcripts. Cover deferred guards with selected/unselected unavailable outputs
and invalid arithmetic, always-on branch type checking, and retained restriction
checks after binding. Structural references, guarded value dependencies, and
explicit prerequisites have separate expected reports. Compare static expressions
with literals and template applications with explicit expansions for logical
equivalence while captured identities/source evidence differ. Authoring preserves
template source structure; semantic readers preserve expanded meaning.

Stage 9 admits only feature-complete inputs. Missing semantic implementation or
unresolved semantics return to Stage 8. Qualification fixes invalidate dependent
evidence: rerun final integration and the measurements whose source-bound inputs
changed, rather than treating earlier passing receipts as evidence for new inputs.

## Gate integrity

The conformance manifest names every active fixture and oracle. Missing,
skipped, flaky, retried, or indeterminate required results fail the stage.
