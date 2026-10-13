<!-- SPDX-License-Identifier: Apache-2.0 -->

# Independent identity expectations

`vectors.json` retains literal framing inputs, transcripts, digests and rejection
vectors. `oracle.json` inventories request-family outcomes and expected input
digests. The suite root compares them with the independent Python oracle and
production pipeline; frozen contents remain unchanged when files move.
