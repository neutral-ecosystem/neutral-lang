<!-- SPDX-License-Identifier: Apache-2.0 -->

# Authoring bridge contract

Status: accepted portable baseline

The authoring bridge is a profile versioned separately from Neutral core. An
Editor requiring generic visual authoring needs a compatible core and authoring
profile; headless consumers need only core unless they request authoring.

Discovery takes exact core profile, semantic vocabulary locks, and separate
authoring metadata and returns a deterministic static descriptor catalogue.
The catalogue declares source-authorable constructs/types/value forms,
properties, ports, constraints, presentation hints, formatting capabilities,
and explicit exclusions. It is data, not executable plugins.

For a captured project, a revision-bound overlay provides local/imported types,
symbols, aliases, actions, comments, mappings, and exact required vocabulary
identities. The editable model is closed and bounded. Connections are the sole
representation for ordinary reuse/reference edges; they do not encode execution
or ordering.

Projection deterministically writes ordinary `.neu` source and mappings. The
compiler compiles that source and is authoritative on mismatch. Formatting and
diagnostic presentation are authoring concerns. The bridge never exposes a
compiler-private AST, grants acquisition, or assigns Flow/CI/CD semantics.
