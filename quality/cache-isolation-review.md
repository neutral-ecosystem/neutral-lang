<!-- SPDX-License-Identifier: Apache-2.0 -->

# Cache, request-isolation, and source-fact review

Review date: 2026-09-06. Owner: maintainer. Result: pass for current production
paths.

Production crates contain no mutable global cache, cross-request interner,
ambient resolver, registry client, or hidden filesystem/network lookup.
Requests own exact source bytes, typed source digests, limits, cancellation, and
optional exact vocabulary bytes plus lock facts. Successful artifacts are
immutable and carry their own source map and derivation partitions.

The Stage 9 isolation test alternates distinct module names, values, bytes, and
digests and verifies each artifact reports only the digest of its request.
Repeated and eight-way concurrent adversarial runs compare exact outcomes.
Vocabulary locks validate the digest of the captured bundle before parsing, and
external decoding reconstructs and validates all relationships before exposing
a reader.

CLI temporary output is unique and committed atomically only after success.
Generated rustdoc caching is presentation-only and keyed by a content-derived
header token; it cannot affect compilation meaning.
