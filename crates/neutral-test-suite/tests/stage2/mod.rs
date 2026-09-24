// SPDX-License-Identifier: Apache-2.0

//! Executable Stage 2 captured-project fixtures and validation evidence.

use neutral_compiler::{
    CAPTURE_REQUEST_VERSION, CapturedProjectRequest, CapturedProjectRequestBuilder,
    CapturedSourceInput, CapturedVocabularyInput, ProjectCaptureControls, ProjectCaptureError,
    ProjectCaptureLimitValues, ProjectCaptureLimits, ProjectHostError, capture_project,
};
use neutral_core::{
    CancellationToken, VocabularyContentDigest,
    profile::{LanguageProfile, V0_SOURCE_PROFILE, V1_SOURCE_PROFILE},
};
use neutral_vocabulary::VocabularyLock;
use std::collections::BTreeMap;

/// Generic scalar table decoded from the closed fixture subset.
type Table = BTreeMap<String, String>;

/// Minimal closed representation of one reviewed TOML fixture.
#[derive(Default)]
struct Fixture {
    /// Root scalar fields.
    root: Table,
    /// Named scalar tables.
    tables: BTreeMap<String, Table>,
    /// Repeated tables.
    arrays: BTreeMap<String, Vec<Table>>,
}

/// Current parser destination while reading a reviewed fixture.
enum Section {
    /// Root scalar fields.
    Root,
    /// One named table.
    Table(String),
    /// One repeated table and its current index.
    Array(String, usize),
}

/// Parses exactly the scalar/table subset used by Stage 2 fixtures.
fn parse_fixture(text: &str) -> Fixture {
    let mut fixture = Fixture::default();
    let mut section = Section::Root;
    for source_line in text.lines() {
        let line = source_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(name) = line
            .strip_prefix("[[")
            .and_then(|value| value.strip_suffix("]]"))
        {
            let tables = fixture.arrays.entry(name.to_owned()).or_default();
            tables.push(Table::new());
            section = Section::Array(name.to_owned(), tables.len() - 1);
            continue;
        }
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
            fixture.tables.entry(name.to_owned()).or_default();
            section = Section::Table(name.to_owned());
            continue;
        }
        let (key, value) = line
            .split_once('=')
            .expect("reviewed fixture line must be key/value");
        let target = match &section {
            Section::Root => &mut fixture.root,
            Section::Table(name) => fixture
                .tables
                .get_mut(name)
                .expect("named table must exist"),
            Section::Array(name, index) => fixture
                .arrays
                .get_mut(name)
                .and_then(|tables| tables.get_mut(*index))
                .expect("array table must exist"),
        };
        assert!(
            target
                .insert(key.trim().to_owned(), value.trim().to_owned())
                .is_none(),
            "reviewed fixture keys must be unique"
        );
    }
    fixture
}

/// Decodes the TOML basic-string escapes used by reviewed fixtures.
fn string(value: &str) -> String {
    let body = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .expect("reviewed string must be quoted");
    let mut output = String::new();
    let mut characters = body.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            output.push(character);
            continue;
        }
        match characters.next().expect("escape must have a value") {
            'n' => output.push('\n'),
            'r' => output.push('\r'),
            't' => output.push('\t'),
            '"' => output.push('"'),
            '\\' => output.push('\\'),
            other => panic!("unsupported reviewed fixture escape: {other}"),
        }
    }
    output
}

/// Returns one required decoded string field.
fn required_string(table: &Table, key: &str) -> String {
    string(table.get(key).expect("reviewed fixture field must exist"))
}

/// Returns one required positive integer field.
fn number(table: &Table, key: &str) -> u64 {
    table
        .get(key)
        .expect("reviewed numeric field must exist")
        .parse()
        .expect("reviewed numeric field must be unsigned")
}

/// Decodes one reviewed lowercase hexadecimal byte sequence.
fn hexadecimal(value: &str) -> Vec<u8> {
    let value = string(value);
    assert_eq!(value.len() % 2, 0, "hexadecimal length must be even");
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let text = std::str::from_utf8(pair).expect("hex pair must be UTF-8");
            u8::from_str_radix(text, 16).expect("hex pair must be lowercase hexadecimal")
        })
        .collect()
}

/// Converts one complete reviewed request fixture to the public contract.
fn fixture_limits(fixture: &Fixture) -> ProjectCaptureLimits {
    let controls = fixture
        .tables
        .get("controls")
        .expect("capture fixture must have controls");
    ProjectCaptureLimits::new(ProjectCaptureLimitValues {
        total_source_bytes: number(controls, "total_source_bytes"),
        source_bytes_per_unit: number(controls, "source_bytes_per_unit"),
        source_units: number(controls, "source_units"),
        source_id_bytes: number(controls, "source_id_bytes"),
        module_id_bytes: number(controls, "module_id_bytes"),
        vocabulary_units: number(controls, "vocabulary_units"),
        vocabulary_bytes_per_unit: number(controls, "vocabulary_bytes_per_unit"),
        total_vocabulary_bytes: number(controls, "total_vocabulary_bytes"),
        imports_per_module: number(controls, "imports_per_module"),
        import_edges: number(controls, "import_edges"),
        scc_units: number(controls, "scc_units"),
        declarations: number(controls, "declarations"),
        diagnostics: number(controls, "diagnostics"),
        output_bytes: number(controls, "output_bytes"),
    })
}

