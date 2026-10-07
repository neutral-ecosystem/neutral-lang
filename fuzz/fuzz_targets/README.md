<!-- SPDX-License-Identifier: Apache-2.0 -->

# Fuzz targets

Each target drives one untrusted-input or consumer boundary and accepts no host
authority. Target bodies rely on production limits so arbitrary bytes cannot
silently become authoritative partial output.
`composition_support.rs` shares finite successor capture controls and valid-seed
mutation between source, IR and probe targets. It is harness support, not a
production resolver or an additional target.
`transitive_support.rs` extends vocabulary mutation through a locked four-bundle
diamond and the complete successor compiler/reader/probe path. Seed acceptance is
asserted so a silently rejected valid seed cannot masquerade as pipeline coverage.
