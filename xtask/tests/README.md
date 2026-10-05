<!-- SPDX-License-Identifier: Apache-2.0 -->

# Automation tests

This directory owns tests for workspace automation and policy enforcement.
Nested modules may access private `xtask` helpers without placing test bodies in
production source.

`command_output.rs` exercises the real executable: lifecycle rows, help,
single-error reporting, color opt-outs, and script-compatible release tags and
environment JSON. Tests in `unit/` stay linked to their owning source modules
through path-based test declarations after folder moves.
