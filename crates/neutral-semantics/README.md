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

Run `cargo xtask test all` for complete tests and doctests, or
`cargo test --package neutral-semantics` for this library's tests.
