<!-- SPDX-License-Identifier: Apache-2.0 -->

# 02 — Semantic project core

Status: accepted operational plan

## Stages 4–6

### Stage 4 — `v0.4.0 -> v0.5.0`

Implement private-by-default declarations, explicit public declarations,
qualified imported access, public signature closure, cross-module immutable
reuse, and public `Ref<T>` target rules. Test declaration-level value cycles
separately from allowed import SCCs.

### Stage 5 — `v0.5.0 -> v0.6.0`

Implement multiple explicitly aliased, exact data-only vocabulary locks. Check
the exact lock cover across all modules and reject missing, unused, duplicate,
or conflicting locks. Add `url` and `path` as distinct inert scalar values:
they preserve text but never acquire/open/normalize anything.

### Stage 6 — `v0.6.0 -> v0.7.0`

Lower only valid complete projects to project-level IR with modules, export
index, private validation content, source maps, provenance, resource facts,
and derivation facts. Implement public reader validation and consumer-selected
views. Roots are input only to a view request and cannot alter project meaning.

## Exit evidence

An independent reader-only probe must enumerate a project and its selected
public view without parsing source or accessing compiler-private models.
