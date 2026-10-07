<!-- SPDX-License-Identifier: Apache-2.0 -->

# Explicit successor source integration

This directory contains request-local parsing, type resolution and lowering for
the composition capture profile. It reuses the private lexer and inherited type
grammar, then feeds source and vocabulary contracts through one shared semantic
validator. Frozen project compilation remains on its separate entry point.

`composition_attribution` traces materialized occurrence paths back to original
source/default/reuse tokens or canonical vocabulary default owners. Missing
evidence stays absent, never synthesized; the independent reader rechecks it.
`composition_cache` optionally reuses exact private parsed units under independent
retention bounds. All graph, catalogue/default, semantic and companion work stays
fresh; failed runs never publish a pending generation.

`composition_graph` shares the graph syntax scanner but retains successor import
topology fallibly, without constructing old-profile public `Arc` projections.
Its iterative SCC passes enforce component size and traversal work before
expansion. Resolver indexes, recursive copies and companion assembly use checked
fallible retention; resource or cancellation failures publish no artifact.

Complete producer IR must pass the independent reader before encoding. Nothing
here acquires imports, opens inert locations, chooses public roots or executes
operations.
