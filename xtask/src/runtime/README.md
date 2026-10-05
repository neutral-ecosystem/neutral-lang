<!-- SPDX-License-Identifier: Apache-2.0 -->

# Runtime

This directory owns shared process execution, filesystem helpers, and structured workflow logs. It supports the developer and CI automation layer, not production language behavior.

`execution.rs` runs processes, `workflow.rs` records aggregate steps, and
`files.rs` supplies shared filesystem helpers. `output.rs` owns the terminal
palette and row style; `progress.rs` owns elapsed-time and budget reporting.
`environment.rs`, `workspace.rs`, and `results.rs` resolve host tools, the
workspace root, and safe generated-output locations.

Human diagnostics go to stderr; machine payloads go to stdout without colors
or lifecycle labels. Reporting never changes quality acceptance. Command routing
remains in [the crate entry point](../lib.rs); production contracts stay separate.

`json.rs` serializes typed payloads as pretty JSON with a final newline.
Workflow summaries use that representation; append-only workflow events stay
compact JSONL (one typed event per line). Serde owns escaping and field encoding.
