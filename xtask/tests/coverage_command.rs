// SPDX-License-Identifier: Apache-2.0

//! Command-level coverage reuse, invalidation and failure recovery without running real tests.

#![cfg(unix)]

mod support;
use std::{fs, process::Output};
use support::Workspace;

/// Implements only the external coverage protocol; the actual xtask owns selection and caching.
const CARGO: &str = r#"#!/bin/sh
if [ "$2" = "--version" ] || [ "$1" = "--version" ]; then
    printf 'fixture tool 1\n'
    exit 0
fi
if [ "$1" = "nextest" ]; then exit 0; fi
[ "$1" = "llvm-cov" ] || exit 90
printf '%s\n' "$*" >> control/invocations
case "$2" in
    clean) exit 0 ;;
    nextest) test ! -f control/fail; exit $? ;;
    report)
        while [ "$#" -gt 0 ]; do
            case "$1" in
                --output-dir)
                    shift
                    mkdir -p "$1/html"
                    printf 'coverage report\n' > "$1/html/index.html"
                    exit 0 ;;
                --output-path)
                    shift
                    printf '{"data":[{"totals":{"lines":{"percent":100},"functions":{"percent":100},"regions":{"percent":100}}}]}\n' > "$1"
                    exit 0 ;;
            esac
            shift
        done ;;
esac
exit 91
"#;

/// Invokes real command dispatch and preserves the process diagnostics for assertions.
fn invoke(workspace: &Workspace, success: bool) -> Output {
    let output = workspace.xtask("coverage").output().unwrap();
    assert_eq!(
        output.status.success(),
        success,
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

/// Records every actual tool run, excluding version and availability queries.
fn invocations(workspace: &Workspace) -> Vec<String> {
    fs::read_to_string(workspace.root.join("control/invocations"))
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// Unchanged files reuse evidence; source and guide edits clear reports and rerun the whole workspace.
#[test]
fn coverage_reuses_only_identical_files_and_cleans_changed_runs() {
    let workspace = Workspace::with_cargo(CARGO);
    invoke(&workspace, true);
    let first = invocations(&workspace);
    assert_eq!(first.len(), 4);
    assert_eq!(first[0], "llvm-cov clean --workspace");
    assert!(first[1].starts_with("llvm-cov nextest "));
    assert!(
        first[1]
            .split_whitespace()
            .any(|argument| argument == "--workspace")
    );
    let cached = invoke(&workspace, true);
    assert!(String::from_utf8_lossy(&cached.stderr).contains("inputs unchanged"));
    assert_eq!(invocations(&workspace), first);

    for (index, file) in ["source.rs", "README.md"].iter().enumerate() {
        let stale = workspace
            .root
            .join("test-results/analysis/coverage/stale.html");
        fs::write(&stale, "old report").unwrap();
        fs::write(workspace.root.join(file), "changed file").unwrap();
        invoke(&workspace, true);
        assert!(!stale.exists());
        assert_eq!(invocations(&workspace).len(), first.len() * (index + 2));
        invoke(&workspace, true);
        assert_eq!(invocations(&workspace).len(), first.len() * (index + 2));
    }
}

/// Missing exported reports/evidence and failed reruns must never be accepted as reusable coverage.
#[test]
fn coverage_recovers_from_missing_reports_and_failed_runs() {
    let workspace = Workspace::with_cargo(CARGO);
    invoke(&workspace, true);
    let report = workspace.root.join("test-results/analysis/coverage");
    let mut runs = invocations(&workspace).len();
    for missing in ["html/index.html", "coverage.json"] {
        fs::remove_file(report.join(missing)).unwrap();
        invoke(&workspace, true);
        runs += 4;
        assert_eq!(
            invocations(&workspace).len(),
            runs,
            "missing report: {missing}"
        );
    }
    fs::remove_dir_all(workspace.root.join("test-results/analysis/quality-gates")).unwrap();
    invoke(&workspace, true);
    runs += 4;
    assert_eq!(invocations(&workspace).len(), runs);

    fs::write(workspace.root.join("source.rs"), "broken source").unwrap();
    fs::write(workspace.root.join("control/fail"), "fail tests").unwrap();
    invoke(&workspace, false);
    assert!(!report.join("inputs.sha256").exists());
    runs += 2;
    assert_eq!(invocations(&workspace).len(), runs);
    fs::remove_file(workspace.root.join("control/fail")).unwrap();
    invoke(&workspace, true);
    runs += 4;
    assert_eq!(invocations(&workspace).len(), runs);
    invoke(&workspace, true);
    assert_eq!(invocations(&workspace).len(), runs);
}
