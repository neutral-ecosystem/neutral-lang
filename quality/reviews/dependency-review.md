<!-- SPDX-License-Identifier: Apache-2.0 -->

# Dependency and executable-build review

Review date: 2026-09-06. Owner: maintainer. Result: historical reviewed snapshot;
current lockfiles require automated advisory scans during release qualification.

Repository-tooling update (2026-10-04): typed TOML parsing, formatting-preserving
edits, Cargo discovery, and test inventory use `toml`, `toml_edit`,
`cargo_metadata`, and `nextest-metadata` in the non-published `xtask` package
only. Their locked dependency graph is outside shipped compiler/reader/probe
closures and remains subject to dependency-source and release advisory checks.
The separately installed nextest executable is a development/CI tool, not a
language dependency. This update does not renew the historical advisory scan
below or approve a new release measurement.

The release-engineering review revalidated locked offline Cargo metadata and the declared
registry/checksum source policy on 2026-09-08. Automation now reuses the same
locked `sha2` version for digest-addressed portable snapshots; this introduces
no new third-party package or version. The root lock remains the release lock,
while `fuzz/Cargo.lock` remains an isolated, non-release cargo-fuzz tool lock.

The historical production graph contained one third-party direct dependency, `sha2`, owned
by `neutral-core`; its small transitive cryptographic utility closure is pinned
by `Cargo.lock`. All other normal edges are workspace contracts and are checked
by `cargo xtask check`. `neutral-bench` and `neutral-test-suite` use
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

## Fallible ownership review (08-10-2026)

`neutral-core` adds locked `trybox` 0.1.2 with default features disabled and
`triomphe` 0.1.16 with only `std` enabled. Source review checks allocation layout,
null handling, initialization and final destruction. Neither selected backend
adds a build script, proc macro, native code or runtime dependency. Both contain
upstream unsafe allocation/reference-counting internals; repository code retains
its unsafe prohibition and exposes only safe wrappers. This is an explicit change
to the historical third-party unsafe surface, not a claim that dependencies are
unsafe-free.

The box wrapper uses only `trybox::or_drop` (including its zero-sized path), never
its allocating error conversions. Shared ownership uses `triomphe::Arc::try_new`,
which checks layout/allocation before initialization; infallible constructors,
raw-pointer APIs and weak ownership are not exposed. Successor APIs migrate to
the core `Shared<T>` wrapper; old-profile public types remain unchanged.
Dependency-boundary allowlists include these two reviewed backends. Locked
advisory scans remain a separate qualification requirement; this source review
does not refresh the historical audit result above or prove every pipeline
allocation is fallible.
