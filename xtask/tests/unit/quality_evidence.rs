// SPDX-License-Identifier: Apache-2.0

//! Regression checks for measured rather than manually asserted release gates.

use super::*;

/// Creates an isolated report directory and checksum-bound coverage fixture.
fn fixture(label: &str) -> (PathBuf, Receipt, QualitySettings) {
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
        schema_version: 2,
        gate: QualityGate::Coverage,
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
            tool_version: "test fixture tool 1".to_owned(),
            arguments: Vec::new(),
            elapsed_ms: 1,
            report: "tool".to_owned(),
        }],
        reports,
    };
    (
        directory,
        receipt,
        QualitySettings::load(&workspace_root().unwrap()).unwrap(),
    )
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
    let (directory, mut receipt, settings) = fixture("provenance");
    write_receipt(&directory, &receipt);
    assert!(
        verify(
            &directory,
            QualityGate::Coverage,
            &receipt.inputs_sha256,
            &settings
        )
        .is_ok()
    );
    assert!(
        verify(
            &directory,
            QualityGate::Coverage,
            &"c".repeat(64),
            &settings
        )
        .is_err()
    );
    receipt.policy_sha256 = "d".repeat(64);
    write_receipt(&directory, &receipt);
    assert!(
        verify(
            &directory,
            QualityGate::Coverage,
            &receipt.inputs_sha256,
            &settings
        )
        .is_err()
    );
    fs::write(directory.join("coverage.json"), "{}").unwrap();
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
    fs::remove_dir_all(directory).unwrap();
}

/// Rejects a native acceptance report not protected by a checksum.
#[test]
fn rejects_unhashed_native_metrics_and_low_coverage() {
    let (directory, mut receipt, settings) = fixture("metrics");
    receipt.reports.remove("coverage.json");
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
    fs::write(directory.join("coverage.json"), r#"{"data":[{"totals":{"lines":{"percent":0},"functions":{"percent":0},"regions":{"percent":0}}}]}"#).unwrap();
    receipt.reports.insert(
        "coverage.json".to_owned(),
        sha256_file(&directory.join("coverage.json")).unwrap(),
    );
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
    receipt
        .reports
        .insert("../outside".to_owned(), "a".repeat(64));
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
    fs::remove_dir_all(directory).unwrap();
}

/// Rejects mutation survivors, narrowed selections, and empty or timed-out campaigns.
#[test]
fn validates_real_mutation_counts() {
    let (directory, mut receipt, settings) = fixture("mutants");
    receipt.gate = QualityGate::Mutation;
    receipt.invocations[0].arguments = settings
        .policy
        .mutation
        .critical_targets
        .iter()
        .flat_map(|target| ["--file".to_owned(), target.clone()])
        .collect();
    receipt.invocations[0]
        .arguments
        .push("--no-config".to_owned());
    receipt.invocations[0]
        .arguments
        .push("--cargo-arg=--workspace".to_owned());
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
        assert_eq!(
            validate_reports(&receipt, &directory, &settings).is_ok(),
            accepted
        );
    }
    replace_report(
        &directory,
        &mut receipt,
        "outcomes.json",
        r#"{"caught":38,"missed":0,"timeout":0,"success":0}"#,
    );
    let complete = receipt.invocations[0].arguments.clone();
    receipt.invocations[0].arguments.drain(..2);
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
    receipt.invocations[0].arguments.clone_from(&complete);
    receipt.invocations[0]
        .arguments
        .push("--re=one_function".to_owned());
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
    receipt.invocations[0].arguments.clone_from(&complete);
    receipt.invocations[0]
        .arguments
        .retain(|argument| argument != "--cargo-arg=--workspace");
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
    receipt.invocations[0].arguments.clone_from(&complete);
    receipt.invocations[0]
        .arguments
        .retain(|argument| argument != "--no-config");
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
    receipt.invocations[0].arguments.clear();
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
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

/// Executable inputs invalidate receipts regardless of extension or directory.
#[test]
fn input_fingerprint_tracks_oracles_scripts_templates_and_deletions() {
    let directory =
        crate::unique_generated_directory(&std::env::temp_dir().join("neutral-inputs-all"))
            .unwrap();
    fs::create_dir_all(directory.join("docs")).unwrap();
    assert!(
        Command::new(constants::GIT_COMMAND)
            .args(["init", "--quiet"])
            .current_dir(&directory)
            .status()
            .unwrap()
            .success()
    );
    fs::write(directory.join(".gitignore"), "reports/\n").unwrap();
    let mut previous = input_digest(&directory).unwrap();
    for name in [
        "oracle.py",
        "release.sh",
        "release.ps1",
        "docs/template.html",
        "tool-without-extension",
        "policy.yaml",
        "language-fixture.md",
    ] {
        let path = directory.join(name);
        fs::write(&path, "first").unwrap();
        let added = input_digest(&directory).unwrap();
        assert_ne!(previous, added, "addition: {name}");
        fs::write(&path, "second").unwrap();
        let changed = input_digest(&directory).unwrap();
        assert_ne!(added, changed, "edit: {name}");
        fs::remove_file(path).unwrap();
        assert_eq!(
            previous,
            input_digest(&directory).unwrap(),
            "deletion: {name}"
        );
        previous = input_digest(&directory).unwrap();
    }
    fs::write(directory.join("oracle.py"), "oracle").unwrap();
    assert!(
        Command::new(constants::GIT_COMMAND)
            .args(["add", "oracle.py"])
            .current_dir(&directory)
            .status()
            .unwrap()
            .success()
    );
    let tracked = input_digest(&directory).unwrap();
    fs::rename(directory.join("oracle.py"), directory.join("renamed.py")).unwrap();
    assert_ne!(tracked, input_digest(&directory).unwrap());
    fs::create_dir_all(directory.join("reports")).unwrap();
    fs::write(directory.join("reports/local.json"), "local paths").unwrap();
    fs::create_dir_all(directory.join("quality/evidence/v1.2.3")).unwrap();
    let before_approval = input_digest(&directory).unwrap();
    fs::write(
        directory.join("quality/evidence/v1.2.3/record.toml"),
        "approval",
    )
    .unwrap();
    assert_eq!(before_approval, input_digest(&directory).unwrap());
    fs::remove_dir_all(directory).unwrap();
}

/// A missing receipt or unsupported schema never proves gate acceptance.
#[test]
fn rejects_missing_and_unknown_receipts() {
    let (directory, mut receipt, settings) = fixture("schema");
    assert!(read_receipt(&directory).is_err());
    receipt.schema_version += 1;
    write_receipt(&directory, &receipt);
    assert!(
        verify(
            &directory,
            QualityGate::Coverage,
            &receipt.inputs_sha256,
            &settings
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
    let (directory, mut receipt, settings) = fixture("fuzz-duration");
    receipt.gate = QualityGate::Fuzz;
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
            tool_version: "fixture 1".to_owned(),
            arguments: vec![format!("-max_total_time={budget}")],
            elapsed_ms: elapsed,
            report: target,
        });
    }
    assert!(validate_reports(&receipt, &directory, &settings).is_ok());
    let target = receipt.invocations[0].report.clone();
    replace_report(
        &directory,
        &mut receipt,
        &format!("{target}.stderr"),
        "Done 1 runs in 1 second(s)\n",
    );
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
    receipt.invocations.pop();
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
    fs::remove_dir_all(directory).unwrap();
}

