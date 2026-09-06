<!-- SPDX-License-Identifier: Apache-2.0 -->

# Dependency and executable-build review

Review date: 2026-09-06. Owner: maintainer. Result: pass for the current lockfile.

The production graph contains one third-party direct dependency, `sha2`, owned
by `neutral-core`; its small transitive cryptographic utility closure is pinned
by `Cargo.lock`. All other normal edges are workspace contracts and are checked
by `cargo xtask boundary check`. `neutral-bench` and `neutral-test-suite` use
only development edges and cannot enter production package closures.

Repository production/workspace inspection found no `build.rs`, proc-macro
crate, native C/C++/assembly source, dynamically loaded module, or executable
code-generation input. Workspace lint policy forbids Rust `unsafe` code. Text
matches for “unsafe” are policy prose or hostile-test descriptions, not unsafe
blocks. The isolated non-production `fuzz/` package necessarily adds
`libfuzzer-sys`, `cc`, and native libFuzzer build machinery; it is separated by
its own workspace and lockfile and cannot enter a shipped crate closure.

`cargo audit 0.22.2` loaded 1,239 RustSec advisories and reported no vulnerability
for either the 22-dependency production lockfile or the 26-dependency fuzz-tool
lockfile. The audit database was fetched into an isolated temporary Cargo home;
release qualification must repeat both scans with the candidate locks and
retain their machine-readable output.
