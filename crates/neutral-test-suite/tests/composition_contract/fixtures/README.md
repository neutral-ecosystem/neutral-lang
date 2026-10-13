<!-- SPDX-License-Identifier: Apache-2.0 -->

# Composition fixture data

`requests/` contains literal captured request families, `identity/` contains their
independent expected transcripts/digests and outcome inventory, and `source/`
contains readable source inputs for pipeline regressions. Rust tests load these
through the suite root so individual test modules do not repeat directory paths.
Relocation does not regenerate or reformat frozen bytes. Tests run without the
active portable plan; these are runtime-owned inputs, not generated reports.
