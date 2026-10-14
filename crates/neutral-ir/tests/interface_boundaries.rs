// SPDX-License-Identifier: Apache-2.0

//! Public-interface framing boundaries and inert exact location values.

use neutral_core::{CoreError, profile::V1_SOURCE_PROFILE};
use neutral_ir::{
    ExactNumber, LogicalModuleIdentity, LogicalValue, ModuleSymbolIdentity, ResolvedType,
    project_interface::{
        MAX_PROJECT_INTERFACE_TYPE_DEPTH, ProjectInterface, ProjectLocationValue,
        ProjectPublicExport, ProjectPublicSignature, ProjectPublicType,
    },
};

#[test]
/// Exact decoded location text remains inert and URL/path values remain distinct.
fn location_text_preserves_non_normalized_payload_and_nominal_kind() {
    let text = "../folder/./file?query=é";
    let url = ProjectLocationValue::Url(text.into());
    let path = ProjectLocationValue::Path(text.into());
    assert_eq!(url.text(), text);
    assert_eq!(path.text(), text);
    assert_ne!(url, path);
}

#[test]
/// Nullable list validation still enforces the exact type of every materialized item.
fn nullable_lists_preserve_element_contracts_instead_of_accepting_any_payload() {
    let list = ResolvedType::List(Box::new(ResolvedType::Num));
    let nullable = ResolvedType::Nullable(Box::new(list.clone()));
    assert!(nullable.is_nullable());
    assert_eq!(nullable.nullable_inner(), Some(&list));
    assert!(!list.is_nullable());
    assert_eq!(list.nullable_inner(), None);
    assert!(nullable.accepts_value(&LogicalValue::Null));
    assert!(!list.accepts_value(&LogicalValue::Null));
    let number = LogicalValue::Number(ExactNumber::from_source("1", 4096, 4096).unwrap());
    assert!(nullable.accepts_value(&LogicalValue::List(vec![number.clone()])));
    assert!(!nullable.accepts_value(&LogicalValue::List(vec![
        number,
        LogicalValue::Boolean(true)
    ])));
    assert!(!nullable.accepts_value(&LogicalValue::Boolean(false)));
}

#[test]
/// Type framing accepts the frozen depth exactly and rejects its immediate successor.
fn interface_type_depth_rejects_one_over_without_a_fingerprint() {
    let mut ty = ProjectPublicType::Num;
    let symbol = ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(V1_SOURCE_PROFILE, "example"),
        "value",
    );
    for depth in 0..=MAX_PROJECT_INTERFACE_TYPE_DEPTH + 1 {
        let result = ProjectInterface::with_computed_fingerprint(
            vec![ProjectPublicExport::new(
                symbol.clone(),
                ProjectPublicSignature::Binding(ty.clone()),
            )],
            vec![],
        );
        if depth <= MAX_PROJECT_INTERFACE_TYPE_DEPTH {
            assert!(result.is_ok(), "depth {depth}");
        } else {
            assert_eq!(result.unwrap_err(), CoreError::TranscriptLengthExceeded);
        }
        ty = ProjectPublicType::List(Box::new(ty));
    }
}
