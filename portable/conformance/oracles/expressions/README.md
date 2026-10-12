<!-- SPDX-License-Identifier: Apache-2.0 -->

# Expression review oracles

This directory owns literal expected result projections and their independent
byte/arithmetic review checker. It must not import the production compiler,
evaluator or encoding implementation. It performs no operation execution.

[results.json](results.json) pairs every candidate request with a complete
value projection, demanded bindings, unavailable state or stable failure code.
[verify.py](verify.py) checks those IDs and the arithmetic/framing/CBOR/matrix
vectors in the [fixture corpus](../../../specs/fixtures/expressions/README.md).
The checker uses standard-library integer rationals and explicit NHT/CBOR framing;
it does not bless expected output by invoking Neutral or rewriting fixtures.
[project_vectors.py](project_vectors.py) reconstructs complete source-only
identity partitions and checks the registered envelope bytes and variations.
[authoring_vectors.py](authoring_vectors.py) independently projects the named
minimal editable-model subset; it rejects constructs outside that subset.
Neither helper is an untrusted-input validator or a production service.

The result projections are not a substitute for the eventual complete public
result/failure envelope. This review-only suite cannot claim parsing, folding,
decoder, production authoring, allocation-fault or compiler-independent evaluator coverage.
Written approval and byte pins are recorded in the freeze manifest. Implement
production and independent interpreter comparisons without regenerating expected
results; embedded candidate labels identify the original reviewed snapshots.
