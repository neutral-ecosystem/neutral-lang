<!-- SPDX-License-Identifier: Apache-2.0 -->

# neutral-test-support

`neutral-test-support` contains reusable test-only builders, fixtures, and
assertions shared by Neutral test owners.

It is non-published infrastructure and never belongs in a production dependency
graph. Unit tests remain with the package under test, while cross-package tests
belong to `neutral-test-suite`; this package only removes duplication between
those test locations.

## Command

This is test-only support infrastructure. Verify its shared helpers with:

```sh
cargo test --package neutral-test-support
```
