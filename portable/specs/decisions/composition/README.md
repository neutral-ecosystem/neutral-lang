<!-- SPDX-License-Identifier: Apache-2.0 -->

# Composition schema design inputs

This directory owns complete-envelope examples for the proposed
[vocabulary composition contract](../../contracts/VOCABULARY-COMPOSITION.md).
It helps the compiler, vocabulary, IR, and reader owners agree on one model.
It is not an active conformance suite or a second progress tracker.

`bundle.json` demonstrates a defaulted restricted number, optional nullable
text, a list of closed variants, and a typed reference. It has no external
dependency and is not accepted by the existing project `1.0` validator.
The separate composition catalogue API validates an independent runtime copy
in the vocabulary crate's integration tests. This does not activate project
compilation. Full semantic oracles, new identity/wire vectors, complete capture
requests, allocation review and manifest pinning are required before project
activation.
