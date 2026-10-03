// SPDX-License-Identifier: Apache-2.0

//! Regression checks for measured rather than manually asserted release gates.

use super::*;

/// Creates an isolated report directory and checksum-bound coverage fixture.
fn fixture(label: &str) -> (PathBuf, Receipt) {
    let directory =
        std::env::temp_dir().join(format!("neutral-evidence-{label}-{}", std::process::id()));
    fs::create_dir_all(&directory).expect("fixture directory");
    let mut reports = BTreeMap::new();
    for (name, body) in [
        ("tool.stdout", ""),
        ("tool.stderr", ""),
        (
            "coverage.json",
            r#"{"data":[{"totals":{"lines":{"percent":100},"functions":{"percent":100},"regions":{"percent":100}}}]}"#,
        ),
    ] {
        fs::write(directory.join(name), body).expect("fixture report");
        reports.insert(
            name.to_owned(),
            sha256_file(&directory.join(name)).expect("report digest"),
        );
    }
    let receipt = Receipt {
        schema_version: 1,
        gate: "coverage".to_owned(),
        source_commit: "a".repeat(40),
        inputs_sha256: "b".repeat(64),
        policy_sha256: sha256_file(
            &workspace_root()
                .unwrap()
                .join(constants::QUALITY_GATES_FILE),
        )
        .unwrap(),
        toolchain: "test fixture -nightly compiler".to_owned(),
        finished_at_unix_ms: unix_time_millis().unwrap(),
        invocations: vec![Invocation {
            program: "test fixture tool".to_owned(),
            arguments: Vec::new(),
            elapsed_ms: 1,
            report: "tool".to_owned(),
        }],
        reports,
    };
    (directory, receipt)
}

/// Writes a fixture receipt without executing any measurement tools.
fn write_receipt(directory: &Path, receipt: &Receipt) {
    fs::write(
        directory.join("receipt.json"),
        serde_json::to_vec(receipt).unwrap(),
    )
    .unwrap();
}

/// Accepts actual passing metrics and rejects stale provenance and altered bytes.
#[test]
fn rejects_stale_and_modified_evidence() {
    let (directory, mut receipt) = fixture("provenance");
    let root = workspace_root().unwrap();
    write_receipt(&directory, &receipt);
    assert!(verify(&directory, "coverage", &receipt.inputs_sha256, &root).is_ok());
    assert!(verify(&directory, "coverage", &"c".repeat(64), &root).is_err());
    receipt.policy_sha256 = "d".repeat(64);
    write_receipt(&directory, &receipt);
    assert!(verify(&directory, "coverage", &receipt.inputs_sha256, &root).is_err());
    fs::write(directory.join("coverage.json"), "{}").unwrap();
    assert!(validate_reports(&receipt, &directory).is_err());
    fs::remove_dir_all(directory).unwrap();
}

