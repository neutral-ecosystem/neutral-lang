// SPDX-License-Identifier: Apache-2.0

//! Reader-only public composition inspection without compiler construction.

use neutral_core::{CancellationToken, StructuralLimits, VocabularyContentDigest};
use neutral_ir::{
    composition::{ClosedValue as V, CompositionBody, ValueOriginKind as K},
    project_interface::ProjectPublicType as T,
};
use neutral_reader::composition::{CompositionCatalogue, CompositionLookupError as E};
use neutral_reader::composition::{
    CompositionInspectionLimits, CompositionReferenceError as R, ReferenceTypeSegment as P,
};
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
    open(bytes.as_bytes())
}

/// Opens runtime-owned captured bytes; construction verifies the complete catalogue first.
fn open(bytes: &[u8]) -> CompositionCatalogue {
    let lock = VocabularyLock::new(
        IDENTITY,
        REVISION,
        composition::ENCODING_VERSION,
        composition::SCHEMA_VERSION,
        VocabularyContentDigest::from_bytes(bytes),
        vec![composition::REQUIRED_FEATURE.to_owned()],
    )
    .unwrap();
    let limits = CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
        StructuralLimits::new(65_536, 64).unwrap(),
    ));
    CompositionCatalogue::new(
        validate_composition_closure(
            &[CapturedCompositionBundle { bytes, lock: &lock }],
            &[(IDENTITY, REVISION)],
            limits,
            &CancellationToken::new(),
        )
        .unwrap(),
    )
    .unwrap()
}

/// Creates a public reader over the literal reference-graph fixture.
fn reference_catalogue() -> CompositionCatalogue {
    open(include_bytes!("composition-fixtures/references.json"))
}

/// Supplies bounded value policy independent of the catalogue's validation counters.
fn value_limits() -> CompositionLimits {
    CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
        StructuralLimits::new(65_536, 64).unwrap(),
    ))
}

/// Reader materialization exposes safe origins, not fabricated source spans or private roots.
#[test]
fn reader_composition_materializes_public_closed_values_independently() {
    let catalogue = reference_catalogue();
    let supplied = V::Variant {
        tag: "text".into(),
        payload: Box::new(V::String("done".into())),
    };
    let value = catalogue
        .materialize(
            (IDENTITY, REVISION, "Outcome"),
            &supplied,
            value_limits(),
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(value.value(), &supplied);
    assert!(
        value
            .origins()
            .iter()
            .all(|origin| origin.kind == K::Supplied)
    );
    for name in ["Private", "Missing"] {
        assert_eq!(
            catalogue.materialize(
                (IDENTITY, REVISION, name),
                &V::Record(vec![]),
                value_limits(),
                &CancellationToken::new()
            ),
            Err(composition::CompositionError::UnknownType)
        );
    }
    assert_eq!(
        catalogue.materialize(
            (IDENTITY, REVISION, "Request"),
            &V::Record(vec![]),
            value_limits(),
            &CancellationToken::new()
        ),
        Err(composition::CompositionError::InvalidValue)
    );
}

/// Every reference wrapper and unselected variant branch preserves its exact nominal target.
#[test]
fn reader_composition_reference_types_cover_wrappers_and_all_alternatives() {
    let catalogue = reference_catalogue();
    let policy = CompositionInspectionLimits {
        visits: 1000,
        references: 10,
        depth: 10,
    };
    let references = catalogue
        .reference_types(
            (IDENTITY, REVISION, "Request"),
            policy,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(references.len(), 2);
    assert_eq!(references[0].path, [P::Field("direct")]);
    assert_eq!(
        references[1].path,
        [P::Field("others"), P::ListElement, P::NullableInner]
    );
    assert_eq!(
        references[0].target,
        &T::VocabularyNominal {
            identity: IDENTITY.into(),
            version: REVISION.into(),
            name: "Leaf".into()
        }
    );
    assert_eq!(
        references[1].target,
        &T::VocabularyNominal {
            identity: IDENTITY.into(),
            version: REVISION.into(),
            name: "Outcome".into()
        }
    );
    let alternatives = catalogue
        .reference_types(
            (IDENTITY, REVISION, "Outcome"),
            policy,
            &CancellationToken::new(),
        )
        .unwrap();
    assert_eq!(alternatives.len(), 1);
    assert_eq!(alternatives[0].path, [P::Alternative("target")]);
}

/// Independent exact/one-over traversal, reference and depth policies publish no partial enumeration.
#[test]
fn security_reader_composition_reference_traversal_is_bounded_and_atomic() {
    let catalogue = reference_catalogue();
    let exact = CompositionInspectionLimits {
        visits: 22,
        references: 2,
        depth: 3,
    };
    let owner = (IDENTITY, REVISION, "Request");
    assert!(
        catalogue
            .reference_types(owner, exact, &CancellationToken::new())
            .is_ok()
    );
    for short in [
        CompositionInspectionLimits {
            visits: 21,
            ..exact
        },
        CompositionInspectionLimits {
            references: 1,
            ..exact
        },
        CompositionInspectionLimits { depth: 2, ..exact },
    ] {
        assert_eq!(
            catalogue.reference_types(owner, short, &CancellationToken::new()),
            Err(R::Limit)
        );
    }
    for zero in [
        CompositionInspectionLimits { visits: 0, ..exact },
        CompositionInspectionLimits {
            references: 0,
            ..exact
        },
        CompositionInspectionLimits { depth: 0, ..exact },
    ] {
        assert_eq!(
            catalogue.reference_types(owner, zero, &CancellationToken::new()),
            Err(R::InvalidLimits)
        );
    }
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert_eq!(
        catalogue.reference_types(owner, exact, &cancelled),
        Err(R::Cancelled)
    );
    assert_eq!(
        catalogue.materialize(
            (IDENTITY, REVISION, "Leaf"),
            &V::Record(vec![]),
            value_limits(),
            &cancelled
        ),
        Err(composition::CompositionError::Cancelled)
    );
    for name in ["Private", "Missing"] {
        assert_eq!(
            catalogue.reference_types((IDENTITY, REVISION, name), exact, &CancellationToken::new()),
            Err(R::Lookup(E::UnavailableType))
        );
    }
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
