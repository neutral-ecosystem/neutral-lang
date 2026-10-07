// SPDX-License-Identifier: Apache-2.0

//! Independent public source/vocabulary type inspection, with no compiler dependency.

use neutral_core::allocation::Shared as Arc;
use neutral_core::{CancellationToken, StructuralLimits, profile::V1_SOURCE_PROFILE};
use neutral_ir::{
    LogicalModuleIdentity, ModuleSymbolIdentity,
    composition::{
        ClosedValue as V, CompositionAlternative, CompositionBody, CompositionDefinition,
        SourceCompositionDefinition,
    },
    project_interface::ProjectPublicType as T,
};
use neutral_reader::composition::{
    CompositionInspectionLimits, CompositionLookupError as E, CompositionReferenceError as R,
    CompositionTypeCatalogue, ReferenceTypeSegment as P,
};
use neutral_vocabulary::{
    VocabularyLimits,
    composition::{
        CompositionError, CompositionLimits, validate_composition_closure,
        validate_composition_scope,
    },
};

/// Creates fallible successor shared ownership for fixtures; allocation failure fails the test.
fn shared<T>(value: T) -> Arc<T> {
    Arc::try_new(value).unwrap()
}

/// Common finite independent validation policy.
fn limits() -> CompositionLimits {
    CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
        StructuralLimits::new(65_536, 64).unwrap(),
    ))
}

/// Exact source owner, assembled without a compiler, source file or host path.
fn owner(name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(V1_SOURCE_PROFILE, "example"),
        name,
    )
}

/// Opens a valid scope containing public scalar/ref variants and one private unused contract.
fn reader() -> CompositionTypeCatalogue {
    let catalogue = shared(
        validate_composition_closure(&[], &[], limits(), &CancellationToken::new()).unwrap(),
    );
    let definitions = [
        (
            "Outcome",
            true,
            vec![CompositionAlternative {
                tag: "ok".to_owned(),
                ty: T::String,
            }],
        ),
        (
            "Next",
            true,
            vec![CompositionAlternative {
                tag: "values".to_owned(),
                ty: T::List(Box::new(T::Nullable(Box::new(T::Ref(Box::new(
                    T::Nominal(owner("Outcome")),
                )))))),
            }],
        ),
        (
            "PrivateSecret",
            false,
            vec![CompositionAlternative {
                tag: "hidden".to_owned(),
                ty: T::Bool,
            }],
        ),
    ]
    .into_iter()
    .map(|(name, public, alternatives)| SourceCompositionDefinition {
        owner: owner(name),
        definition: CompositionDefinition {
            name: name.to_owned(),
            public,
            body: CompositionBody::Variant(alternatives),
        },
    })
    .collect();
    let scope =
        validate_composition_scope(catalogue, definitions, limits(), &CancellationToken::new())
            .unwrap();
    CompositionTypeCatalogue::from_shared(shared(scope))
}

/// Independent traversal policy for all unselected variant/reference wrappers.
fn inspection() -> CompositionInspectionLimits {
    CompositionInspectionLimits {
        visits: 1024,
        references: 16,
        depth: 16,
    }
}

/// Public lookup enumerates exact source owners and all alternatives, not private declarations.
#[test]
fn composition_scope_reader_public_source_contracts() {
    let reader = reader();
    assert_eq!(
        reader
            .source_types()
            .map(|source| source.owner.declaration_name())
            .collect::<Vec<_>>(),
        ["Next", "Outcome"]
    );
    assert_eq!(reader.vocabularies().vocabularies().count(), 0);
    assert_eq!(
        reader.source_type(&owner("Outcome")).unwrap(),
        reader.public_type(&T::Nominal(owner("Outcome"))).unwrap()
    );
    assert!(matches!(
        reader.source_type(&owner("Next")).unwrap().body,
        CompositionBody::Variant(_)
    ));
}

