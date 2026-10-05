<!-- SPDX-License-Identifier: Apache-2.0 -->

# Release quality evidence

This directory contains reviewed, immutable evidence summaries grouped by
release. A release directory records the tools, scope, measurements, archive
identity, and conclusions used for that version without retaining bulky or
machine-specific raw output.

Only compact approval records and reviewed summaries belong in Git. Raw
`v<version>/gates/` snapshots are ignored local state: coverage, profiler, and
tool logs may disclose personal paths or host details. Release qualification
still checks actual reports; a fresh runner must regenerate them or restore a
trusted private snapshot. Do not bypass that check or force-add raw reports.
