<!-- SPDX-License-Identifier: Apache-2.0 -->

# Fuzz corpora

`cargo-fuzz` creates one generated corpus subdirectory per target during a
campaign. Tracked schema seeds under [`../seeds/`](../seeds/) originate from
the reviewed portable fixtures and are supplied as read-only input directories;
confirmed minimized failures are promoted into tracked deterministic tests or
normative/security fixtures rather than left as unexplained opaque bytes here.
