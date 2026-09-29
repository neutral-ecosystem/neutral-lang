<!-- SPDX-License-Identifier: Apache-2.0 -->

# Rejected graph requests

These requests capture complete source sets but must not publish a graph when
imports are missing, self-referential, duplicated, ambiguous, or forbidden.
The semantic-cycle case is different: its import graph is valid; project
semantic analysis owns the eventual rejection.
