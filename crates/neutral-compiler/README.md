<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-compiler

`neutral-compiler` owns the captured-input compilation pipeline: private
frontend processing, semantic analysis, and lowering to public logical IR.

It depends on core, IR, and vocabulary contracts only. The CLI supplies any
host-facing capture work before this crate runs; `neutral-compiler` must remain
deterministic and free of filesystem, environment, network, command, locale,
and clock access. Its parser and semantic internals are intentionally private.
For code orientation, follow [capture](src/project_capture.rs),
[graph construction](src/module_graph.rs),
[semantic resolution](src/frontend/project_semantics.rs), and
[contextual lowering](src/frontend/project_lowering.rs). Their function comments
explain phase ordering, validation responsibilities, and publication boundaries;
the [cache](src/frontend/project_cache.rs) documents the narrow reuse seam.

The public boundary captures exact bytes immutably. The v1 project-capture
boundary accepts only a closed, versioned, data-only request with explicit
independent limits. It retains the complete supplied source set, validates
request/header agreement and exact vocabulary-lock coverage, and exposes no
resolver, callback, path, URL, root, or ambient acquisition hook.
The public `CapturedProjectRequestBuilder` is the shared host adapter boundary:
CLI, Editor-style hosts, and test harnesses supply the same logical source and
vocabulary facts while retaining locations outside core. Successful capture
exposes exact immutable sources, vocabularies, aggregate resource facts,
meaning equivalence, and replay through a newly supplied cancellation token.
`CapturedProject::identity_transcript` projects verified exact source/vocabulary
facts into the shared bounded captured-closure identity contract without
recapture or I/O. `identity_capture_limits` exposes the explicit control order
used for derivation identity; it never adds these controls to logical meaning.
The pure `CapturedProject::module_graph` API scans those already-captured bytes
for logical module imports, validates the complete supplied closure, and
returns a deterministic, bounded SCC graph. Modules, edges, and diagnostics
retain exact source IDs and typed original-byte locations. Graph construction
cannot acquire source units or resolve paths and URLs.
The project analyzer resolves public/private roots and cross-module
type, value, and reference edges over that graph. It can produce a narrow
public-only interface snapshot with a signature fingerprint. The separate
`compile_project` boundary validates all contextual values and closed defaults
before publishing complete `neutral-ir::project::ProjectIr`, including private
and disconnected modules, materialized reuse, identity-only references, exact
source/vocabulary companions, and resource facts. Failures never publish partial
IR. This library API does not activate the standalone v1 compiler profile.

The separate [composition capture](src/project_capture/composition.rs) API,
`capture_composition_project`, requires an explicit successor envelope and exact
features. It validates complete transitive locks, schemas, public type closure,
defaults and restrictions, then shares immutable contracts with independent
readers. Repeated local aliases refer to one canonical lock; dependencies never
become implicit source aliases. Replay preserves exact bytes and both policies.
Its separately typed captured identity matches the frozen successor transcript.
This capture-only output cannot enter `compile_project`: source variants,
complete successor IR, logical identities and binary transport are not yet active.

Within one clean project compilation, semantic analysis hands its private parsed
units to lowering instead of parsing them again. This request-local handoff does
not expose syntax internals or retain data between requests; the explicit cache
API remains a separate, caller-owned optimization.
For captured projects, it validates every supplied vocabulary bundle as
one exact canonical set, resolves module-local `use` aliases to locked public
types, and retains direct `url`/`path` scalar bindings as distinct inert values.
The independently validated reader derives public views only after complete
compilation; selected roots never prune input or change complete logical meaning.
`neutral-encoding` owns external project transport; the standalone probe
validates it through reader-only dependencies and reports the typed complete
logical identity independently of selected roots.

`ProjectCompilationCache` is an optional caller-owned, bounded in-process syntax
cache. Its explicit retention budgets limit units and original source bytes;
statistics distinguish actual parser execution from reused units. Keys check
module, language profile, exact source digest, and exact bytes, not public or
logical fingerprints. Every run reconstructs the graph and revalidates vocabulary,
semantics, values, source maps, provenance, resource facts, and current controls.
Failures do not publish a new cache generation. There is no persistent/importable
cache, global state, host acquisition, or final-artifact memoization.
Its private frontend recognizes the supported source, identifier, comment, exact-number,
bounded-string, Boolean, nullable-scalar, null, nominal-record, and
contextual-record behavior.
It collects the complete root scope before nominal type resolution and rejects
embedded record cycles. It accepts one exact host-captured vocabulary bundle and
lock, validates it before source payloads, and resolves optional `use` plus
qualified nominal types without performing acquisition. Exact trivia stays
private for the reference formatter and is never lowered into authoritative
logical IR.

The reference formatter accepts only captured source that completes the compiler
validation path. It emits canonical header order, LF newlines,
four-space record/list indentation, one field or item per line with a trailing
comma, normalized delimiter spacing, and no semicolons. Comments retain their
text and source order with LF-normalized line endings, and are deterministically
lifted to standalone top-level positions before the root construct that
contained or followed them; comments
after the final construct remain at end of file. Formatter output is ordinary
source bytes with a new source digest when recaptured—not logical IR, canonical
artifact encoding, or signing material.

## Command

Use `cargo xtask test all` for complete repository validation, including doctests.
The focused Cargo command below runs this crate's test binaries only.

This is a library crate. Verify its compilation pipeline with:

```sh
cargo test --package neutral-compiler
```
