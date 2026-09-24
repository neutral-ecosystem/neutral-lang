<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 2.3 public capture integration

Status: public integration gate complete

Date: 2026-09-25  
Toolchain: `rustc 1.98.1 (48a229cea 2026-09-01)`

## Public contract

`neutral-compiler` publicly exports the versioned request, shared host builder,
capture outcome, stable capture and host errors, and aggregate resource facts.
The outcome exposes exact immutable source and vocabulary facts plus source and
vocabulary unit/byte totals. It can construct a replay request with a fresh
cancellation token and compare capture meaning without treating processing
limits or host correlation keys as semantic inputs.

The builder accepts only already-acquired logical facts. It coalesces identical
host mappings and rejects a conflicting mapping as `NEU-HOST-001` before core
capture. It has no path, URL, resolver, or host-location field.

## Shared host construction

The CLI `capture-project` command and the Editor-style public integration probe
both construct requests through `CapturedProjectRequestBuilder`. The Editor
probe keeps its tab/workspace location outside the request. The CLI reports
only bounded aggregate resource facts and maps cancellation to its stable exit
class.

Public tests construct the same logical project through Editor- and CLI-style
hosts, verify their meaning is equivalent, and exercise equivalent and
conflicting host mappings. Reversing both source and vocabulary input order
produces an exactly equal canonical capture.

## Validation

The following commands passed:

```text
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo xtask test property
cargo xtask ci pr
```

The relevant independent tests are
`crates/neutral-compiler/tests/project_capture_contract.rs` and the
`system_cli_captures_an_explicit_v1_project_request` CLI system test.
