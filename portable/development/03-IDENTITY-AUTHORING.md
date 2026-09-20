<!-- SPDX-License-Identifier: Apache-2.0 -->

# 03 — Identity and authoring bridge

Status: accepted operational plan

## Stages 7–8

### Stage 7 — `v0.7.0 -> v0.8.0`

Implement and independently review the identity chain: captured closure,
logical project, derivation, and artifact. Canonical logical form uses stable
module/type/module-symbol identities rather than graph-local labels. Host paths,
aliases, source evidence, roots, and capture order do not change logical
project identity. Literal transcript and SHA-256 vectors are release assets.

### Stage 8 — `v0.8.0 -> v0.9.0`

Implement the separately versioned, data-only authoring profile: exact profile
discovery; static core/vocabulary descriptor catalogue; revision-bound project
overlay; closed editable graph model; deterministic source projection and
formatting; and authoring diagnostics. Connections are the only visual form of
ordinary reuse and identity-reference edges. The compiler, not the Editor,
remains the source authority.

## Exit evidence

A generic Editor probe must build its palette from discovery, edit a bounded
project model, project source, compile it, reopen it, and no-op round trip
without a handwritten construct table or a private parser/AST dependency.
