<!-- SPDX-License-Identifier: Apache-2.0 -->

# Shared visual assets

This directory owns reusable Neutral visual identity assets and repository
diagrams. Product code must not depend on these files for language behavior.

The three `neutral-logo*.png` files are synchronized from the Neutral ecosystem
[website assets](https://github.com/neutral-ecosystem/website/tree/main/public).
`cargo xtask docs` copies them into the local generated Rustdoc site. The
language graph documents repository structure and may be referenced by project
documentation independently.
