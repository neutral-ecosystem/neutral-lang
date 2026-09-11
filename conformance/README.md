<!-- SPDX-License-Identifier: Apache-2.0 -->

# Release conformance

This directory owns immutable, executable verification inputs for released
Neutral language versions. Unlike `portable/`, these files remain available
after a planning package is archived or replaced.

Each `releases/<version>/` bundle contains the accepted specifications,
fixtures, vocabulary bundles, expected oracles, and conformance manifest needed
to rebuild and test that release. Production crates and tests may depend on a
released bundle; they must never depend on an archived roadmap checkout.

New-version planning belongs in an optional root `portable/` directory. Its
accepted contracts are promoted into a new immutable release bundle only when
that version completes qualification.
