<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 4 public integration and validation

Status: implementation gates `v0.4.3` and `v0.4.4` complete; `v0.5.0` release promotion remains separate.

The compiler resolves the complete captured declaration graph, then projects a
separate public-only interface: canonical module-symbol exports, type and field
signatures, and public-to-public typed dependencies. No captured bytes, source
IDs, byte spans, private root names, private edge endpoints, or raw provenance
are members of the interface. `neutral-reader` validates canonical order,
public nominal closure, exact signature-derived type edges, structural edge
categories, type-depth bounds, and the domain-separated fingerprint before
publishing indexed export lookup. `neutral-probe` enumerates cross-module value
reuse and identity references using only this reader view, without compiler
linkage. This is not the complete Stage 6 project IR or an encoded project
artifact.
The reader validates this snapshot's internal consistency, not its producer's
claim that every export was public in original source; independent proof of
that claim belongs to the complete Stage 6 project-IR boundary.

The reviewed Stage 4 fixture oracle pins the positive public export list and
fingerprint. The cross-package tests verify redaction of private implementation
data, independently rejected forged interfaces, public field order, reference
type edges, private-value fingerprint invariance, public-signature fingerprint
sensitivity, alias/capture-order invariance, and concurrent reader determinism.
Compiler tests cover visibility, transitive public closure, nominal identity,
reuse/ref typing, an accepted import SCC without a semantic cycle, a rejected
cross-SCC value cycle, and canonical multi-error ordering. Reader tests also
reject missing or mislabeled signature-derived type edges. Test-owned fixture
and oracle copies match their portable counterparts byte-for-byte, allowing
the suite to survive portable archival.

## Compatibility review

| Inherited v0 rule | Stage 4 extension and checked boundary |
| --- | --- |
| The `0.1` single-unit language remains available | Stage 4 resolution accepts only complete captured `1.0` projects; it does not enable the standalone `1.0` compiler path or alter the inherited v0 suite. |
| Explicit immutable bindings and exact scalar/container types | Cross-module reuse requires the same resolved type. Nominals use full module-symbol identity, so equal record shapes from different modules remain incompatible. |
| Explicit record fields and defaults | Public signatures include canonical field names/types, not private defaults or contextual values; Stage 6 will validate and lower full values. |
| Typed `Ref<T>` is identity-only | Publicly reachable private targets fail; reference-type edges do not create embedded-value cycles. Imported targets must be public. |
| No ambient source or host acquisition | Names cross modules only through explicit captured import aliases; the reader and probe consume an immutable interface, not paths, URLs, or source acquisition. |
| Bounded, deterministic failure | Semantic resolution returns no model on errors/cancellation/limits; diagnostics and export/edge order are canonical, and the reader rejects malformed public snapshots before traversal. |

Verification completed on 2026-09-29:

```text
cargo test --workspace --quiet
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
cargo xtask fixtures check
cargo xtask portable verify
cargo xtask ci pr
git diff --check
```

The composed CI profile passed with its workflow log at ignored
`test-results/workflows/ci/pr/run-2-13`. The fixture command verified 40 fixtures and
40 oracles with no remaining manifest or contract-freeze drift. Promotion
still requires the separate
checklist/manifest/traceability review and release-quality workflow.
