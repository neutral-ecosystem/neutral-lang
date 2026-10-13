<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-semantics

Shared pure expression semantics for the compiler and independent reader. It
depends only on core, IR and vocabulary contracts: no parsing, wire decoding,
host lookup, callbacks, scheduling or execution. Semantic-model validity remains
distinct from complete artifact, companion and identity validity.

`validate_expression_model` checks binding types, all residual branches, lexical
match frames, nominal restrictions, output selectors and retained resource facts.
It derives obligations without evaluating expressions or acquiring inputs.
Public templates are rejected as unsupported until their validation is complete;
this boundary does not activate the successor source or artifact profile.

Run `cargo xtask test all` for complete tests and doctests, or
`cargo test --package neutral-semantics` for this library's tests.
