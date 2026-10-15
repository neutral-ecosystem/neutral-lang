<!-- SPDX-License-Identifier: Apache-2.0 -->

# Automation tests

This directory owns tests for workspace automation and policy enforcement.
Nested modules may access private `xtask` helpers without placing test bodies in
production source.

`command_output.rs` exercises the real executable: lifecycle rows, help,
single-error reporting, color opt-outs, and script-compatible release tags and
environment JSON. Tests in `unit/` stay linked to their owning source modules
through path-based test declarations after folder moves.

`coverage_command.rs` exercises coverage reuse, whole-workspace reruns, report
cleanup, and failure recovery through the actual executable. It shares an isolated
workspace fixture with `mutation_command.rs`; controlled external tool scripts
record invocations while xtask performs the real fingerprinting and receipt checks.
