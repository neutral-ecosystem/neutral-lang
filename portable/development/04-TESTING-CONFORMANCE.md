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
- independent reader, generic Editor, and Flow-boundary probes; and
- reviewed captured-closure/logical-project/artifact identity vectors.

Flow evidence verifies only that the public reader can supply resolved exported
data to an external consumer. It does not authorize or execute a mapper.

## Gate integrity

The conformance manifest names every active fixture and oracle. Missing,
skipped, flaky, retried, or indeterminate required results fail the stage.
