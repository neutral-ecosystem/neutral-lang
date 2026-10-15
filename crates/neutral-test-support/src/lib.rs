// SPDX-License-Identifier: Apache-2.0

//! Reusable support for Neutral tests.
//!
//! This non-published crate owns test-only builders, fixture loading, assertions,
//! and comparators. It must not contain production behavior or own executable
//! cross-package test suites.

use neutral_core::{
    CancellationToken, SemanticDigest, SourceContentDigest, profile::V1_SOURCE_PROFILE,
};
use neutral_ir::{
    LogicalModuleIdentity,
    composition::{
        profile,
        project::{CompositionPolicy, CompositionProjectIr, CompositionResourceFacts},
    },
    project::{ProjectLimits, ProjectModule, ProjectResourceFacts, ProjectSource},
    project_identity::{IdentityLimits, MAX_TRANSCRIPT_BYTES, composition_interface},
};
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

/// Builds complete, compiler-independent composition IR with no declarations.
///
/// # Panics
/// Panics if the fixed fixture cannot be framed under the supplied project controls.
#[must_use]
pub fn empty_composition_ir(
    limits: ProjectLimits,
    policy: CompositionPolicy,
) -> CompositionProjectIr {
    let source = b"neu \"1.0\"\nmodule example\n";
    let mut ir = CompositionProjectIr {
        schema: profile::PROJECT_IR_SCHEMA.to_owned(),
        modules: vec![ProjectModule {
            identity: LogicalModuleIdentity::new(V1_SOURCE_PROFILE, "example"),
            imports: vec![],
        }],
        declarations: vec![],
        vocabularies: vec![],
        interface_digest: SemanticDigest::from_raw_bytes([0; 32]),
        sources: vec![ProjectSource {
            module: "example".into(),
            source_id: "source:example".into(),
            digest: SourceContentDigest::from_bytes(source),
            byte_len: source.len() as u64,
        }],
        source_maps: vec![],
        provenance: vec![],
        limits,
        resources: ProjectResourceFacts {
            source_units: 1,
            source_bytes: source.len() as u64,
            vocabulary_units: 0,
            vocabulary_bytes: 0,
            declarations: 0,
            import_edges: 0,
            value_nodes: 0,
        },
        vocabulary_sources: vec![],
        origins: vec![],
        composition_limits: policy,
        composition_resources: CompositionResourceFacts::from_values([0; 7]),
    };
    ir.interface_digest = composition_interface(
        &ir,
        IdentityLimits {
            bytes: MAX_TRANSCRIPT_BYTES,
            nodes: limits.nodes,
        },
        &CancellationToken::new(),
    )
    .unwrap()
    .identity();
    ir
}
