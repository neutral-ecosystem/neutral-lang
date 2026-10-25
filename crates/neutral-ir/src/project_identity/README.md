<!-- SPDX-License-Identifier: Apache-2.0 -->

# Current identity contracts

This module owns bounded, domain-separated `/3` captured, logical, interface,
derivation and artifact transcripts. Framing performs checked,
cancellation-aware writes. It does not parse source, validate project
semantics, acquire inputs or implement caching. The independent reader remains
responsible for complete artifact validation.

| Module | Responsibility |
| --- | --- |
| [captured.rs](captured.rs) | Shared borrowed exact source/vocabulary capture facts. |
| [expression](expression/) | Frozen `/3` identity layers and shared composition-contract framing. |
| [derived.rs](derived.rs) | Shared exact producer-control and artifact-selection framing helpers. |
| [framing.rs](framing.rs) | Checked NHT framing, canonical ordering and fallible retention. |

Literal expected transcript bytes remain in the independent identity tests;
identity refactors must preserve them exactly. Run `cargo test -p neutral-ir
--test expression_identity` for the complete frozen vectors and boundary cases.
