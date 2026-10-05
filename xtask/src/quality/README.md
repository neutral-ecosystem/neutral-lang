<!-- SPDX-License-Identifier: Apache-2.0 -->

# Quality

This directory owns quality evaluation and approval ledger management. It supports the developer and CI automation layer, not production language behavior.

`ledger.rs` evaluates profiles and manages compact approval records.
`quality_evidence.rs` measures, retains, and validates source-bound tool reports.
Raw reports remain ignored local state; only reviewed approval summaries belong
in Git. Neither reporting style nor directory organization lowers a gate.

`gate.rs` supplies the closed `QualityGate` identifiers shared by measurements,
receipts, and validation. Serialized gate names remain compatible. Validation
uses one typed configuration snapshot across all gates; measurement completion
also rejects changes to its source inputs or policy bytes during execution.
