<!-- SPDX-License-Identifier: Apache-2.0 -->

# Authoring foundation fixtures

This directory owns literal authoring-foundation inputs, not a production
authoring implementation. [AUTHORING](../../contracts/AUTHORING.md) defines
their profile, model, metadata, ordering, formatting, diagnostics, and controls.
The [oracle](../../../conformance/oracles/authoring/foundation.json) records
expected results independently of any future bridge implementation.

`foundation.json` has a default exact profile tuple and complete controls,
named literal inputs, and 26 cases. Each request resolves `model`, `sources`,
`metadata`, `vocabularies`, and snapshot labels only against this file's input
dictionaries. Labels are fixture-adapter references, not public API members or
filesystem paths. No input is fetched. Request `controls` override named default
controls; omitted `cancelled` means false. No other default is inferred.

For each snapshot the future harness captures/compiles the named sources and
validates exact locks derived from the supplied literal bundle bytes, then obtains
the overlay revision from `open`; it never fabricates identity digests or trusts
the edited model as the compiler result. A null source list is the deliberate
redacted-source negative. `source_aliases` checks descriptor deduplication;
it is not semantic lock acquisition. `compare_metadata` runs discovery against
the other literal metadata input without changing semantic bundle inputs.

`round-trip` repeats open/project/compile/read using actual projected buffers;
`format` repeats formatting when requested. `verify.candidate_sources` deliberately
substitutes a literal candidate into the compile/read comparison to distinguish
compiler rejection from accepted-but-wrong meaning. These are test scenarios,
not bridge hooks permitting an editor to bypass compiler authority.

Exact output buffers and UTF-8 byte mappings are literal oracle values. Core and
vocabulary descriptor summaries, stale source/metadata revisions, invalid model
connections, multiline defaults/omissions, Unicode offsets, exact/one-over
controls, and every foundation diagnostic family are represented. Production
comparison remains pending; registration/pin checks are not behavior passes.
