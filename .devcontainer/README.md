<!-- SPDX-License-Identifier: Apache-2.0 -->

# Development container

This directory owns the supported container definition. Rebuilding it selects
the latest stable Rust toolchain and nextest runner, mounts generated Cargo/results storage,
and delegates the normal stable prerequisite check to `cargo xtask bootstrap`.
The container intentionally does not install the optional nightly, fuzz,
mutation, or profiling tool set required by the separate complete workstation
audit. It is a developer adapter, not a release artifact or language input.

Initial creation fetches locked Cargo dependencies before bootstrap, so an empty
cache works with network access. Later offline work is optional after the cache
is populated. Rebuild the container to pick up stable toolchain updates.
