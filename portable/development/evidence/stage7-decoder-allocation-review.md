<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 7 decoder fuzzing and allocation review

Status: passed and approved on 2026-09-06

## Scope and conclusion

This review covers hostile external bytes from fixed-frame validation through
restricted-CBOR parsing, typed reconstruction, and the final trusted-reader
gate. The decoder is approved for the Stage 7 bounds contract: every
input-controlled allocation is preceded by an immutable hard ceiling, a lower
host ceiling where applicable, remaining-input validation, or a previously
validated captured structural limit.

The review does not claim constant memory. A valid artifact may consume memory
proportional to its accepted bounded size. Stage 9 remains responsible for
allocator profiling, performance baselines, long campaigns, and limit tuning.

## Reviewed allocation boundaries

| Boundary | Guard before growth | Evidence |
| --- | --- | --- |
| Complete frame | Artifact size is checked before header or section parsing | `decode_frame` and hostile artifact-limit tests |
| Section ranges | Fixed directory arithmetic is checked; each nonzero length is bounded and contained in input | frame truncation, length, overlap, and ordering tests |
| Byte/text strings | Declared length is converted safely, clamped to decoder limits, and checked against remaining bytes before copying | `Parser::byte_string` and arbitrary-byte campaign |
| Arrays/maps | Item count and remaining bytes are checked before iteration; untrusted lengths are never passed to `reserve` | `Parser::container_length`, duplicate-map tests, mutation campaign |
| Recursive CBOR/logical values | Nesting and total traversal-node budgets are charged before descent | depth/node limit tests and mutation campaign |
| Logical strings/lists/records | Captured acceptance limits are applied before the second-stage owned model is built | string, list, record, and captured-limit tests |
| Exact numbers | Digit and scale limits plus canonical spelling are checked while borrowed; the coefficient is copied only after validation | `ExactNumber::from_normalized_parts` and numeric decoder tests |
| Public reader view | Logical, source-map, provenance, derivation, fingerprint, vocabulary, and edge validation completes before publication | Stage 7 decoder integration/security suites |

No decoder path reserves capacity directly from an untrusted container length.
Checked arithmetic protects frame offsets and ranges. Decoder cancellation is
tested independently and is polled during recursive parsing and reconstruction.

## Fuzz campaign evidence

The stable Stage 7 campaign is reproducible and has three executable targets:

- every proper truncation of a valid encoded artifact;
- 2,048 seeded structured multi-byte mutations; and
- 1,024 seeded arbitrary byte sequences up to 4,096 bytes.

Run it with:

```console
cargo xtask fuzz campaign
```

The campaign passed on 2026-09-06 with no panic, hang, partial reader view, or
unclassified allocation failure. Single-byte mutation smoke and deterministic
hostile regression cases remain part of the PR-equivalent suite. Coverage-guided
and long-running campaigns remain Stage 9 work and do not weaken this Stage 7
bounded-decoder approval.

## Review decision

- [x] Frame and section lengths are checked before section decoding.
- [x] String, byte-string, container, depth, and traversal growth is bounded.
- [x] Captured structural limits constrain typed reconstruction.
- [x] Invalid exact-number coefficients are rejected before their retained copy.
- [x] Stable decoder fuzz targets pass and retain a reproducible seed.
- [x] No public reader view is returned from partially validated data.
