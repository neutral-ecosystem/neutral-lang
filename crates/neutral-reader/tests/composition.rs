// SPDX-License-Identifier: Apache-2.0

//! Reader-only public composition inspection without compiler construction.

use neutral_core::{CancellationToken, StructuralLimits, VocabularyContentDigest};
use neutral_ir::composition::CompositionBody;
use neutral_reader::composition::{CompositionCatalogue, CompositionLookupError as E};
use neutral_vocabulary::{
    VocabularyLimits, VocabularyLock,
    composition::{
        self, CapturedCompositionBundle, CompositionLimits, validate_composition_closure,
    },
};

/// Canonical test owner, not a source alias.
const IDENTITY: &str = "Fixture";
/// Exact semantic revision, unrelated to the workspace version.
const REVISION: &str = "1.0.0";

/// Opens exact captured public/private contracts through the independent vocabulary boundary.
fn catalogue() -> CompositionCatalogue {
    let bytes = format!(
        r#"{{
      "format": "neutral-vocabulary-bundle", "encoding_version": "{}", "schema_version": "{}",
      "identity": "{IDENTITY}", "version": "{REVISION}", "required_features": ["{}"], "dependencies": [],
      "types": [
        {{ "kind": "record", "name": "Private", "public": false, "fields": [] }},
        {{ "kind": "variant", "name": "Outcome", "public": true, "alternatives": [
          {{ "tag": "success", "type": {{ "kind": "string" }} }},
          {{ "tag": "failure", "type": {{ "kind": "num" }} }}
        ] }},
        {{ "kind": "record", "name": "Request", "public": true, "fields": [] }}
      ]
    }}"#,
        composition::ENCODING_VERSION,
        composition::SCHEMA_VERSION,
        composition::REQUIRED_FEATURE
    );
    let lock = VocabularyLock::new(
        IDENTITY,
        REVISION,
        composition::ENCODING_VERSION,
        composition::SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(bytes.as_bytes()),
        vec![composition::REQUIRED_FEATURE.to_owned()],
    )
    .unwrap();
    let limits = CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
        StructuralLimits::new(65_536, 64).unwrap(),
    ));
    CompositionCatalogue::new(
        validate_composition_closure(
            &[CapturedCompositionBundle {
                bytes: bytes.as_bytes(),
                lock: &lock,
            }],
            &[(IDENTITY, REVISION)],
            limits,
            &CancellationToken::new(),
        )
        .unwrap(),
    )
}

/// Reader traversal preserves exact profiles and complete ordered public alternatives.
#[test]
fn reader_composition_public_catalogue_preserves_contract_facts() {
    let catalogue = catalogue();
    let identities = catalogue.vocabularies().collect::<Vec<_>>();
    assert_eq!(identities.len(), 1);
    assert_eq!(identities[0].identity(), IDENTITY);
    assert_eq!(identities[0].schema_version(), composition::SCHEMA_VERSION);
    assert_eq!(
        identities[0].required_features(),
        [composition::REQUIRED_FEATURE]
    );
    assert_eq!(
        catalogue
            .public_types(IDENTITY, REVISION)
            .unwrap()
            .map(|ty| ty.name.as_str())
            .collect::<Vec<_>>(),
        ["Outcome", "Request"]
    );
    let CompositionBody::Variant(alternatives) = &catalogue
        .public_type(IDENTITY, REVISION, "Outcome")
        .unwrap()
        .body
    else {
        panic!("variant expected")
    };
    assert_eq!(
        alternatives
            .iter()
            .map(|alternative| alternative.tag.as_str())
            .collect::<Vec<_>>(),
        ["failure", "success"]
    );
    assert_eq!(catalogue.dependencies(IDENTITY, REVISION).unwrap(), []);
}

/// Missing/private types have the same failure; aliases and revision guesses never resolve.
#[test]
fn security_reader_composition_lookup_redacts_private_type_existence() {
    let catalogue = catalogue();
    assert!(!format!("{catalogue:?}").contains("Private"));
    assert_eq!(
        catalogue.public_type(IDENTITY, REVISION, "Private"),
        Err(E::UnavailableType)
    );
    assert_eq!(
        catalogue.public_type(IDENTITY, REVISION, "Missing"),
        Err(E::UnavailableType)
    );
    assert_eq!(
        catalogue.public_type("Alias", REVISION, "Outcome"),
        Err(E::UnknownVocabulary)
    );
    assert_eq!(
        catalogue.public_type(IDENTITY, "2.0.0", "Outcome"),
        Err(E::RevisionMismatch)
    );
    assert_eq!(
        catalogue.dependencies("Alias", REVISION),
        Err(E::UnknownVocabulary)
    );
    assert_eq!(
        catalogue.dependencies(IDENTITY, "2.0.0"),
        Err(E::RevisionMismatch)
    );
}

/// Immutable cloned reader handles expose identical facts during concurrent enumeration.
#[test]
fn reader_composition_catalogue_is_shareable_without_mutable_contract_access() {
    let catalogue = catalogue();
    let handles = (0..4)
        .map(|_| {
            let catalogue = catalogue.clone();
            std::thread::spawn(move || {
                catalogue
                    .public_types(IDENTITY, REVISION)
                    .unwrap()
                    .map(|ty| ty.name.clone())
                    .collect::<Vec<_>>()
            })
        })
        .collect::<Vec<_>>();
    for handle in handles {
        assert_eq!(handle.join().unwrap(), ["Outcome", "Request"]);
    }
}
