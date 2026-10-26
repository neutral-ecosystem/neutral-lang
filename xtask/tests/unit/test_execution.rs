// SPDX-License-Identifier: Apache-2.0

//! Backend and inventory boundary tests.

use super::*;

/// Empty native iterations still test the complete workspace with the same mutation profile.
#[test]
fn mutation_zero_pending_baseline_preserves_backend_profile_and_scope() {
    let settings = configuration::automation().unwrap().testing;
    for backend in [TestRunner::Nextest, TestRunner::Cargo] {
        let baseline = mutation_baseline_arguments(backend, &settings);
        assert!(baseline.contains(&flags::WORKSPACE.to_owned()));
        assert!(baseline.contains(&flags::LOCKED.to_owned()));
        if backend == TestRunner::Nextest {
            assert_eq!(&baseline[..2], ["nextest", "run"]);
            let mutation =
                mutation_arguments(backend, &["src/lib.rs".to_owned()], "reports", &settings);
            let separator = mutation
                .iter()
                .position(|argument| argument == "--")
                .unwrap();
            assert_eq!(&baseline[4..], &mutation[separator + 1..]);
        } else {
            assert_eq!(baseline, ["test", flags::WORKSPACE, flags::LOCKED]);
        }
    }
}

/// Argument construction consumes its supplied snapshot, never a fresh configuration read.
#[test]
fn command_arguments_use_one_local_settings_snapshot() {
    let mut execution = TestExecution::load().unwrap();
    execution.testing.ci_profile = "snapshot-profile".to_owned();
    execution.testing.config = "config/snapshot.toml".to_owned();
    execution.verbose = true;
    let args = execution
        .arguments(TestRunner::Nextest, "run", false, true, None)
        .unwrap();
    assert!(args.contains(&"snapshot-profile".to_owned()));
    assert!(args.iter().any(|arg| arg.ends_with("config/snapshot.toml")));
    assert!(
        args.windows(2)
            .any(|pair| pair == ["--status-level", "pass"])
    );
    assert_ne!(
        TestExecution::load().unwrap().testing.ci_profile,
        "snapshot-profile"
    );
}

/// Analysis uses native Nextest integrations without changing coverage targets or fallback policy.
#[test]
fn analysis_commands_share_the_selected_test_backend() {
    for backend in [TestRunner::Nextest, TestRunner::Cargo] {
        let coverage = TestExecution::load()
            .unwrap()
            .coverage_arguments(backend)
            .expect("coverage command");
        assert_eq!(coverage[0], "llvm-cov");
        assert!(coverage.contains(&"--all-targets".to_owned()));
        assert!(coverage.contains(&"--locked".to_owned()));
        assert!(coverage.contains(&"--no-report".to_owned()));
        assert!(!coverage.contains(&"--lib".to_owned()));
        let mutation = mutation_arguments(
            backend,
            &["src/parser.rs".to_owned(), "src/decoder.rs".to_owned()],
            "reports",
            &configuration::automation().unwrap().testing,
        );
        assert!(mutation.contains(&"--no-config".to_owned()));
        assert!(mutation.contains(&"--cargo-arg=--workspace".to_owned()));
        assert!(
            mutation
                .windows(2)
                .any(|pair| pair == ["--copy-vcs", "true"])
        );
        assert!(mutation.windows(2).any(|pair| pair == ["--jobs", "2"]));
        assert!(
            mutation
                .windows(2)
                .any(|pair| pair == ["--gitignore", "true"])
        );
        assert!(!mutation.contains(&"--test-workspace".to_owned()));
        if backend == TestRunner::Nextest {
            assert_eq!(coverage[1], "nextest");
            assert!(coverage.contains(&"--ignore-default-filter".to_owned()));
            assert_eq!(mutation[2], "nextest");
            let settings = configuration::automation().expect("settings").testing;
            assert!(mutation.contains(&settings.config));
            assert!(mutation.contains(&settings.mutation_profile));
            assert!(mutation.contains(&"--ignore-default-filter".to_owned()));
            assert!(mutation.contains(&"--fail-fast".to_owned()));
        } else {
            assert!(!coverage.contains(&"nextest".to_owned()));
            assert_eq!(mutation[2], "cargo");
        }
        assert_eq!(
            &mutation[3..9],
            [
                "--output",
                "reports",
                "--file",
                "src/parser.rs",
                "--file",
                "src/decoder.rs"
            ]
        );
    }
}

/// Counts prefixes on test names only, retaining distinct binaries' same-named tests.
#[test]
fn automation_test_counts_ignore_metadata_and_substrings() {
    let minima = BTreeMap::from([("unit".to_owned(), 1), ("security".to_owned(), 1)]);
    let discovered = counts(
        &minima,
        [
            "tests::unit_first",
            "other::unit_first",
            "security_check",
            "tests::not_unit_test",
            "unit_test::helper",
        ]
        .into_iter(),
    );
    assert_eq!(discovered["unit"], 2);
    assert_eq!(discovered["security"], 1);
    let listing = "path/security_binary\ntests::unit_first: test\nunit_fake: benchmark\n1 test, 0 benchmarks\n";
    assert_eq!(
        cargo_test_names(listing).collect::<Vec<_>>(),
        ["tests::unit_first"]
    );
}

