<!-- SPDX-License-Identifier: Apache-2.0 -->

# CLI system tests

This directory owns black-box tests for the built `neutral-cli` executable.
They exercise real standard streams and isolated filesystem roots rather than
calling command internals.

Within the ecosystem, these tests protect the host boundary: explicit source
and vocabulary acquisition, bounded compilation, safe diagnostic disclosure,
stable exits, and atomic output publication. Artifact inspection remains the
separate `neutral-probe` responsibility.
