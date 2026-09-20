<!-- SPDX-License-Identifier: Apache-2.0 -->

# Fixture inventory

Status: staged release asset

The v1 delta corpus is created fixture-first by the nine-stage plan. It must
contain exact complete project requests, source units, vocabulary locks, reader
artifacts, authoring requests, expected accepted summaries/diagnostics, and
identity vectors. No fixture may depend on a host filesystem path, URL, package
manager, network, or ambient vocabulary resolver.

Families include capture closure, modules/imports/SCCs, visibility/public
closure, cross-module reuse/refs, vocabulary locks/location values, project
IR/views, identities, authoring projection, resource limits, and public probes.