/// Performance labels without heap and successful allocation reports are insufficient.
#[test]
fn performance_requires_real_profiler_and_matching_profile() {
    let (directory, mut receipt, settings) = fixture("profilers");
    receipt.gate = QualityGate::PerformanceRelease;
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
        .map(|phase| format!("PASS  performance name={phase} iterations=250 elapsed-ns=100\n"))
        .join("");
    replace_report(&directory, &mut receipt, "benchmark.stdout", &output);
    replace_report(&directory, &mut receipt, "benchmark.stderr", "");
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
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
        tool_version: "fixture 1".to_owned(),
        arguments: vec!["--error-exitcode=1".to_owned()],
        elapsed_ms: 1,
        report: "memcheck".to_owned(),
    });
    assert!(
        validate_reports(&receipt, &directory, &settings).is_err(),
        "old-only benchmark must not qualify the composition pipeline"
    );
    let mut expanded = String::new();
    for phase in &settings.policy.performance.required_phases {
        use std::fmt::Write as _;
        writeln!(
            expanded,
            "PASS  performance name={phase} iterations=250 elapsed-ns=100"
        )
        .unwrap();
    }
    replace_report(&directory, &mut receipt, "benchmark.stdout", &expanded);
    assert!(validate_reports(&receipt, &directory, &settings).is_ok());
    receipt.gate = QualityGate::PerformanceSoak;
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
    receipt.gate = QualityGate::PerformanceRelease;
    replace_report(
        &directory,
        &mut receipt,
        "memcheck.stderr",
        "==1== ERROR SUMMARY: 1 errors\n",
    );
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
    fs::remove_dir_all(directory).unwrap();
}

/// Fresh zero-vulnerability output must cover every configured dependency lock.
#[test]
fn advisories_require_all_locks_and_fresh_zero_findings() {
    let (directory, mut receipt, settings) = fixture("advisories");
    receipt.gate = QualityGate::Advisories;
    receipt.invocations.clear();
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
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
            tool_version: "fixture 1".to_owned(),
            arguments: vec!["--file".to_owned(), lock],
            elapsed_ms: 1,
            report,
        });
    }
    assert!(validate_reports(&receipt, &directory, &settings).is_ok());
    receipt.finished_at_unix_ms = unix_time_millis().unwrap() + 60_000;
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
    receipt.finished_at_unix_ms = unix_time_millis().unwrap();
    replace_report(
        &directory,
        &mut receipt,
        "lock-0.stdout",
        r#"{"vulnerabilities":{"found":true,"count":1}}"#,
    );
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
    fs::remove_dir_all(directory).unwrap();
}

