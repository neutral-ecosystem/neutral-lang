<!-- SPDX-License-Identifier: Apache-2.0 -->

# Complete headless identity and result vectors

`baselines.json` retains the three frozen minimal/static, residual and template
projects' exact tuples, wire bytes and all five identity partitions. No original
reviewed source or vector file is rewritten. `vectors.json` adds six complete
source-backed residual cases: subtraction, multiplication, rounded division,
changed operands, selected alternatives and source-evidence-only changes.

The independent Python implementation uses explicit NHT/CBOR framing and integer
rational arithmetic. It imports no production Neutral code. Its checker is
read-only:

```sh
python3 -B crates/neutral-test-suite/tests/expression_pipeline/fixtures/identity-supplement/verify.py
```

The owning `identity_vectors` tests compare the literal complete frames through
independent decoding, compile the actual `.neu` files, compare all five source
identity partitions and check literal evaluation results from both authorities.
Derivation controls are explicit vector inputs, not measurements or attestations
about the host. Frozen identity variations remain exercised by the owning IR
crate's complete-partition tests.

This supplement covers the named scalar/residual/template vectors. It is not a
universal source interpreter, an authoring implementation or campaign evidence.
`registered/dispositions.json` lists historical cases and separate authoring
vectors that remain excluded/blocked; oracle integrity is not a production pass.
