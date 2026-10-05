<!-- SPDX-License-Identifier: Apache-2.0 -->

# Release

This directory owns release policy and approval validation. It supports the developer and CI automation layer, not production language behavior.

`plan.rs` parses the release scope, `approval.rs` validates version and approval
requirements, and `release_metadata.rs` serializes artifact summaries and
manifests. Artifact assembly lives in `commands/distribution.rs`. These modules
prepare and validate releases without tagging, pushing, or publishing them.

Release entries, manifests, and build provenance are typed Serde payloads.
Generated JSON is pretty-printed; existing field names, types, and schema versions
remain unchanged. Checksums bind to the actual serialized artifact bytes.
