<!-- SPDX-License-Identifier: Apache-2.0 -->

# Development container

This directory owns the reproducible supported container definition. It pins
the base image and stable toolchain, mounts generated Cargo/results storage,
and delegates the normal stable prerequisite check to `cargo xtask bootstrap`.
The container intentionally does not install the optional nightly, fuzz,
mutation, or profiling tool set required by the separate complete workstation
audit. It is a developer adapter, not a release artifact or language input.
