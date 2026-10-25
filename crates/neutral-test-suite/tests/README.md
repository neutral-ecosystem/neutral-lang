<!-- SPDX-License-Identifier: Apache-2.0 -->

# Cross-package test inventory

Only modules registered by `src/lib.rs` execute. Historical records retain their
original paths and bytes so existing approvals, pins and references stay valid.

| Active module | Responsibility |
| --- | --- |
| `source_pipeline/` | File-backed current sources, imports, cache and CLI/probe processes |
| `expression_pipeline/` | Complete wire, literal results, consumers, views, identities and registered expression corpus |
| `composition_contract/` | Raw catalogue contracts and current capture/reader integration; its `pipeline/` record is historical |
| `transport/` | Complete current artifact round trips, deterministic wire and hostile decoding |
| `hardening/` | Current-source/property campaigns and deterministic fuzz smoke |

The `module_graph/`, `project_capture/`, `project_ir/`, `public_semantics/` and
`project_identity/` directories retain historical requests, fixtures or oracles;
their presence does not register an executable suite. Consult the current module
above for active coverage. Source-pipeline fixtures execute only when explicitly
referenced; merely adding a `.neu` file does not add a test.

Use `cargo xtask test all` for full discovery, or `cargo nextest list --package
neutral-test-suite` to inspect the executable inventory without running it.
