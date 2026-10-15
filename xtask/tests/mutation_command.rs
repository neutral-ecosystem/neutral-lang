// SPDX-License-Identifier: Apache-2.0

//! Exercise development reuse and full-evidence separation through the real executable.

#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

/// Isolated workspace whose mutation tool finishes immediately instead of running real mutants.
struct Fixture {
    /// Disposable Git workspace containing copied, current automation configuration.
    root: PathBuf,
}

impl Fixture {
    /// Installs a deterministic tool protocol fixture with native iteration files.
    fn new() -> Self {
        let parent =
            std::env::temp_dir().join(format!("neutral-mutation-command-{}", std::process::id()));
        fs::create_dir_all(&parent).unwrap();
        let root = parent.join("workspace");
        fs::create_dir(&root).unwrap();
        fs::create_dir(root.join("config")).unwrap();
        fs::create_dir(root.join("tests")).unwrap();
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        for path in [
            "config/automation.toml",
            "config/quality-gates.toml",
            "config/nextest.toml",
            "config/dependency-sources.toml",
        ] {
            fs::copy(source.join(path), root.join(path)).unwrap();
        }
        fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
        fs::write(root.join(".gitignore"), "test-results/\ncontrol/\n").unwrap();
        fs::write(root.join("source.rs"), "initial source").unwrap();
        fs::create_dir(root.join("control")).unwrap();
        let tool = root.join("fake-cargo");
        fs::write(
            &tool,
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
        .unwrap();
        fs::set_permissions(&tool, fs::Permissions::from_mode(0o755)).unwrap();
        let fixture = Self { root };
        fixture.git(&["init", "--quiet", "--initial-branch=main"]);
        fixture.git(&["add", "."]);
        fixture.git(&[
            "-c",
            "user.name=Mutation fixture",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ]);
        fixture
    }

    /// Runs only local Git operations in the disposable workspace.
    fn git(&self, arguments: &[&str]) {
        assert!(
            Command::new("git")
                .args(arguments)
                .current_dir(&self.root)
                .status()
                .unwrap()
                .success()
        );
    }

    /// Keeps execution environment stable across repeated command invocations.
    fn command(&self, full: bool) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
        command
            .arg("mutate")
            .current_dir(&self.root)
            .env("NEUTRAL_CARGO_COMMAND", self.root.join("fake-cargo"))
            .env_remove("NEUTRAL_TEST_RESULTS")
            .env("NO_COLOR", "1");
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

impl Drop for Fixture {
    /// Removes only this test's validated disposable workspace.
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
        fs::remove_dir(self.root.parent().unwrap()).unwrap();
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
