<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 2 captured-project fixtures

These TOML fixtures describe exact host-supplied request values before a Rust
API exists. `neutral.capture-fixture/1` maps directly to the closed fields in
the captured-project request contract. `source_utf8` is the exact source byte
sequence; escape decoding follows TOML basic-string rules. `bundle_hex` is the
exact captured vocabulary byte sequence in lowercase hexadecimal.

Host-mapping fixtures use `neutral.host-mapping-fixture/1`. Their
`host_location` values are inert test labels and are never copied into a
request or used for filesystem access. Oracles own expected outcomes; fixtures
contain inputs only.

Every request fixture supplies the complete processing-control table. Small
limits are intentional and make exact/one-over cases reviewable.
