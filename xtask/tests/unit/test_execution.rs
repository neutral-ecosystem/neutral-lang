// SPDX-License-Identifier: Apache-2.0

//! Backend and inventory boundary tests.

use super::*;

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
    assert_eq!(
        parse_runner("nextest").expect("nextest"),
        TestRunner::Nextest
    );
    assert_eq!(parse_runner("cargo").expect("cargo"), TestRunner::Cargo);
    assert!(parse_runner("unknown").is_err());
    assert!(parse_runner("").is_err());
    assert!(run_filter("bad) or all()").is_err());
    assert!(run_filter("").is_err());
}

/// Full-gate commands ignore default filters and use the configured CI profile.
#[test]
fn automation_test_arguments_keep_full_gate_complete() {
    let config = configuration::automation().expect("automation settings");
    let args =
        arguments(TestRunner::Nextest, "run", false, true, Some("security")).expect("nextest args");
    assert!(args.contains(&config.testing.ci_profile));
    assert!(args.contains(&"--ignore-default-filter".to_owned()));
    assert!(args.contains(&"--tests".to_owned()));
    assert!(args.contains(&"--locked".to_owned()));
    assert!(args.contains(&"test(/(^|::)security_/)".to_owned()));
    let unit = arguments(TestRunner::Cargo, "run", true, false, None).expect("cargo args");
    assert!(!unit.contains(&"--tests".to_owned()));
    assert_eq!(unit[0], "test");
}
