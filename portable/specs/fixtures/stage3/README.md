<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 3 module graph fixtures

Each TOML file is a complete `neutral.capture-fixture/1` request using the
frozen Stage 2 input schema. `source_utf8` contains exact source bytes. Graph
expectations live in the matching Stage 3 oracles. These fixtures are reviewed
inputs for Stage 3 implementation; registration does not count as an executable
pass. The semantic-cycle case also reserves the Stage 4 semantic outcome while
requiring Stage 3 to accept its import SCC.

The small explicit controls make exact and one-over import-edge, per-module
import, and SCC-size boundaries reviewable. No source path is an import target.