fn request_fixture(text: &str) -> CapturedProjectRequest {
    let fixture = parse_fixture(text);
    assert_eq!(
        required_string(&fixture.root, "schema"),
        "neutral.capture-fixture/1"
    );
    let profile = match required_string(&fixture.root, "profile").as_str() {
        V1_SOURCE_PROFILE => LanguageProfile::V1_0,
        V0_SOURCE_PROFILE => LanguageProfile::V0_1,
        other => panic!("unknown reviewed profile {other}"),
    };
    let controls = fixture
        .tables
        .get("controls")
        .expect("capture fixture must have controls");
    let limits = fixture_limits(&fixture);
    let cancellation = CancellationToken::new();
    if controls
        .get("cancelled")
        .is_some_and(|value| value == "true")
    {
        cancellation.cancel();
    }
    let sources = fixture
        .arrays
        .get("sources")
        .into_iter()
        .flatten()
        .map(|source| {
            CapturedSourceInput::new(
                required_string(source, "source_id"),
                required_string(source, "module_id"),
                required_string(source, "source_utf8").into_bytes(),
            )
        })
        .collect();
    let vocabularies = fixture
        .arrays
        .get("vocabularies")
        .into_iter()
        .flatten()
        .map(|vocabulary| {
            let bytes = hexadecimal(
                vocabulary
                    .get("bundle_hex")
                    .expect("vocabulary bytes must exist"),
            );
            let lock = VocabularyLock::new(
                required_string(vocabulary, "canonical_identity"),
                required_string(vocabulary, "semantic_version"),
                required_string(vocabulary, "encoding_version"),
                required_string(vocabulary, "schema_version"),
                VocabularyContentDigest::parse_text(&required_string(vocabulary, "content_sha256"))
                    .expect("reviewed digest must be valid"),
                Vec::new(),
            )
            .expect("reviewed lock must be valid");
            CapturedVocabularyInput::new(bytes, lock)
        })
        .collect();
    let mut request = CapturedProjectRequest::new(
        required_string(&fixture.root, "request_version"),
        profile,
        sources,
        vocabularies,
        ProjectCaptureControls::new(limits, cancellation),
    );
    if let Some(project_key) = fixture.root.get("project_key") {
        request = request.with_project_key(string(project_key));
    }
    request
}

/// Asserts one reviewed fixture's exact stable capture outcome.
fn assert_outcome(text: &str, expected: Result<(), ProjectCaptureError>) {
    let actual = capture_project(request_fixture(text)).map(|_| ());
    assert_eq!(actual, expected);
}

#[test]
fn conformance_stage2_all_request_fixtures_match_their_oracles() {
    for fixture in [
        include_str!("fixtures/positive/complete-closure.toml"),
        include_str!("fixtures/positive/disconnected-unit.toml"),
        include_str!("fixtures/positive/exact-vocabulary-lock.toml"),
        include_str!("fixtures/boundary/source-unit-limit-exact.toml"),
    ] {
        assert_outcome(fixture, Ok(()));
    }
    for (fixture, error) in [
        (
            include_str!("fixtures/negative/duplicate-source-id.toml"),
            ProjectCaptureError::DuplicateSourceId,
        ),
        (
            include_str!("fixtures/negative/duplicate-module-id.toml"),
            ProjectCaptureError::DuplicateModuleId,
        ),
        (
            include_str!("fixtures/negative/profile-header-mismatch.toml"),
            ProjectCaptureError::ProfileMismatch,
        ),
        (
            include_str!("fixtures/negative/module-header-mismatch.toml"),
            ProjectCaptureError::ModuleHeaderMismatch,
        ),
        (
            include_str!("fixtures/negative/missing-vocabulary-lock.toml"),
            ProjectCaptureError::MissingVocabulary,
        ),
        (
            include_str!("fixtures/negative/extra-vocabulary-lock.toml"),
            ProjectCaptureError::ExtraVocabulary,
        ),
        (
            include_str!("fixtures/negative/conflicting-vocabulary-lock.toml"),
            ProjectCaptureError::DuplicateVocabulary,
        ),
        (
            include_str!("fixtures/boundary/source-unit-limit-over.toml"),
            ProjectCaptureError::LimitExceeded,
        ),
    ] {
        assert_outcome(fixture, Err(error));
        assert_eq!(
            capture_project(request_fixture(fixture))
                .expect_err("negative fixture must fail")
                .code(),
            error.code()
        );
    }
}