/// Rejects a native acceptance report not protected by a checksum.
#[test]
fn rejects_unhashed_native_metrics_and_low_coverage() {
    let (directory, mut receipt) = fixture("metrics");
    receipt.reports.remove("coverage.json");
    assert!(validate_reports(&receipt, &directory).is_err());
    fs::write(directory.join("coverage.json"), r#"{"data":[{"totals":{"lines":{"percent":0},"functions":{"percent":0},"regions":{"percent":0}}}]}"#).unwrap();
    receipt.reports.insert(
        "coverage.json".to_owned(),
        sha256_file(&directory.join("coverage.json")).unwrap(),
    );
    assert!(validate_reports(&receipt, &directory).is_err());
    receipt
        .reports
        .insert("../outside".to_owned(), "a".repeat(64));
    assert!(validate_reports(&receipt, &directory).is_err());
    fs::remove_dir_all(directory).unwrap();
}

/// Rejects mutation survivors, narrowed selections, and empty or timed-out campaigns.
#[test]
fn validates_real_mutation_counts() {
    let (directory, mut receipt) = fixture("mutants");
    receipt.gate = "mutation".to_owned();
    receipt.invocations[0].arguments = vec![
        "--file".to_owned(),
        quality_value("mutation", "critical_target").unwrap(),
    ];
    for (caught, missed, timeout, accepted) in [
        (38, 0, 0, true),
        (38, 1, 0, false),
        (0, 0, 0, false),
        (38, 0, 1, false),
    ] {
        fs::write(
            directory.join("outcomes.json"),
            serde_json::to_vec(
                &serde_json::json!({"caught":caught,"missed":missed,"timeout":timeout,"success":0}),
            )
            .unwrap(),
        )
        .unwrap();
        receipt.reports.insert(
            "outcomes.json".to_owned(),
            sha256_file(&directory.join("outcomes.json")).unwrap(),
        );
        assert_eq!(validate_reports(&receipt, &directory).is_ok(), accepted);
    }
    receipt.invocations[0].arguments.clear();
    assert!(validate_reports(&receipt, &directory).is_err());
    fs::remove_dir_all(directory).unwrap();
}

/// Ensures newly added source files participate in freshness before a commit.
#[test]
fn input_fingerprint_includes_untracked_source() {
    let directory =
        std::env::temp_dir().join(format!("neutral-input-proof-{}", std::process::id()));
    fs::create_dir_all(&directory).unwrap();
    assert!(
        Command::new(constants::GIT_COMMAND)
            .args(["init", "--quiet"])
            .current_dir(&directory)
            .status()
            .unwrap()
            .success()
    );
    let before = input_digest(&directory).unwrap();
    fs::write(directory.join("new.rs"), "// new source\n").unwrap();
    let after = input_digest(&directory).unwrap();
    assert_ne!(before, after);
    fs::write(directory.join("README.md"), "documentation only\n").unwrap();
    assert_eq!(after, input_digest(&directory).unwrap());
    fs::remove_dir_all(directory).unwrap();
}

/// A missing receipt or unsupported schema never proves gate acceptance.
#[test]
fn rejects_missing_and_unknown_receipts() {
    let (directory, mut receipt) = fixture("schema");
    assert!(read_receipt(&directory).is_err());
    receipt.schema_version += 1;
    write_receipt(&directory, &receipt);
    assert!(
        verify(
            &directory,
            "coverage",
            &receipt.inputs_sha256,
            &workspace_root().unwrap()
        )
        .is_err()
    );
    fs::remove_dir_all(directory).unwrap();
}

/// Replaces one synthetic report and updates its checksum for validator tests.
fn replace_report(directory: &Path, receipt: &mut Receipt, name: &str, contents: &str) {
    fs::write(directory.join(name), contents).unwrap();
    receipt
        .reports
        .insert(name.to_owned(), sha256_file(&directory.join(name)).unwrap());
}

/// Compilation wall time alone cannot stand in for a full fuzzer campaign.
#[test]
fn fuzz_requires_every_target_and_actual_fuzzer_duration() {
    let (directory, mut receipt) = fixture("fuzz-duration");
    receipt.gate = "fuzz".to_owned();
    receipt.invocations.clear();
    let budget = quality_value("fuzz", "minimum_seconds_per_target").unwrap();
    let elapsed = budget.parse::<u128>().unwrap() * 1_000;
    for target in quality_array("fuzz", "targets").unwrap() {
        replace_report(&directory, &mut receipt, &format!("{target}.stdout"), "");
        replace_report(
            &directory,
            &mut receipt,
            &format!("{target}.stderr"),
            &format!("Done 100 runs in {budget} second(s)\n"),
        );
        receipt.invocations.push(Invocation {
            program: "fixture".to_owned(),
            arguments: vec![format!("-max_total_time={budget}")],
            elapsed_ms: elapsed,
            report: target,
        });
    }
    assert!(validate_reports(&receipt, &directory).is_ok());
    let target = receipt.invocations[0].report.clone();
    replace_report(
        &directory,
        &mut receipt,
        &format!("{target}.stderr"),
        "Done 1 runs in 1 second(s)\n",
    );
    assert!(validate_reports(&receipt, &directory).is_err());
    receipt.invocations.pop();
    assert!(validate_reports(&receipt, &directory).is_err());
    fs::remove_dir_all(directory).unwrap();
}

/// Performance labels without heap and successful allocation reports are insufficient.
#[test]
fn performance_requires_real_profiler_and_matching_profile() {
    let (directory, mut receipt) = fixture("profilers");
    receipt.gate = "performance-release".to_owned();
    receipt.invocations[0].report = "benchmark".to_owned();
    receipt.invocations[0].arguments.push("release".to_owned());
    let phases = [
        "compile-end-to-end",
        "reader-validation",
        "artifact-encoding",
        "artifact-decoding",
        "probe-traversal",
        "declaration-growth",
        "concurrent-isolation",
    ];
    let output = phases
        .map(|phase| format!("[info] performance name={phase} iterations=250 elapsed-ns=100\n"))
        .join("");
    replace_report(&directory, &mut receipt, "benchmark.stdout", &output);
    replace_report(&directory, &mut receipt, "benchmark.stderr", "");
    assert!(validate_reports(&receipt, &directory).is_err());
    replace_report(&directory, &mut receipt, "massif.out", "mem_heap_B=100\n");
    replace_report(&directory, &mut receipt, "memcheck.stdout", "");
    replace_report(
        &directory,
        &mut receipt,
        "memcheck.stderr",
        "==1== ERROR SUMMARY: 0 errors from 0 contexts\n",
    );
    receipt.invocations.push(Invocation {
        program: "fixture".to_owned(),
        arguments: vec!["--error-exitcode=1".to_owned()],
        elapsed_ms: 1,
        report: "memcheck".to_owned(),
    });
    assert!(validate_reports(&receipt, &directory).is_ok());
    receipt.gate = "performance-soak".to_owned();
    assert!(validate_reports(&receipt, &directory).is_err());
    receipt.gate = "performance-release".to_owned();
    replace_report(
        &directory,
        &mut receipt,
        "memcheck.stderr",
        "==1== ERROR SUMMARY: 1 errors\n",
    );
    assert!(validate_reports(&receipt, &directory).is_err());
    fs::remove_dir_all(directory).unwrap();
}

/// Fresh zero-vulnerability output must cover every configured dependency lock.
#[test]
fn advisories_require_all_locks_and_fresh_zero_findings() {
    let (directory, mut receipt) = fixture("advisories");
    receipt.gate = "advisories".to_owned();
    receipt.invocations.clear();
    assert!(validate_reports(&receipt, &directory).is_err());
    let policy = read_workspace_text(
        &workspace_root().unwrap(),
        constants::DEPENDENCY_SOURCES_FILE,
    )
    .unwrap();
    let mut locks = vec![constants::CARGO_LOCK_FILE.to_owned()];
    locks.extend(configuration_array_from(&policy, "", "isolated_tool_lockfiles").unwrap());
    for (index, lock) in locks.into_iter().enumerate() {
        let report = format!("lock-{index}");
        replace_report(
            &directory,
            &mut receipt,
            &format!("{report}.stdout"),
            r#"{"vulnerabilities":{"found":false,"count":0}}"#,
        );
        replace_report(&directory, &mut receipt, &format!("{report}.stderr"), "");
        receipt.invocations.push(Invocation {
            program: "fixture".to_owned(),
            arguments: vec!["--file".to_owned(), lock],
            elapsed_ms: 1,
            report,
        });
    }
    assert!(validate_reports(&receipt, &directory).is_ok());
    receipt.finished_at_unix_ms = unix_time_millis().unwrap() + 60_000;
    assert!(validate_reports(&receipt, &directory).is_err());
    receipt.finished_at_unix_ms = unix_time_millis().unwrap();
    replace_report(
        &directory,
        &mut receipt,
        "lock-0.stdout",
        r#"{"vulnerabilities":{"found":true,"count":1}}"#,
    );
    assert!(validate_reports(&receipt, &directory).is_err());
    fs::remove_dir_all(directory).unwrap();
}

/// Failed commands keep diagnostic output without producing passing evidence.
#[test]
fn failed_tool_never_writes_a_receipt() {
    let (directory, receipt) = fixture("failed-command");
    let mut measurement = Measurement {
        directory: directory.clone(),
        receipt,
    };
    assert!(
        measurement
            .run(constants::POSIX_SHELL_COMMAND, &["-c", "exit 1"], "failure")
            .is_err()
    );
    assert!(directory.join("failure.stderr").is_file());
    assert!(!directory.join("receipt.json").exists());
    assert!(measurement.finish().is_err());
    fs::remove_dir_all(directory).unwrap();
}
