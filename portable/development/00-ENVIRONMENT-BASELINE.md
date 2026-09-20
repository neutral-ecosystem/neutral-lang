<!-- SPDX-License-Identifier: Apache-2.0 -->

# 00 — Environment and v0 baseline

Status: accepted operational plan

## Purpose

Prepare the existing Rust implementation for the v1 release train without
forking or reimplementing v0. The baseline is the working `v0.1.0` behavior,
not a new source language.

## Required controls

- Pin and record the Rust toolchain, formatter, linter, dependency policy, CI
  image, and conformance command line used for each release gate.
- Keep the v0.1 corpus executable under explicit `neu "0.1"` selection at every
  v0.x and v1 candidate release.
- Make profile dispatch explicit before accepting `neu "1.0"`; a compiler
  release number must never reinterpret an existing v0 source file.
- Keep capture/semantic compilation pure after the host supplies bytes. Tests
  must prove that no filesystem, environment, clock, network, or resolver is
  consulted on that path.
- Retain machine-readable test, vector, dependency-audit, and benchmark
  evidence for each `.4` stage gate.

## Package boundaries

The Rust workspace may evolve, but must preserve separate ownership for core
types/limits, IR and encoding, vocabulary validation, compilation, reader,
authoring services, CLI host integration, probes, and test support. A split is
not required for its own sake; the public reader and authoring APIs must remain
free of compiler-private parser and semantic-model types.

## Exit evidence

Stage 1 cannot close until clean CI proves the v0 corpus, the profile-selection
matrix, deterministic diagnostics, and an empty v1 feature dispatch shell.
