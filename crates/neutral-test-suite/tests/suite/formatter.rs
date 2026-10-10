// SPDX-License-Identifier: Apache-2.0

//! formatter cross-package regressions.

use super::*;

#[test]
/// Verifies the exact canonical header, indentation, field, list, and spacing style.
fn conformance_formatter_emits_canonical_layout() {
    let source = b"/* license */ neu \"0.1\"\r\nmodule style\r\nrecord Item{string name,}\r\nList<Item>items=[{name:\"x\",},]\r\n";
    let expected = b"/* license */\nneu \"0.1\"\nmodule style\n\nrecord Item {\n    string name,\n}\n\nList<Item> items = [\n    {\n        name: \"x\",\n    },\n]\n";
    assert_eq!(format_fixture(source, false), expected);
}

#[test]
/// Verifies the published complete language example compiles with its captured bundle.
fn conformance_documentation_showcase_compiles() {
    let source = LANGUAGE_SHOWCASE
        .split_once("```neu\n")
        .and_then(|(_, remainder)| remainder.split_once("\n```"))
        .map(|(source, _)| source.as_bytes())
        .expect("showcase must contain one complete Neutral example first");
    assert!(matches!(
        compile_with_vocabulary(source, VOCABULARY_BUNDLE),
        CompilationResult::Success(_)
    ));
}

#[test]
/// Verifies formatting every positive source fixture is exactly idempotent.
fn property_formatter_is_idempotent_across_positive_corpus() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("test-suite crate should be inside the workspace");
    let mut fixtures = Vec::new();
    collect_positive_sources(
        &workspace.join("conformance/releases/v0.1.0/specs/fixtures/positive"),
        &mut fixtures,
    );
    assert_ne!(fixtures.len(), 0);
    for fixture in fixtures {
        let source = fs::read(&fixture).expect("positive source should be readable");
        let uses_vocabulary = fixture
            .components()
            .any(|component| component.as_os_str() == "vocabulary");
        let once = format_fixture(&source, uses_vocabulary);
        let twice = format_fixture(&once, uses_vocabulary);
        assert_eq!(once, twice, "formatter was not idempotent: {fixture:?}");
    }
}

#[test]
/// Verifies formatting preserves logical IR and every accepted provenance category.
fn property_formatter_preserves_logic_and_provenance() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("test-suite crate should be inside the workspace");
    let mut fixtures = Vec::new();
    collect_positive_sources(
        &workspace.join("conformance/releases/v0.1.0/specs/fixtures/positive"),
        &mut fixtures,
    );
    for fixture in fixtures {
        let source = fs::read(&fixture).expect("positive source should be readable");
        let uses_vocabulary = fixture
            .components()
            .any(|component| component.as_os_str() == "vocabulary");
        let formatted = format_fixture(&source, uses_vocabulary);
        let before = compile_fixture(&source, uses_vocabulary);
        let after = compile_fixture(&formatted, uses_vocabulary);
        assert!(
            before.logically_equivalent(&after),
            "formatter changed logical IR: {fixture:?}"
        );
        assert_eq!(before.provenance(), after.provenance());
        assert_eq!(before.field_provenance(), after.field_provenance());
        assert_eq!(before.reuse_provenance(), after.reuse_provenance());
        assert_eq!(before.reference_provenance(), after.reference_provenance());
    }
}

#[test]
/// Verifies comment relocation is deterministic and remains logically irrelevant.
fn property_formatter_places_comments_deterministically() {
    let before_name = b"neu \"0.1\"\nmodule comments\nnum /* retained */ answer = 42\n";
    let before_type = b"neu \"0.1\"\nmodule comments\n/* retained */ num answer = 42\n";
    let first = format_fixture(before_name, false);
    let second = format_fixture(before_type, false);
    assert_eq!(first, second);
    assert!(
        std::str::from_utf8(&first)
            .expect("formatted source must be UTF-8")
            .contains("/* retained */\nnum answer = 42")
    );
    assert!(
        compile_fixture(before_name, false).logically_equivalent(&compile_fixture(&first, false))
    );
}

#[test]
/// Verifies formatted bytes acquire a distinct source identity without changing logic.
fn property_formatted_bytes_are_not_artifact_identity() {
    let source = b"neu \"0.1\"\r\nmodule identity\r\nnum\tanswer=00042\r\n";
    let formatted = format_fixture(source, false);
    assert_ne!(source.as_slice(), formatted);
    assert_ne!(
        neutral_core::SourceContentDigest::from_bytes(source),
        neutral_core::SourceContentDigest::from_bytes(&formatted)
    );
    assert!(
        compile_fixture(source, false).logically_equivalent(&compile_fixture(&formatted, false))
    );
}
