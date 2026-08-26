# SPDX-License-Identifier: Apache-2.0

FROM rust:1.97.1-bookworm@sha256:14bc9c5966e7b3a385794b3d5389a8765668342025fbcc7b2e3d2866ac4bd8c3

RUN rustup component add rustfmt clippy \
    && useradd --create-home --shell /bin/bash neutral

USER neutral
WORKDIR /workspaces/neutral-lang

# The base image already contains this exact pinned toolchain. Keeping Rustup on
# that installed target prevents an unnecessary registry refresh during
# network-denied test execution.
ENV RUSTUP_TOOLCHAIN=1.97.1-x86_64-unknown-linux-gnu
