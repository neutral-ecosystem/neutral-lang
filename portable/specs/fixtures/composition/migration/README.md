<!-- SPDX-License-Identifier: Apache-2.0 -->

# Migration composition inputs

These literal migration cases belong to the standalone vocabulary composition
catalogue test boundary. They are not proof of compiler/project/codec activation.
See the parent fixture README for ownership and the pinned conformance manifest.

`requests.json` retains a schema `1.0` leaf under an explicit `2.0` adapter and
rejects a new-shape bundle relabelled as old schema. Old data is never modified or
silently upgraded, and no package version selects a schema.
