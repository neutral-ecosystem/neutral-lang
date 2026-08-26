# Neutral v0 conformance execution assets

The normative source fixtures live in `spec/v0/fixtures/`. This directory owns
the executable conformance manifest, expected-result metadata, generated
reports, and runner configuration created during implementation.

Do not duplicate fixtures here. `conformance/manifest.toml` becomes the sole
fixture-discovery and oracle index when Stage 2 introduces it.
