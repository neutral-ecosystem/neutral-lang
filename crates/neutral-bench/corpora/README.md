<!-- SPDX-License-Identifier: Apache-2.0 -->

# Benchmark corpora

This directory owns fixed benchmark inputs, not normative language contracts.
`composition.neu` and `composition-vocabulary.json` exercise the current
multi-module pipeline, variants, typed references, vocabulary defaults and
restrictions. The harness retains exact vocabulary bytes in its lock and builds
fixed growth cases from source. Compatibility document measurements reference
their released conformance corpus directly. Changes to either corpus invalidate
source-bound measurements; new timings need a controlled-runner review.
