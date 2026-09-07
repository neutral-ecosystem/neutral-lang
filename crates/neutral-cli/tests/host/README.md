<!-- SPDX-License-Identifier: Apache-2.0 -->

# CLI host private tests

This directory owns path-based tests for private filesystem, input-limit, and
atomic-publication helpers in the CLI host adapter. It exists inside the CLI
crate test tree so production source retains only the path declaration while
these tests can verify host-boundary behavior directly.
