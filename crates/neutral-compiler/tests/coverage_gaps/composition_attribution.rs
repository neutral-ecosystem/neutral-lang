// SPDX-License-Identifier: Apache-2.0

//! Missing source-occurrence coverage for reuse of materialized vocabulary defaults.

/// A reused vocabulary default keeps its original default owner without relabelling copied data.
#[test]
#[ignore = "UNIMPLEMENTED: copied vocabulary defaults must not acquire new default attribution"]
fn reused_vocabulary_default_does_not_relabel_supplied_origin() {
    // TODO: Compile a vocabulary-defaulted record and a source binding reusing it; assert the original Defaulted path names the vocabulary owner while the copied non-Defaulted path has no attribution, exercising composition_attribution.rs:27-28.
    todo!("UNIMPLEMENTED");
}
