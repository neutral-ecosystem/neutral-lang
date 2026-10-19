<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-semantics

Shared pure expression semantics for the compiler and independent reader. It
depends only on core, IR and vocabulary contracts: no parsing, wire decoding,
host lookup, callbacks, scheduling or execution. Semantic-model validity remains
distinct from complete artifact, companion and identity validity.

`validate_expression_model` checks binding types, all residual branches, lexical
match/template frames, nominal restrictions, output selectors and retained resource facts.
It derives obligations without evaluating expressions or acquiring inputs.
Public templates retain exact parameter phases and definition-scoped defaults;
defaults can refer only to earlier parameters. Obligation sites distinguish
binding roots, template bodies and parameter defaults. Validation never invokes
or expands a template, proves application-time staticness, or activates a source,
artifact or evaluator profile. Those remain separate integration boundaries.

`evaluate` checks every supplied binding envelope, then evaluates only selected
paths. Missing, unavailable, failed and null are distinct. Output values must
supply the complete result root so restrictions and selector availability are
checked against real data. Decimal arithmetic uses the frozen decimal contract,
not host floats; work, storage, numerical growth and cancellation are bounded.
`evaluate_checked` uses the same kernel but leaves malformed preflight/envelope
failures as outer errors, while demanded unavailable/failed states remain report
outcomes. Complete-artifact consumer adapters use it to avoid preparing bindings
twice or resetting the shared request budget. `validate_evaluation_bindings` stops
after preflight and never demands a value.

`normalize_binding` reuses that kernel with no external bindings. It permits
producer-private entries without making them public evaluator entries. Fully
static results become values; unconditional static operands also fold inside
residual trees. Unresolved branches stay unevaluated, with eliminated match
payload frames substituted without capturing retained local binders. The result
is owned data, not a published artifact; resource facts must be recomputed.

Run `cargo xtask test all` for complete tests and doctests, or
`cargo test --package neutral-semantics` for this library's tests.
