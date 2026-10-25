// SPDX-License-Identifier: Apache-2.0

//! Reusable support for Neutral tests.
//!
//! This non-published crate owns test-only builders, fixture loading, assertions,
//! and comparators. It must not contain production behavior or own executable
//! cross-package test suites.

use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

/// Monotonic suffix shared by temporary directories in this test process.
static DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Owns a newly created directory and removes it when the test leaves scope.
pub struct TestDirectory(PathBuf);

impl TestDirectory {
    /// Creates an isolated directory without accepting leftovers from an earlier process.
    ///
    /// # Errors
    /// Returns filesystem errors other than an existing candidate directory.
    pub fn new() -> io::Result<Self> {
        loop {
            let sequence = DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("neutral-test-{}-{sequence}", std::process::id()));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self(path)),
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error),
            }
        }
    }

    /// Returns the directory owned by this fixture.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
