// SPDX-License-Identifier: Apache-2.0

//! Exercise development reuse and full-evidence separation through the real executable.

#![cfg(unix)]

mod support;

use std::{
    fs,
    process::{Command, Output},
};
use support::Workspace as Fixture;

impl Fixture {
    /// Installs the mutation protocol in an isolated command workspace.
    fn new() -> Self {
        Self::with_cargo(
            r#"#!/bin/sh
if [ "$2" = "--version" ] || [ "$1" = "--version" ]; then
    printf 'fixture tool 1\n'
    exit 0
fi
if [ "$1" = "nextest" ] || [ "$1" = "test" ]; then
    printf '%s\n' "$*" >> control/baselines
    test ! -f control/baseline-fail
    exit $?
fi
printf '%s\n' "$*" >> control/invocations
output=
iterate=false
while [ "$#" -gt 0 ]; do
    case "$1" in
        --output) shift; output=$1 ;;
        --iterate) iterate=true ;;
    esac
    shift
done
test -n "$output" || exit 90
mkdir -p "$output/mutants.out"
if [ "$iterate" = true ] && [ -f "$output/mutants.out/caught.txt" ]; then
    printf 'resumed\n' >> control/resumes
    if [ -f "$output/mutants.out/previously_caught.txt" ] || grep -q '"missed":0' "$output/mutants.out/outcomes.json"; then
        cp "$output/mutants.out/caught.txt" "$output/mutants.out/previously_caught.txt"
        rm -f "$output/mutants.out/outcomes.json"
        printf '[]' > "$output/mutants.out/mutants.json"
        exit 0
    fi
fi
printf 'caught mutant\n' > "$output/mutants.out/caught.txt"
if [ -f control/fail ]; then
    printf '{"caught":1,"missed":1,"timeout":0,"success":0}\n' > "$output/mutants.out/outcomes.json"
    exit 2
fi
printf '{"caught":1,"missed":0,"timeout":0,"success":0}\n' > "$output/mutants.out/outcomes.json"
"#,
        )
    }

    /// Keeps execution environment stable across repeated command invocations.
    fn command(&self, full: bool) -> Command {
        let mut command = self.xtask("mutate");
        if full {
            command.arg("--full");
        }
        command
    }

    /// Executes the configured fixture command with captured diagnostic streams.
    fn invoke(&self, full: bool) -> Output {
        self.command(full).output().unwrap()
    }

    /// Counts mutation executions, excluding version and backend availability queries.
    fn runs(&self) -> usize {
        fs::read_to_string(self.root.join("control/invocations"))
            .unwrap()
            .lines()
            .count()
    }

    /// Finds the current local campaign, not a quality receipt.
    fn state(&self) -> serde_json::Value {
        serde_json::from_slice(
            &fs::read(
                self.root
                    .join("test-results/analysis/quality-gates/mutation-development/state.json"),
            )
            .unwrap(),
        )
        .unwrap()
    }
}

/// Real dispatch resumes failures, caches passes, invalidates edits and isolates full receipts.
#[test]
fn mutation_development_and_full_command_lifecycle() {
    let fixture = Fixture::new();
    fs::write(fixture.root.join("control/fail"), "fail first attempt").unwrap();
    let failed = fixture.invoke(false);
    assert!(
        !failed.status.success(),
        "{}",
        String::from_utf8_lossy(&failed.stderr)
    );
    assert!(
        String::from_utf8_lossy(&failed.stderr).contains("retry survivors/unfinished mutants"),
        "{}",
        String::from_utf8_lossy(&failed.stderr)
    );
    assert_eq!(fixture.runs(), 1);
    let original = fixture.state()["directory"].clone();
    fs::remove_file(fixture.root.join("control/fail")).unwrap();
    let resumed = fixture.invoke(false);
    assert!(
        resumed.status.success(),
        "{}",
        String::from_utf8_lossy(&resumed.stderr)
    );
    assert_eq!(fixture.runs(), 2);
    assert_eq!(fixture.state()["directory"], original);
    assert_eq!(
        fs::read_to_string(fixture.root.join("control/resumes")).unwrap(),
        "resumed\n"
    );
    let cached = fixture.invoke(false);
    assert!(cached.status.success());
    assert_eq!(fixture.runs(), 2);
    assert!(String::from_utf8_lossy(&cached.stderr).contains("unchanged passing campaign reused"));

    fs::write(fixture.root.join("tests/new.rs"), "additional test").unwrap();
    assert!(fixture.invoke(false).status.success());
    assert_eq!(fixture.runs(), 3);
    assert_eq!(fixture.state()["directory"], original);
    assert_eq!(
        fs::read_to_string(fixture.root.join("control/baselines"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    fs::write(fixture.root.join("tests/broken.rs"), "new broken test").unwrap();
    fs::write(fixture.root.join("control/baseline-fail"), "fail baseline").unwrap();
    assert!(!fixture.invoke(false).status.success());
    assert_eq!(fixture.runs(), 4);
    assert!(fixture.state()["passed_reports"].is_null());
    fs::remove_file(fixture.root.join("control/baseline-fail")).unwrap();
    assert!(fixture.invoke(false).status.success());
    assert_eq!(fixture.runs(), 5);
    assert_eq!(fixture.state()["directory"], original);
    fs::write(fixture.root.join("tests/new.rs"), "modified existing test").unwrap();
    assert!(fixture.invoke(false).status.success());
    assert_eq!(fixture.runs(), 6);
    let after_test_edit = fixture.state()["directory"].clone();
    assert_ne!(after_test_edit, original);
    fs::write(fixture.root.join("source.rs"), "changed source").unwrap();
    assert!(fixture.invoke(false).status.success());
    assert_eq!(fixture.runs(), 7);
    assert_ne!(fixture.state()["directory"], after_test_edit);

    let before_environment = fixture.state()["directory"].clone();
    assert!(
        fixture
            .command(false)
            .env("NEUTRAL_MUTATION_FIXTURE_BINDING", "changed")
            .output()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(fixture.runs(), 8);
    assert_ne!(fixture.state()["directory"], before_environment);

    let gate_root = fixture.root.join("test-results/analysis/quality-gates");
    assert_eq!(fs::read_dir(&gate_root).unwrap().count(), 1);
    for expected in [9, 10] {
        let full = fixture.invoke(true);
        assert!(
            full.status.success(),
            "{}",
            String::from_utf8_lossy(&full.stderr)
        );
        assert_eq!(fixture.runs(), expected);
        let invocations = fs::read_to_string(fixture.root.join("control/invocations")).unwrap();
        assert!(!invocations.lines().last().unwrap().contains("--iterate"));
    }
    assert_eq!(fs::read_dir(gate_root).unwrap().count(), 2);
}