#[test]
fn conformance_stage2_host_mapping_fixtures_use_the_shared_builder() {
    let equivalent = parse_fixture(include_str!(
        "fixtures/host-mapping/equivalent-host-mappings.toml"
    ));
    let controls = ProjectCaptureControls::new(
        ProjectCaptureLimits::new(ProjectCaptureLimitValues {
            total_source_bytes: 4096,
            source_bytes_per_unit: 1024,
            source_units: 1,
            source_id_bytes: 64,
            module_id_bytes: 64,
            vocabulary_units: 1,
            vocabulary_bytes_per_unit: 1024,
            total_vocabulary_bytes: 1024,
            imports_per_module: 8,
            import_edges: 8,
            scc_units: 1,
            declarations: 64,
            diagnostics: 16,
            output_bytes: 4096,
        }),
        CancellationToken::new(),
    );
    let mut builder = CapturedProjectRequestBuilder::new(LanguageProfile::V1_0, controls.clone());
    for name in ["left", "right"] {
        let mapping = equivalent
            .tables
            .get(name)
            .expect("equivalent mapping table must exist");
        assert!(!required_string(mapping, "host_location").is_empty());
        builder
            .add_source(CapturedSourceInput::new(
                required_string(mapping, "source_id"),
                required_string(mapping, "module_id"),
                required_string(mapping, "source_utf8").into_bytes(),
            ))
            .expect("equivalent mappings must coalesce");
    }
    assert_eq!(
        capture_project(builder.build())
            .expect("equivalent mapping must capture")
            .sources()
            .len(),
        1
    );

    let conflicting = parse_fixture(include_str!(
        "fixtures/host-mapping/conflicting-host-mapping.toml"
    ));
    let mut builder = CapturedProjectRequestBuilder::new(LanguageProfile::V1_0, controls);
    let mappings = conflicting
        .arrays
        .get("mappings")
        .expect("conflicting mappings must exist");
    for (index, mapping) in mappings.iter().enumerate() {
        let outcome = builder.add_source(CapturedSourceInput::new(
            required_string(mapping, "source_id"),
            required_string(mapping, "module_id"),
            required_string(mapping, "source_utf8").into_bytes(),
        ));
        if index == 0 {
            assert_eq!(outcome, Ok(()));
        } else {
            assert_eq!(outcome, Err(ProjectHostError::ConflictingSourceMapping));
        }
    }
}

#[test]
fn property_stage2_shuffled_capture_and_replay_are_meaning_equivalent() {
    let request = request_fixture(include_str!("fixtures/positive/disconnected-unit.toml"));
    let captured = capture_project(request).expect("fixture must capture");
    let replayed = capture_project(captured.replay_request(CancellationToken::new()))
        .expect("captured project must replay");
    assert!(captured.meaning_equivalent(&replayed));
    assert_eq!(captured, replayed);
    assert_eq!(captured.sources().len(), 3);
}

#[test]
fn security_stage2_malformed_cancelled_and_bounded_inputs_fail_without_capture() {
    let malformed = CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        vec![CapturedSourceInput::new(
            "source:malformed",
            "malformed::source",
            b"neu \"1.0\"\nmodule malformed::source\nmodule duplicate\n".to_vec(),
        )],
        Vec::new(),
        ProjectCaptureControls::new(
            ProjectCaptureLimits::new(ProjectCaptureLimitValues {
                total_source_bytes: 128,
                source_bytes_per_unit: 128,
                source_units: 1,
                source_id_bytes: 64,
                module_id_bytes: 64,
                vocabulary_units: 1,
                vocabulary_bytes_per_unit: 64,
                total_vocabulary_bytes: 64,
                imports_per_module: 1,
                import_edges: 1,
                scc_units: 1,
                declarations: 1,
                diagnostics: 1,
                output_bytes: 1,
            }),
            CancellationToken::new(),
        ),
    );
    assert_eq!(
        capture_project(malformed),
        Err(ProjectCaptureError::InvalidHeader)
    );

    let cancellation = CancellationToken::new();
    cancellation.cancel();
    let limit_fixture = parse_fixture(include_str!(
        "fixtures/boundary/source-unit-limit-exact.toml"
    ));
    let cancelled = CapturedProjectRequest::new(
        CAPTURE_REQUEST_VERSION,
        LanguageProfile::V1_0,
        vec![CapturedSourceInput::new(
            "source:cancelled",
            "cancelled::source",
            b"neu \"1.0\"\nmodule cancelled::source\n".to_vec(),
        )],
        Vec::new(),
        ProjectCaptureControls::new(fixture_limits(&limit_fixture), cancellation),
    );
    assert_eq!(
        capture_project(cancelled),
        Err(ProjectCaptureError::Cancelled)
    );
}