/// Failed commands keep diagnostic output without producing passing evidence.
#[test]
fn failed_tool_never_writes_a_receipt() {
    let (directory, receipt, settings) = fixture("failed-command");
    let mut measurement = Measurement {
        _store: MeasurementStore::begin(&directory.join("ownership")).unwrap(),
        directory: directory.clone(),
        receipt,
        settings,
    };
    assert!(
        measurement
            .run(
                constants::GIT_COMMAND,
                &["not-a-neutral-command"],
                "failure"
            )
            .is_err()
    );
    assert!(directory.join("failure.stderr").is_file());
    assert!(!directory.join("receipt.json").exists());
    assert!(measurement.finish().is_err());
    fs::remove_dir_all(directory).unwrap();
}

/// Validation consumes the supplied command snapshot, not another on-disk threshold lookup.
#[test]
fn validates_against_supplied_policy_snapshot() {
    let (directory, mut receipt, mut settings) = fixture("snapshot");
    replace_report(
        &directory,
        &mut receipt,
        "coverage.json",
        r#"{"data":[{"totals":{"lines":{"percent":87},"functions":{"percent":100},"regions":{"percent":100}}}]}"#,
    );
    assert!(validate_reports(&receipt, &directory, &settings).is_ok());
    settings.policy.coverage.minimum_line_percent = 90.0;
    assert!(validate_reports(&receipt, &directory, &settings).is_err());
    fs::remove_dir_all(directory).unwrap();
}

/// Unknown gate names fail at receipt decoding rather than falling through string dispatch.
#[test]
fn unknown_receipt_gate_is_rejected() {
    let (directory, receipt, _) = fixture("unknown-gate");
    let mut json = serde_json::to_value(&receipt).unwrap();
    json["gate"] = serde_json::json!("unknown");
    fs::write(
        directory.join(reports::RECEIPT),
        serde_json::to_vec(&json).unwrap(),
    )
    .unwrap();
    assert!(read_receipt(&directory).is_err());
    fs::remove_dir_all(directory).unwrap();
}

/// A second measurement cannot invalidate a live measurement's output.
#[test]
fn competing_measurements_do_not_share_writable_reports() {
    let first = Measurement::begin(QualityGate::Advisories).unwrap();
    let marker = first.directory.join("owner.txt");
    fs::write(&marker, "first owner").unwrap();
    assert!(Measurement::begin(QualityGate::Advisories).is_err());
    assert_eq!(fs::read_to_string(marker).unwrap(), "first owner");
    let first_directory = first.directory.clone();
    drop(first);
    let next = Measurement::begin(QualityGate::Advisories).unwrap();
    assert_ne!(first_directory, next.directory);
}

/// Completed runs resolve through a safe pointer, while failed reruns cannot reuse retained success.
#[test]
fn measurement_pointer_requires_complete_latest_run() {
    let (directory, receipt, settings) = fixture("pointer");
    let root = directory.join("store");
    let run = root.join("run-1-0");
    fs::create_dir_all(&run).unwrap();
    for name in receipt.reports.keys() {
        fs::copy(directory.join(name), run.join(name)).unwrap();
    }
    write_receipt(&run, &receipt);
    fs::write(root.join("current"), "run-1-0").unwrap();
    assert!(
        verify(
            &root,
            QualityGate::Coverage,
            &receipt.inputs_sha256,
            &settings
        )
        .is_ok()
    );
    for unsafe_pointer in ["../outside", "/outside", "run-1-0/nested", ""] {
        fs::write(root.join("current"), unsafe_pointer).unwrap();
        assert!(
            verify(
                &root,
                QualityGate::Coverage,
                &receipt.inputs_sha256,
                &settings
            )
            .is_err()
        );
    }
    let generated = directory.join("generated");
    let retained = directory.join("retained");
    fs::create_dir_all(generated.join("coverage")).unwrap();
    fs::create_dir_all(retained.join("coverage")).unwrap();
    for name in receipt.reports.keys() {
        fs::copy(directory.join(name), retained.join("coverage").join(name)).unwrap();
    }
    write_receipt(&retained.join("coverage"), &receipt);
    fs::write(generated.join("coverage/current"), "run-failed").unwrap();
    assert!(
        verify_gate_at(
            QualityGate::Coverage,
            &receipt.inputs_sha256,
            &generated,
            &retained,
            &settings
        )
        .is_err()
    );
    fs::remove_dir_all(directory).unwrap();
}
