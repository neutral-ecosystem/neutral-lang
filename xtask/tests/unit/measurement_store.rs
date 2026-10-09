// SPDX-License-Identifier: Apache-2.0

//! Process-level ownership checks for generated measurement reports.

use super::*;
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};

/// Child process used to exercise abrupt termination while holding the real store lock.
#[test]
fn measurement_lock_child() {
    let Some(root) = std::env::var_os("NEUTRAL_TEST_MEASUREMENT_LOCK") else {
        return;
    };
    let store = MeasurementStore::begin(Path::new(&root)).unwrap();
    fs::write(Path::new(&root).join("ready"), "ready").unwrap();
    let _keep_alive = store;
    loop {
        std::thread::park();
    }
}

/// Process exit releases locks; different gates never block each other.
#[test]
fn measurement_lock_survives_competition_and_releases_after_kill() {
    let root =
        std::env::temp_dir().join(format!("neutral-measurement-lock-{}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    let gate = root.join("coverage");
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "quality::measurement_store::tests::measurement_lock_child",
            "--nocapture",
        ])
        .env("NEUTRAL_TEST_MEASUREMENT_LOCK", &gate)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let start = Instant::now();
    while !gate.join("ready").exists() {
        if start.elapsed() > Duration::from_secs(10) || child.try_wait().unwrap().is_some() {
            let _ = child.kill();
            let _ = child.wait();
            panic!("lock child did not become ready");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let competing = MeasurementStore::begin(&gate).is_err();
    let independent = MeasurementStore::begin(&root.join("mutation")).is_ok();
    child.kill().unwrap();
    child.wait().unwrap();
    assert!(competing);
    assert!(independent);
    let next = MeasurementStore::begin(&gate).unwrap();
    assert!(gate.join("measurement.lock").is_file());
    drop(next);
    fs::remove_dir_all(root).unwrap();
}
