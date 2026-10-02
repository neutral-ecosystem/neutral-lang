<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 5 vocabulary and inert-location fixtures

Each TOML file is an exact captured-project request. `bundle_utf8` contains
already-acquired bundle bytes and its lock pins their SHA-256 digest. Positive
cases cover multiple independent aliases and distinct inert scalar values;
negative cases cover source-inaccessible private types and executable members.
The Stage 2 missing, extra, and conflicting-lock fixtures are reused by this
gate instead of copying their immutable bytes.
