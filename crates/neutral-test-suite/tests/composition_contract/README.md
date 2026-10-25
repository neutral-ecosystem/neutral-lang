<!-- SPDX-License-Identifier: Apache-2.0 -->

# Composition contract tests

This directory tests the raw composition and vocabulary catalogue contracts
used by current expression-project capture and independent reading. These
contracts do not create a separate complete `/2` project artifact.

```text
composition_contract/
├── capture.rs              current compiler capture and reader catalogue checks
├── reference.py            retained independent historical identity oracle
├── interface_reference.py  independent public projection oracle
└── fixtures/
    ├── requests/            reviewed capture and catalogue inputs
    ├── identity/            immutable historical transcript vectors
    └── source/              source cases used by catalogue capture tests
```

The historical `/2` identity vectors, Python oracle bytes, and reviewed fixture
inputs remain immutable compatibility evidence. They are not producers or
decoders for current runtime artifacts. The current transport is the complete
expression-project `/3` format, and no decoder fallback accepts historical
project magic.

The executable tests verify literal transcript framing and digests, pinned
capture inputs, registered raw catalogue cases, reader visibility/reference
paths, exact restrictions, permutation determinism, cancellation, and bounded
capture controls. Catalogue-only cases use explicit synthetic source inputs;
they are not source-compilation expectations. Python 3 is required for the
independent frozen oracle and a missing interpreter fails the test.

Run `cargo test -p neutral-test-suite composition_contract` for this module, or
`cargo xtask test all` for complete cross-package and doctest validation.
