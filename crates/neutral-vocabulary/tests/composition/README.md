<!-- SPDX-License-Identifier: Apache-2.0 -->

# Composition integration inputs

This directory owns captured schema-selection, semantic-default, and dependency
inputs for the standalone vocabulary boundary. Tests in `../composition.rs`
exercise the shared IR contracts without compiler linkage. These are not proof
of source-to-artifact integration or release readiness.
`copy.rs` is a path-based private test module for fallible occurrence copies. It
checks each reservation failure and exact Unicode/path preservation; injected
errors are not evidence of a real out-of-memory campaign.

`decode.rs` interrupts each request-local schema retention checkpoint with a
synthetic allocation error or cancellation, checks that no raw bundle is returned,
and verifies a fresh unchanged request succeeds. Independent outer-count and
wrapper-depth tests reject before proportional retention.
