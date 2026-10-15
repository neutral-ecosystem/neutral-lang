// SPDX-License-Identifier: Apache-2.0

//! Disposable workspaces for testing the actual xtask executable with controlled tools.

use neutral_test_support::TestDirectory;
use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
};

/// Owns an independent Git workspace and its deterministic test executables.
pub struct Workspace {
    /// Checkout used as the command's invocation directory.
    pub root: PathBuf,
    /// Removes the complete fixture on scope exit, including failed tests.
    _directory: TestDirectory,
}

impl Workspace {
    /// Creates current automation configuration and commits the fixed fixture inputs.
    pub fn with_cargo(script: &str) -> Self {
        let directory = TestDirectory::new().unwrap();
        let root = directory.path().join("workspace");
        for name in ["config", "tests", "control"] {
            fs::create_dir_all(root.join(name)).unwrap();
        }
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        for path in [
            "config/automation.toml",
            "config/quality-gates.toml",
            "config/nextest.toml",
            "config/dependency-sources.toml",
        ] {
            fs::copy(source.join(path), root.join(path)).unwrap();
        }
        fs::write(
            root.join("Cargo.toml"),
            format!(
                "[workspace]\nmembers = []\n[workspace.package]\nversion = \"{}\"\n",
                env!("CARGO_PKG_VERSION")
            ),
        )
        .unwrap();
        fs::write(root.join(".gitignore"), "test-results/\ncontrol/\n").unwrap();
        fs::write(root.join("source.rs"), "initial source").unwrap();
        for (name, contents) in [
            ("fake-cargo", script),
            (
                "fake-rustc",
                "#!/bin/sh\nprintf 'rustc 1.99.0-nightly (fixture)\\n'\n",
            ),
        ] {
            let path = root.join(name);
            fs::write(&path, contents).unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let workspace = Self {
            root,
            _directory: directory,
        };
        workspace.git(&["init", "--quiet", "--initial-branch=main"]);
        workspace.git(&["add", "."]);
        workspace.git(&[
            "-c",
            "user.name=Command fixture",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "commit",
            "--quiet",
            "-m",
            "fixture",
        ]);
        workspace
    }

    /// Runs local Git operations without touching the real checkout.
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

    /// Configures the built xtask binary to use only fixture-owned tools and output.
    pub fn xtask(&self, task: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_xtask"));
        command
            .arg(task)
            .current_dir(&self.root)
            .env("NEUTRAL_CARGO_COMMAND", self.root.join("fake-cargo"))
            .env("NEUTRAL_RUSTC_COMMAND", self.root.join("fake-rustc"))
            .env("NEUTRAL_TEST_RUNNER", "nextest")
            .env_remove("NEUTRAL_TEST_RESULTS")
            .env("NO_COLOR", "1");
        command
    }
}
