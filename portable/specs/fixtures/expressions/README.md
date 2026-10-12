<!-- SPDX-License-Identifier: Apache-2.0 -->

# Expressions and finite expansion — frozen corpus

This directory owns file-backed review inputs for V1-CONS-011..016, not an
implemented language extension. [EXPRESSION-PROFILE](../../contracts/EXPRESSION-PROFILE.md)
freezes capture/IR/wire/identity /3 and authoring /2. Existing profiles never
select these forms by source spelling, package version or fallback.

- `positive/`: arithmetic, guards, matching, construction, input/output values,
  restrictions, definition-scoped templates and finite matrices.
- `negative/`: syntax/type/scope errors, selected arithmetic failure, invalid
  templates, deferred shape, collisions, restrictions and imperative exclusions.
- `boundary/`: caller controls exactly at and one below required numeric, byte,
  depth, node and Cartesian-product consumption (one-over consumption rejects).
- [bundle.json](bundle.json): host-supplied closed schema 2.0 restriction/default
  contracts; it contains no executable expressions or acquisition instructions.
- [cases.json](cases.json): exact files/digests, explicit successor selection,
  per-case bindings/controls and review operations. Numeric result strings denote
  decimal `num`, not string-valued bindings. Omitted bindings differ from null.
- [vectors.json](vectors.json): literal independent numeric results, complete
  minimal captured/logical/interface/derivation/artifact NHT vectors, a complete
  minimal static binary envelope, scoped residual-node fragments and matrices.
- [complete-vectors.json](complete-vectors.json): complete guarded-residual and
  public-template envelopes, all five identity transcripts, source companions,
  expansion attribution and ten controlled identity variations.
- [authoring-vectors.json](authoring-vectors.json): three literal editable-model
  projections with byte mappings and six negative model cases. The independent
  projector covers only these named constructs, not the full authoring service.

The [result projections](../../../conformance/oracles/expressions/results.json)
are literal review expectations, not production observations or complete result
envelopes. A supplied malformed binding fails preflight even on a skipped branch;
a valid unavailable/failed binding is demanded only on a selected branch.
Core errors do not impersonate consumer scheduling/availability policy failures.

Run the independent integrity check from the repository root:

```bash
python3 portable/conformance/oracles/expressions/verify.py
```

It checks file pins, case/oracle correspondence, rational division results,
explicit frame bytes/digests, restricted-CBOR encoding and matrix enumeration.
It also reconstructs all five complete identity partitions, checks source
occurrences, and compares minimal authoring projections and UTF-8 mappings.
It does not parse Neutral source, validate complete IR, simulate resource/fault
behavior, implement the shared evaluator, or compare against production.

The exact profile tuple, lexical binding representation, transport layout,
identity framing, source amendment and literal vectors were approved on 12-10-2026.
The suite is `frozen`, not `required`. Embedded candidate status strings and
candidate-named IDs are retained as part of the reviewed immutable snapshots;
[freeze.toml](../../contracts/freeze.toml) and the manifest govern acceptance.
Full production comparisons and hostile/fault/boundary tests
remain activation requirements; no current compiler acceptance is implied by a
positive filename. Historical fixture bytes remain untouched.