/// Private and absent roots cannot be distinguished through lookup, values, references or debug.
#[test]
fn composition_scope_reader_private_roots_are_redacted() {
    let reader = reader();
    for name in ["PrivateSecret", "Absent"] {
        let ty = T::Nominal(owner(name));
        assert_eq!(reader.public_type(&ty), Err(E::UnavailableType));
        assert_eq!(
            reader.materialize(&ty, &V::Null, limits(), &CancellationToken::new()),
            Err(CompositionError::UnknownType)
        );
        assert_eq!(
            reader.reference_types(&ty, inspection(), &CancellationToken::new()),
            Err(R::Lookup(E::UnavailableType))
        );
    }
    let debug = format!("{reader:?}");
    assert!(!debug.contains("PrivateSecret"));
    assert!(!debug.contains("hidden"));
    assert!(!debug.contains("example"));
}

/// Canonical reference paths expose the exact source target from an unselected alternative.
#[test]
fn composition_scope_reader_all_reference_wrappers() {
    let reader = reader();
    let ty = T::Nominal(owner("Next"));
    let references = reader
        .reference_types(&ty, inspection(), &CancellationToken::new())
        .unwrap();
    assert_eq!(references.len(), 1);
    assert_eq!(
        references[0].path,
        [P::Alternative("values"), P::ListElement, P::NullableInner]
    );
    assert_eq!(references[0].target, &T::Nominal(owner("Outcome")));
    assert!(reader.public_type(references[0].target).is_ok());
}

/// The reader shares both-origin value validation without source reparsing or a compiler callback.
#[test]
fn composition_scope_reader_closed_value_validation() {
    let reader = reader();
    let ty = T::Nominal(owner("Outcome"));
    let valid = V::Variant {
        tag: "ok".to_owned(),
        payload: Box::new(V::String("ready".to_owned())),
    };
    assert_eq!(
        reader
            .materialize(&ty, &valid, limits(), &CancellationToken::new())
            .unwrap()
            .value(),
        &valid
    );
    for invalid in [
        V::Null,
        V::Variant {
            tag: "unknown".to_owned(),
            payload: Box::new(V::Bool(true)),
        },
        V::Variant {
            tag: "ok".to_owned(),
            payload: Box::new(V::Bool(true)),
        },
    ] {
        assert_eq!(
            reader.materialize(&ty, &invalid, limits(), &CancellationToken::new()),
            Err(CompositionError::InvalidValue)
        );
    }
}

/// Source nominal lookup never retries as a vocabulary name or a composed root type.
#[test]
fn composition_scope_reader_no_origin_or_type_fallback() {
    let reader = reader();
    assert_eq!(reader.public_type(&T::String), Err(E::UnavailableType));
    assert_eq!(
        reader.public_type(&T::List(Box::new(T::Nominal(owner("Outcome"))))),
        Err(E::UnavailableType)
    );
    assert_eq!(
        reader.public_type(&T::VocabularyNominal {
            identity: "Fixture".to_owned(),
            version: "1.0.0".to_owned(),
            name: "Outcome".to_owned()
        }),
        Err(E::UnknownVocabulary)
    );
    let wrong_profile = ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new("unsupported", "example"),
        "Outcome",
    );
    assert_eq!(reader.source_type(&wrong_profile), Err(E::UnavailableType));
}

/// Invalid policy, cancellation and exact/one-over traversal bounds return no partial enumeration.
#[test]
fn composition_scope_reader_bounds_and_cancellation() {
    let reader = reader();
    let ty = T::Nominal(owner("Next"));
    let mut policy = inspection();
    policy.depth = 3;
    policy.references = 1;
    assert_eq!(
        reader
            .reference_types(&ty, policy, &CancellationToken::new())
            .unwrap()
            .len(),
        1
    );
    policy.depth = 2;
    assert_eq!(
        reader.reference_types(&ty, policy, &CancellationToken::new()),
        Err(R::Limit)
    );
    policy = inspection();
    policy.references = 0;
    assert_eq!(
        reader.reference_types(&ty, policy, &CancellationToken::new()),
        Err(R::InvalidLimits)
    );
    policy = inspection();
    policy.visits = 1;
    assert_eq!(
        reader.reference_types(&ty, policy, &CancellationToken::new()),
        Err(R::Limit)
    );
    let cancellation = CancellationToken::new();
    cancellation.cancel();
    assert_eq!(
        reader.reference_types(&ty, inspection(), &cancellation),
        Err(R::Cancelled)
    );
    assert_eq!(
        reader.materialize(&ty, &V::Null, limits(), &cancellation),
        Err(CompositionError::Cancelled)
    );
}