/// Unsupported backends and unsafe category filters fail rather than silently changing policy.
#[test]
fn automation_test_runner_selection_is_explicit() {
    for runner in [TestRunner::Nextest, TestRunner::Cargo] {
        assert_eq!(parse_runner(runner.as_str()).unwrap(), runner);
    }
    assert_eq!(
        parse_runner("nextest").expect("nextest"),
        TestRunner::Nextest
    );
    assert_eq!(parse_runner("cargo").expect("cargo"), TestRunner::Cargo);
    assert!(parse_runner("unknown").is_err());
    assert!(parse_runner("").is_err());
    assert!(parse_runner("Cargo").is_err());
    assert!(parse_runner(" nextest").is_err());
    assert!(run_filter("bad) or all()").is_err());
    assert!(run_filter("").is_err());
}

/// Full-gate commands ignore default filters and use the configured CI profile.
#[test]
fn automation_test_arguments_keep_full_gate_complete() {
    let config = configuration::automation().expect("automation settings");
    let args = TestExecution::load()
        .unwrap()
        .arguments(TestRunner::Nextest, "run", false, true, Some("security"))
        .expect("nextest args");
    assert!(args.contains(&config.testing.ci_profile));
    assert!(args.contains(&"--ignore-default-filter".to_owned()));
    assert!(args.contains(&"--tests".to_owned()));
    assert!(args.contains(&"--locked".to_owned()));
    assert!(args.contains(&"test(/(^|::)security_/)".to_owned()));
    let unit = TestExecution::load()
        .unwrap()
        .arguments(TestRunner::Cargo, "run", true, false, None)
        .expect("cargo args");
    assert!(!unit.contains(&"--tests".to_owned()));
    assert_eq!(unit[0], "test");
}

/// Unit commands exclude cross-package and explicitly categorized suites on both backends.
#[test]
fn unit_selection_is_not_all_library_tests() {
    for backend in [TestRunner::Nextest, TestRunner::Cargo] {
        let args = TestExecution::load()
            .unwrap()
            .arguments(backend, "run", true, false, None)
            .unwrap();
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--exclude", constants::NEUTRAL_TEST_SUITE])
        );
        for category in [
            "integration_",
            "system_",
            "conformance_",
            "property_",
            "security_",
        ] {
            assert!(
                args.iter().any(|argument| argument.contains(category)),
                "unit command did not exclude {category}"
            );
        }
        let all = TestExecution::load()
            .unwrap()
            .arguments(backend, "run", false, false, None)
            .unwrap();
        assert!(!all.contains(&"--exclude".to_owned()));
    }
}

/// Compact reporting hides only successful per-test rows, never failures or test selection.
#[test]
fn compact_reporting_preserves_failure_visibility() {
    let mut nextest = vec!["nextest".to_owned(), "run".to_owned()];
    reporting_arguments(&mut nextest, TestRunner::Nextest, false);
    assert_eq!(
        nextest,
        [
            "nextest",
            "run",
            "--status-level",
            "slow",
            "--final-status-level",
            "fail",
            "--show-progress",
            "auto"
        ]
    );
    let mut cargo = vec!["test".to_owned(), "--".to_owned(), "security_".to_owned()];
    reporting_arguments(&mut cargo, TestRunner::Cargo, false);
    assert_eq!(cargo, ["test", "--", "security_", "--quiet"]);
}

/// Verbosity is configurable and never adds human-report flags to typed listings.
#[test]
fn verbose_reporting_is_explicit() {
    assert!(parse_verbose("true").unwrap());
    assert!(parse_verbose("1").unwrap());
    assert!(!parse_verbose("false").unwrap());
    assert!(!parse_verbose("0").unwrap());
    assert!(parse_verbose("maybe").is_err());
    let mut args = Vec::new();
    reporting_arguments(&mut args, TestRunner::Nextest, true);
    assert!(
        args.windows(2)
            .any(|pair| pair == ["--status-level", "pass"])
    );
    let listing = TestExecution::load()
        .unwrap()
        .arguments(TestRunner::Nextest, "list", false, true, None)
        .unwrap();
    assert!(!listing.contains(&"--status-level".to_owned()));
}

/// Worker count is an execution setting, independent of backend and mutation scope.
#[test]
fn mutation_commands_use_configured_workers() {
    let source = crate::read_workspace_text(
        &crate::workspace_root().unwrap(),
        constants::AUTOMATION_CONFIG_FILE,
    )
    .unwrap();
    let mut config: toml::Table = configuration::parse(&source, "automation").unwrap();
    config["testing"]["mutation_jobs"] = toml::Value::Integer(3);
    let testing = configuration::parse::<crate::configuration_models::Automation>(
        &toml::to_string(&config).unwrap(),
        "automation",
    )
    .unwrap()
    .testing;
    for backend in [TestRunner::Cargo, TestRunner::Nextest] {
        let arguments =
            mutation_arguments(backend, &["src/parser.rs".to_owned()], "reports", &testing);
        assert!(arguments.windows(2).any(|pair| pair == ["--jobs", "3"]));
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--file", "src/parser.rs"])
        );
    }
}
