<!-- SPDX-License-Identifier: Apache-2.0 -->

# Captured vocabulary bundle fixtures

These exact-byte JSON fixtures exercise the Stage 6 captured vocabulary trust
boundary. `positive/` contains accepted logical contracts; `negative/` contains
hostile structural, semantic, and executable-looking inputs.

JSON does not permit comments, so the `.json` fixture bytes cannot carry an
inline SPDX comment without ceasing to be strict JSON. This README records their
Apache-2.0 licensing and the fixture oracles record their exact byte digests.
