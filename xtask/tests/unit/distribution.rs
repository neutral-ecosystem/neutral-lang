// SPDX-License-Identifier: Apache-2.0

//! Typed manifests preserve release semantics without hand-escaped JSON fragments.

use super::*;

/// Selected channels, artifact hashes, and hostile text serialize without changing field shapes.
#[test]
fn release_manifest_serializes_typed_entries_and_empty_arrays() {
    let mut plan = release_plan().unwrap();
    let context = ReleaseEntryContext {
        version: "test-revision",
        candidate_commit: "quote\"newline\n",
        license: "license\ttext",
    };
    let mut entries = Vec::new();
    let mut checksums = Vec::new();
    append_release_entry(
        &mut entries,
        &mut checksums,
        "unicode-é\".bin",
        b"exact bytes",
        "custom\\channel",
        &context,
    );
    for selected in [false, true] {
        if selected {
            plan.channels.insert(release::DistributionChannel::CratesIo);
        } else {
            plan.channels
                .retain(|channel| *channel != release::DistributionChannel::CratesIo);
        }
        for artifacts in [&entries[..], &[][..]] {
            let bytes = release_manifest_bytes(&plan, &context, "host\nname", artifacts).unwrap();
            assert!(bytes.ends_with(b"\n"));
            let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(value["schema_version"], 1);
            assert_eq!(value["candidate_commit"], context.candidate_commit);
            assert_eq!(
                value["supported_targets"],
                serde_json::json!(["host\nname"])
            );
            assert_eq!(value["license"], context.license);
            assert_eq!(value["crates_io_selected"], selected);
            assert_eq!(
                value["deferred"].as_array().unwrap().len(),
                if selected { 1 } else { 2 }
            );
            assert_eq!(
                value["artifacts"].as_array().unwrap().len(),
                artifacts.len()
            );
            if !artifacts.is_empty() {
                assert_eq!(value["artifacts"][0]["filename"], "unicode-é\".bin");
                assert_eq!(value["artifacts"][0]["sha256"], sha256_hex(b"exact bytes"));
                assert_eq!(value["artifacts"][0]["channel"], "custom\\channel");
            }
        }
    }
    assert_eq!(checksums.len(), 1);
    assert!(checksums[0].starts_with(&sha256_hex(b"exact bytes")));
}

/// Typed build provenance preserves existing fields and JSON-safe compiler identification.
#[test]
fn release_provenance_is_typed_and_pretty() {
    let plan = release_plan().unwrap();
    let bytes = release_provenance_bytes(&plan, "commit\"quoted", "target\\host", b"lock").unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert!(bytes.starts_with(b"{\n  "));
    assert_eq!(value["candidate_commit"], "commit\"quoted");
    assert_eq!(value["target"], "target\\host");
    assert_eq!(value["release_tag"], plan.release_tag);
    assert_eq!(value["cargo_lock_sha256"], sha256_hex(b"lock"));
    assert_eq!(value["builder"], value["reproducible_command"]);
}
