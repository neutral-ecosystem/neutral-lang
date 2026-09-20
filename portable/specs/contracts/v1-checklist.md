<!-- SPDX-License-Identifier: Apache-2.0 -->

# Neutral v1 implementation checklist

Status: active release-train checklist

## How to use this checklist

Every item is intentionally unchecked until it has accepted contract prose,
exact fixtures/oracles where applicable, implementation evidence, and the
required public-boundary test. A checked item is not merely designed or demoed.
The v0.1 profile remains covered by its own frozen corpus and is not repeated
here.

## Stage 1 — profile and foundation (`v0.1.0 -> v0.2.0`)

- [ ] V1-BASE-001 — Explicit v0.1 and v1.0 profile dispatch.
- [ ] V1-BASE-002 — v0.1 corpus runs unchanged on every release candidate.
- [ ] V1-BASE-003 — v1 diagnostics/limits have stable public families.
- [ ] V1-BASE-004 — No-I/O capture/compile dependency audit.
- [ ] V1-BASE-005 — v1 exclusions reject functions, effects, acquisition, and
  product semantics.

## Stage 2 — captured project (`v0.2.0 -> v0.3.0`)

- [ ] V1-CAP-001 — Versioned complete `CapturedProjectRequest`.
- [ ] V1-CAP-002 — Unique logical module/source identities and header matching.
- [ ] V1-CAP-003 — Complete supplied closure, including disconnected modules.
- [ ] V1-CAP-004 — Exact lock coverage and conflicting host-mapping rejection.
- [ ] V1-CAP-005 — Independent bounds for bytes, modules, imports, SCCs,
  declarations, diagnostics, and output.
- [ ] V1-CAP-006 — Capture freezes immutable inputs without resolver/I/O.

## Stage 3 — modules (`v0.3.0 -> v0.4.0`)

- [ ] V1-MOD-001 — Exact `neu "1.0"` and qualified module header syntax.
- [ ] V1-MOD-002 — One source unit per logical module.
- [ ] V1-MOD-003 — Mandatory aliased logical imports and shared alias namespace.
- [ ] V1-MOD-004 — Deterministic graph construction and diagnostic ordering.
- [ ] V1-MOD-005 — SCC collection permits import cycles.
- [ ] V1-MOD-006 — Missing/self/duplicate/relative/wildcard/re-export imports
  fail as specified.

## Stage 4 — public project semantics (`v0.4.0 -> v0.5.0`)

- [ ] V1-VIS-001 — Private-by-default root declarations and explicit `public`.
- [ ] V1-VIS-002 — Imported access only through qualified public names.
- [ ] V1-VIS-003 — Public signatures close over public reachable types.
- [ ] V1-XMOD-001 — Cross-module immutable value reuse and provenance.
- [ ] V1-XMOD-002 — Cross-module `Ref<T>` records stable module-symbol identity.
- [ ] V1-XMOD-003 — Publicly exposed refs cannot target private bindings.
- [ ] V1-XMOD-004 — Semantic-cycle rules remain separate from import SCCs.

## Stage 5 — vocabulary and locations (`v0.5.0 -> v0.6.0`)

- [ ] V1-VOC-001 — Multiple aliased exact semantic vocabulary locks.
- [ ] V1-VOC-002 — One canonical vocabulary identity has one project revision.
- [ ] V1-VOC-003 — Only source-authorable/public vocabulary types can surface.
- [ ] V1-LOC-001 — `url` and `path` are typed inert data values.
- [ ] V1-LOC-002 — Location values cannot resolve/import/fetch/open/authorize.
- [ ] V1-VOC-004 — Vocabulary remains closed and data-only.

## Stage 6 — IR, reader, and views (`v0.6.0 -> v0.7.0`)

- [ ] V1-IR-001 — Complete project-level IR and module/export index.
- [ ] V1-IR-002 — Private validation content, source maps, and provenance.
- [ ] V1-IR-003 — Validated public reader without source/private AST access.
- [ ] V1-VIEW-001 — Roots are post-compilation view inputs only.
- [ ] V1-VIEW-002 — Views include required public interpretive dependencies.
- [ ] V1-API-001 — Capture, compile, validate-reader, and derive-view outcomes
  are bounded, versioned, and deterministic.

## Stage 7 — identity (`v0.7.0 -> v0.8.0`)

- [ ] V1-ID-001 — Captured-closure identity is distinct from logical identity.
- [ ] V1-ID-002 — Canonical logical form ignores host paths, aliases, capture
  order, source evidence, roots, and derivation options.
- [ ] V1-ID-003 — Logical project identity uses a versioned canonical transcript
  and SHA-256 digest.
- [ ] V1-ID-004 — Derivation and artifact identities include only their stated
  additional context.
- [ ] V1-ID-005 — Independent implementations agree on literal identity vectors.

## Stage 8 — authoring (`v0.8.0 -> v0.9.0`)

- [ ] V1-AUTH-001 — Exact core/authoring profile compatibility discovery.
- [ ] V1-AUTH-002 — Deterministic core/vocabulary descriptor catalogue.
- [ ] V1-AUTH-003 — Revision-bound project descriptor overlay.
- [ ] V1-AUTH-004 — Closed bounded editable model and one edge representation.
- [ ] V1-AUTH-005 — Deterministic projection, mappings, formatting, and no-op
  round trip.
- [ ] V1-AUTH-006 — Generic Editor probe has no compiler-private dependency.

## Stage 9 — release (`v0.9.0 -> v1.0.0`)

- [ ] V1-CONF-001 — Every v1 requirement has fixture/oracle and traceability.
- [ ] V1-CONF-002 — All named core and authoring diagnostics have negative cases.
- [ ] V1-CONF-003 — Reader, generic Editor, and Flow-boundary probes pass.
- [ ] V1-CONF-004 — Limits, hostile input, deterministic, concurrency, and
  clean/incremental-equivalence evidence pass.
- [ ] V1-REL-001 — Release notes publish profiles, limits, exclusions, and
  migration behavior.
- [ ] V1-REL-002 — Retained evidence approves `v0.9.4 -> v1.0.0`.
