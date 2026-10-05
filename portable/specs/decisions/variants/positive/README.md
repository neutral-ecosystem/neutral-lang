<!-- SPDX-License-Identifier: Apache-2.0 -->

# Planned valid variants

These source inputs illustrate the shared variant model for future compiler,
IR, reader, encoding, and probe tests. They are not currently accepted fixtures.

| Input | Intended result |
| --- | --- |
| `source-declaration.neu` | Local public variant, two selected alternatives, list, and typed reference |
| `imported-provider.neu`, `imported-consumer.neu` | Captured two-unit project with qualified access to an imported public variant |
| `vocabulary-type-entry.json`, `vocabulary-use.neu` | Vocabulary-owned variant through a source-local alias |

The last pair needs a complete new-schema bundle and exact captured lock before
it can execute. The imported pair needs a complete captured project envelope.
Expected IR, public closure, provenance, and identity oracles remain to be frozen.
