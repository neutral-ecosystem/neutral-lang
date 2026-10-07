// SPDX-License-Identifier: Apache-2.0

//! Independent reader inspection of checked public binding values and reference dependencies.

use neutral_core::allocation::Shared as Arc;
use neutral_core::{CancellationToken, StructuralLimits, profile::V1_SOURCE_PROFILE};
use neutral_ir::{
    LogicalModuleIdentity, ModuleSymbolIdentity,
    composition::{
        BindingValue as V, CompositionBinding, CompositionBody, CompositionDefinition,
        CompositionField, FieldPresence, FieldRestrictions, SourceCompositionDefinition,
    },
    project_interface::ProjectPublicType as T,
};
use neutral_reader::composition::{
    CompositionBindingCatalogue, CompositionBindingLookupError as E,
};
use neutral_vocabulary::{
    VocabularyLimits,
    composition::{
        CompositionLimits, validate_composition_bindings, validate_composition_closure,
        validate_composition_scope,
    },
};

/// Creates fallible successor shared ownership for fixtures; allocation failure fails the test.
fn shared<T>(value: T) -> Arc<T> {
    Arc::try_new(value).unwrap()
}

/// Exact logical identity without compiler, source bytes or host information.
fn owner(name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(V1_SOURCE_PROFILE, "example"),
        name,
    )
}

/// Independently constructs contracts and checked bindings without a compiler dependency.
fn reader() -> CompositionBindingCatalogue {
    let cancellation = CancellationToken::new();
    let limits = CompositionLimits::from_vocabulary(VocabularyLimits::from_structural(
        StructuralLimits::new(65_536, 64).unwrap(),
    ));
    let catalogue = shared(validate_composition_closure(&[], &[], limits, &cancellation).unwrap());
    let scope = shared(
        validate_composition_scope(
            catalogue,
            vec![SourceCompositionDefinition {
                owner: owner("Node"),
                definition: CompositionDefinition {
                    name: "Node".to_owned(),
                    public: true,
                    body: CompositionBody::Record(vec![CompositionField {
                        name: "next".to_owned(),
                        ty: T::Nullable(Box::new(T::Ref(Box::new(T::Nominal(owner("Node")))))),
                        presence: FieldPresence::Required,
                        restrictions: FieldRestrictions::default(),
                        default: None,
                    }]),
                },
            }],
            limits,
            &cancellation,
        )
        .unwrap(),
    );
    let bindings = vec![
        CompositionBinding {
            owner: owner("second"),
            public: true,
            ty: T::Nominal(owner("Node")),
            value: V::Record(vec![(
                "next".to_owned(),
                Some(V::Reference(owner("first"))),
            )]),
        },
        CompositionBinding {
            owner: owner("first"),
            public: true,
            ty: T::Nominal(owner("Node")),
            value: V::Record(vec![("next".to_owned(), Some(V::Null))]),
        },
        CompositionBinding {
            owner: owner("private_secret"),
            public: false,
            ty: T::String,
            value: V::String("private_value".to_owned()),
        },
    ];
    CompositionBindingCatalogue::from_shared(shared(
        validate_composition_bindings(scope, &bindings, limits, &cancellation).unwrap(),
    ))
}

/// Public enumeration includes actual reference edges and target signatures from the same scope.
#[test]
fn composition_binding_reader_public_edges_and_contracts() {
    let reader = reader();
    assert_eq!(
        reader
            .bindings()
            .map(|binding| binding.owner.declaration_name())
            .collect::<Vec<_>>(),
        ["first", "second"]
    );
    let refs = reader.references(&owner("second")).unwrap();
    assert_eq!(refs.len(), 1);
    assert_eq!(refs[0].target, owner("first"));
    assert_eq!(
        reader.binding(&refs[0].target).unwrap().ty,
        T::Nominal(owner("Node"))
    );
    assert_eq!(reader.references(&owner("first")).unwrap(), []);
    assert_eq!(reader.types().source_types().count(), 1);
    assert_eq!(reader.origins(&owner("second")).unwrap().len(), 2);
}

/// Public lookup, facts and debug cannot reveal whether a private binding exists.
#[test]
fn composition_binding_reader_private_lookup_redaction() {
    let reader = reader();
    for name in ["private_secret", "missing"] {
        assert_eq!(reader.binding(&owner(name)), Err(E::UnavailableBinding));
        assert_eq!(reader.origins(&owner(name)), Err(E::UnavailableBinding));
        assert_eq!(reader.references(&owner(name)), Err(E::UnavailableBinding));
    }
    let debug = format!("{reader:?}");
    assert!(!debug.contains("private_secret"));
    assert!(!debug.contains("private_value"));
}
