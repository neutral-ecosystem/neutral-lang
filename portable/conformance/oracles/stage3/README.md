<!-- SPDX-License-Identifier: Apache-2.0 -->

# Stage 3 module graph oracles

These are reviewed expected results for the captured module set after graph
construction. They do not claim executable Stage 3 implementation yet. A
successful graph result lists all modules, sorted import edges, and
dependency-first SCCs. Rejections publish neither a partial graph nor project
IR. The semantic-cycle fixture accepts an import SCC at Stage 3; its separate
semantic rejection is reserved for Stage 4.
