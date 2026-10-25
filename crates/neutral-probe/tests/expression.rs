// SPDX-License-Identifier: Apache-2.0

//! Reader-only successor inspection over real encoded nominal entries, without compiler linkage.

use neutral_core::{
    ByteSpan, CancellationToken, SemanticDigest, SourceContentDigest, SourceLocation,
    StructuralLimits, allocation::Shared, profile::V1_SOURCE_PROFILE,
};
use neutral_encoding::{DecodeLimits, expression::encode_expression_project, hard_project_limits};
use neutral_ir::{
    LogicalModuleIdentity, ModuleSymbolIdentity,
    composition::{
        CompositionBody,
        project::{CompositionResourceFacts, CompositionSignature},
    },
    expression::{
        ExpressionBody,
        profile::{self, ExpressionLimits, SemanticProfiles},
        project::{
            ExpressionDeclaration, ExpressionProjectIr, ExpressionResourceFacts,
            inspect_expression_project_resources,
        },
        templates::ExpansionLimits,
    },
    project::{
        ProjectModule, ProjectProvenance, ProjectResourceFacts, ProjectSource, ProjectSourceMap,
    },
    project_identity::{IdentityLimits, expression_interface},
    project_interface::{ProjectPublicEdgeKind, ProjectPublicType},
};
use neutral_probe::expression::{
    ExpressionEntryProbeRequest, ExpressionProbeError, inspect_expression_encoded,
    render_standalone_expression,
};
use neutral_reader::{
    SemanticLimits, ValidatedExpressionProject,
    composition::{CompositionScalarLimits, ProjectCompositionLimits},
    consumer::{ENTRY_REQUEST_SCHEMA, EntryRequest, EntrySelection},
    expression::{ExpressionEntryError, ExpressionViewLimits},
};
use std::{fs, process::Command};

/// Uses logical identities, never a filesystem path, for independently authored probe inputs.
fn symbol(name: &str) -> ModuleSymbolIdentity {
    ModuleSymbolIdentity::new(LogicalModuleIdentity::new(V1_SOURCE_PROFILE, "probe"), name)
}

/// Selects independent hard semantic controls with a bounded captured-catalogue policy.
fn semantic_limits() -> SemanticLimits {
    SemanticLimits {
        project: hard_project_limits(),
        composition: ProjectCompositionLimits::from_vocabulary(
            CompositionScalarLimits::from_structural(StructuralLimits::new(1_048_576, 64).unwrap()),
        ),
        expression: ExpressionLimits::hard(),
        expansion: ExpansionLimits::default(),
    }
}

/// Retains a raw empty-record contract and two explicitly external public entries.
fn raw_fixture() -> (ExpressionProjectIr, SemanticLimits) {
    let semantic = semantic_limits();
    let source = b"captured-probe-fixture";
    let digest = SourceContentDigest::from_bytes(source);
    let location = SourceLocation::new(digest, ByteSpan::new(0, 20).unwrap());
    let contract = ProjectPublicType::Nominal(symbol("Entry"));
    let ir = ExpressionProjectIr {
        schema: profile::PROJECT_IR_SCHEMA.into(),
        modules: vec![ProjectModule {
            identity: symbol("Entry").module().clone(),
            imports: vec![],
        }],
        declarations: vec![
            ExpressionDeclaration {
                identity: symbol("Entry"),
                public: true,
                signature: CompositionSignature::Definition(CompositionBody::Record(vec![])),
                body: None,
            },
            ExpressionDeclaration {
                identity: symbol("first"),
                public: true,
                signature: CompositionSignature::Binding(contract.clone()),
                body: Some(ExpressionBody::Input),
            },
            ExpressionDeclaration {
                identity: symbol("second"),
                public: true,
                signature: CompositionSignature::Binding(contract),
                body: Some(ExpressionBody::Input),
            },
        ],
        vocabularies: vec![],
        interface_digest: SemanticDigest::from_raw_bytes([0; 32]),
        sources: vec![ProjectSource {
            module: "probe".into(),
            source_id: "source:probe".into(),
            digest,
            byte_len: source.len() as u64,
        }],
        source_maps: ["Entry", "first", "second"]
            .into_iter()
            .map(|name| ProjectSourceMap {
                declaration: symbol(name),
                location,
            })
            .collect(),
        provenance: ["first", "second"]
            .into_iter()
            .map(|name| ProjectProvenance {
                from: symbol(name),
                to: symbol("Entry"),
                kind: ProjectPublicEdgeKind::Type,
                location,
            })
            .collect(),
        limits: semantic.project,
        resources: ProjectResourceFacts {
            source_units: 1,
            source_bytes: source.len() as u64,
            vocabulary_units: 0,
            vocabulary_bytes: 0,
            declarations: 3,
            import_edges: 0,
            value_nodes: 0,
        },
        vocabulary_sources: vec![],
        origins: vec![],
        composition_limits: semantic.composition.policy(),
        composition_resources: CompositionResourceFacts::from_values([0; 7]),
        semantic_profiles: SemanticProfiles::try_frozen().unwrap(),
        public_templates: vec![],
        expression_limits: semantic.expression,
        expansion_limits: semantic.expansion,
        expression_resources: ExpressionResourceFacts {
            input_declarations: 2,
            ..Default::default()
        },
        expression_attributions: vec![],
        expansion_attributions: vec![],
    };
    (ir, semantic)
}

/// Recomputes producer facts, validates reader authority and encodes the complete artifact.
fn fixture() -> (Vec<u8>, SemanticLimits) {
    let (ir, semantic) = raw_fixture();
    (encoded(ir, &semantic), semantic)
}

/// Publishes bytes only after complete authority accepts the independent fixture.
fn encoded(mut ir: ExpressionProjectIr, semantic: &SemanticLimits) -> Vec<u8> {
    let cancel = CancellationToken::new();
    ir.resources.declarations = ir.declarations.len() as u64;
    ir.composition_resources =
        inspect_expression_project_resources(&ir, ir.limits, ir.composition_limits, &cancel)
            .unwrap();
    ir.resources.value_nodes = ir.composition_resources.retained_value_nodes;
    ir.interface_digest = SemanticDigest::from_raw_bytes(
        expression_interface(
            &ir,
            IdentityLimits {
                bytes: ir.limits.artifact_bytes,
                nodes: ir.limits.nodes,
            },
            &cancel,
        )
        .unwrap()
        .identity()
        .as_bytes(),
    );
    let project =
        ValidatedExpressionProject::from_ir(Shared::try_new(ir).unwrap(), *semantic, &cancel)
            .unwrap();
    encode_expression_project(&project, &cancel).unwrap()
}

#[test]
fn system_standalone_probe_accepts_current_expression_artifact() {
    let (bytes, _) = fixture();
    let directory = neutral_test_support::TestDirectory::new().unwrap();
    let path = directory.path().join("current.nir");
    fs::write(&path, bytes).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_neutral-probe"))
        .arg("--json")
        .arg(&path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("neutral.project-identity/3"));
    assert!(stdout.contains("neutral.project-ir/3"));
    assert!(stdout.contains("probe::first"));
    assert!(!stdout.contains("source:probe"));
    let invalid = Command::new(env!("CARGO_BIN_EXE_neutral-probe"))
        .arg("--json")
        .arg("--root")
        .arg("probe::private")
        .arg(&path)
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    assert_eq!(invalid.stdout, []);
    let selected = Command::new(env!("CARGO_BIN_EXE_neutral-probe"))
        .arg("--json")
        .arg("--root")
        .arg("probe::first")
        .arg(&path)
        .output()
        .unwrap();
    assert!(selected.status.success());
    let selected = String::from_utf8(selected.stdout).unwrap();
    assert!(selected.contains("probe::first"));
    assert!(!selected.contains("probe::second"));
}

/// Unsupported complete frames reject at the process boundary without stdout publication.
fn reject_standalone_magic(bytes: &[u8]) {
    let directory = neutral_test_support::TestDirectory::new().unwrap();
    let path = directory.path().join("unsupported.nir");
    fs::write(&path, bytes).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_neutral-probe"))
        .arg("--json")
        .arg(path)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(output.stdout, []);
    assert!(String::from_utf8_lossy(&output.stderr).contains("NIR-PROJECT-CBOR/3"));
}

/// Each historical or unknown frame is a separately discovered process regression.
macro_rules! unsupported_frames {
    ($($name:ident => $bytes:expr),* $(,)?) => {$(
        #[test]
        fn $name() { reject_standalone_magic($bytes); }
    )*};
}
unsupported_frames! {
    system_probe_rejects_document_magic => b"NEUIR\r\n\x1a",
    system_probe_rejects_project_one_magic => b"NEUPR\r\n\x1a",
    system_probe_rejects_project_two_magic => b"NEUP2\r\n\x1a",
    system_probe_rejects_future_project_magic => b"NEUP4\r\n\x1a",
    system_probe_rejects_unknown_magic => b"NOPE\r\n\x1a",
}

#[test]
fn standalone_renderer_is_bounded_fallible_and_recovers_after_failure() {
    use neutral_core::allocation::testing;
    let (bytes, _) = fixture();
    let cancel = CancellationToken::new();
    let render = |cancel: &CancellationToken| render_standalone_expression(&bytes, None, cancel);
    let (accepted, count) = testing::observe(None, || render(&cancel));
    let accepted = accepted.unwrap();
    assert!(accepted.starts_with(&format!(
        "{{\n  \"encoding\": \"{}\",\n  \"identity_profile\": \"{}\",\n",
        neutral_ir::expression::profile::ENCODING,
        neutral_ir::expression::profile::IDENTITY_PROFILE,
    )));
    assert!(accepted.contains("\"public_roots\": ["));
    assert!(accepted.contains("probe::first"));
    assert!(!accepted.contains("source:probe"));
    assert_ne!(count, 0);
    for index in 0..count {
        let (failed, _) = testing::observe(Some(index), || render(&cancel));
        assert!(failed.is_err(), "reservation {index}");
        let signal = CancellationToken::new();
        let (failed, _) = testing::observe_cancellation(index, &signal, || render(&signal));
        assert!(failed.is_err(), "cancellation {index}");
    }
    let recovered = render(&cancel).unwrap();
    assert_eq!(recovered, accepted);
    for roots in [
        vec!["probe::first".to_owned(), "probe::first".to_owned()],
        vec!["probe::private".to_owned()],
    ] {
        assert_eq!(
            render_standalone_expression(&bytes, Some(&roots), &cancel).unwrap_err(),
            ExpressionProbeError::Entry(ExpressionEntryError::InvalidInput)
        );
    }
}

/// Builds an exact request whose selection remains independent from complete artifact identity.
fn request<'a>(
    profiles: &'a SemanticProfiles,
    contract: &'a ProjectPublicType,
    selection: EntrySelection<'a>,
) -> ExpressionEntryProbeRequest<'a> {
    ExpressionEntryProbeRequest {
        semantic_profiles: profiles,
        entry: EntryRequest {
            schema: ENTRY_REQUEST_SCHEMA,
            contract,
            selection,
        },
        limits: ExpressionViewLimits::default(),
    }
}

#[test]
/// Public entry roots change projection, never complete logical/interface identity or counts.
fn property_independent_probe_selects_entries_without_rewriting_complete_identity() {
    let (bytes, semantic) = fixture();
    let profiles = SemanticProfiles::try_frozen().unwrap();
    let contract = ProjectPublicType::Nominal(symbol("Entry"));
    let cancel = CancellationToken::new();
    let all = inspect_expression_encoded(
        &bytes,
        DecodeLimits::hard(),
        semantic,
        &request(&profiles, &contract, EntrySelection::Enumerate),
        &cancel,
    )
    .unwrap();
    assert_eq!((all.modules, all.declarations), (1, 3));
    assert_eq!(all.resources.values(), [0, 0, 2, 0, 0, 0, 0]);
    assert_eq!(
        all.view
            .roots()
            .map(ModuleSymbolIdentity::declaration_name)
            .collect::<Vec<_>>(),
        ["first", "second"]
    );
    let roots = [symbol("second")];
    let selected = inspect_expression_encoded(
        &bytes,
        DecodeLimits::hard(),
        semantic,
        &request(&profiles, &contract, EntrySelection::Explicit(&roots)),
        &cancel,
    )
    .unwrap();
    assert_eq!(all.logical_identity, selected.logical_identity);
    assert_eq!(all.interface_identity, selected.interface_identity);
    assert_eq!(
        selected
            .view
            .declarations()
            .map(|d| d.identity.declaration_name())
            .collect::<Vec<_>>(),
        ["Entry", "second"]
    );
    assert_eq!(selected.declarations, 3);
}

#[test]
/// Unsupported profiles and cancellation reject before attempting hostile decoding.
fn independent_probe_preflight_never_falls_back_to_another_profile() {
    let (_, semantic) = fixture();
    let mut profiles = SemanticProfiles::try_frozen().unwrap();
    profiles
        .required_features
        .push("unsupported-feature".into());
    let contract = ProjectPublicType::Nominal(symbol("Entry"));
    let request = request(&profiles, &contract, EntrySelection::Unique);
    assert_eq!(
        inspect_expression_encoded(
            &[],
            DecodeLimits::hard(),
            semantic,
            &request,
            &CancellationToken::new()
        )
        .unwrap_err(),
        ExpressionProbeError::Entry(ExpressionEntryError::Unsupported)
    );
    let cancel = CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        inspect_expression_encoded(&[], DecodeLimits::hard(), semantic, &request, &cancel)
            .unwrap_err(),
        ExpressionProbeError::Entry(ExpressionEntryError::Cancelled)
    );
}

#[test]
/// Selection ambiguity and independently exhausted summary controls fail atomically.
fn independent_probe_rejects_ambiguous_entries_and_summary_limits() {
    let (bytes, semantic) = fixture();
    let profiles = SemanticProfiles::try_frozen().unwrap();
    let contract = ProjectPublicType::Nominal(symbol("Entry"));
    let cancel = CancellationToken::new();
    let mut request = request(&profiles, &contract, EntrySelection::Unique);
    assert_eq!(
        inspect_expression_encoded(&bytes, DecodeLimits::hard(), semantic, &request, &cancel)
            .unwrap_err(),
        ExpressionProbeError::Entry(ExpressionEntryError::AmbiguousEntry)
    );
    request.entry.selection = EntrySelection::Enumerate;
    for limits in [
        ExpressionViewLimits {
            output_bytes: 0,
            ..Default::default()
        },
        ExpressionViewLimits {
            work: 0,
            ..Default::default()
        },
    ] {
        request.limits = limits;
        assert_eq!(
            inspect_expression_encoded(&bytes, DecodeLimits::hard(), semantic, &request, &cancel)
                .unwrap_err(),
            ExpressionProbeError::Entry(ExpressionEntryError::Limit)
        );
    }
    request.limits = ExpressionViewLimits::default();
    assert!(
        inspect_expression_encoded(
            &bytes[..bytes.len() - 1],
            DecodeLimits::hard(),
            semantic,
            &request,
            &cancel
        )
        .is_err()
    );
    assert!(
        inspect_expression_encoded(&bytes, DecodeLimits::hard(), semantic, &request, &cancel)
            .is_ok()
    );
}

#[test]
/// A caller may admit the public interface yet exhaust complete logical identity framing.
fn independent_probe_reports_identity_limits_without_returning_partial_summary() {
    let (mut ir, mut semantic) = raw_fixture();
    let location = ir.source_maps[0].location;
    for index in 0..8 {
        let identity = symbol(&format!("Hidden{index}"));
        ir.declarations.push(ExpressionDeclaration {
            identity: identity.clone(),
            public: false,
            signature: CompositionSignature::Definition(CompositionBody::Record(vec![])),
            body: None,
        });
        ir.source_maps.push(ProjectSourceMap {
            declaration: identity,
            location,
        });
    }
    ir.declarations.sort_by(|a, b| a.identity.cmp(&b.identity));
    ir.source_maps
        .sort_by(|a, b| a.declaration.cmp(&b.declaration));
    let bytes = encoded(ir, &semantic);
    let profiles = SemanticProfiles::try_frozen().unwrap();
    let contract = ProjectPublicType::Nominal(symbol("Entry"));
    let request = request(&profiles, &contract, EntrySelection::Enumerate);
    let cancel = CancellationToken::new();
    let mut saw_identity_limit = false;
    for nodes in 1..=256 {
        semantic.project.nodes = nodes;
        match inspect_expression_encoded(&bytes, DecodeLimits::hard(), semantic, &request, &cancel)
        {
            Err(ExpressionProbeError::Identity(neutral_reader::IdentityError::Limit)) => {
                saw_identity_limit = true;
                break;
            }
            Err(ExpressionProbeError::Decode(_)) => {}
            result => panic!("unexpected boundary result at {nodes}: {result:?}"),
        }
    }
    assert!(saw_identity_limit);
    semantic.project = hard_project_limits();
    assert!(
        inspect_expression_encoded(&bytes, DecodeLimits::hard(), semantic, &request, &cancel)
            .is_ok()
    );
}

#[test]
/// Every probe reservation fails atomically while a previous accepted view remains usable.
fn independent_probe_allocation_and_cancellation_recover_without_partial_authority() {
    use neutral_core::allocation::testing;
    let (bytes, semantic) = fixture();
    let profiles = SemanticProfiles::try_frozen().unwrap();
    let contract = ProjectPublicType::Nominal(symbol("Entry"));
    let request = request(&profiles, &contract, EntrySelection::Enumerate);
    let cancel = CancellationToken::new();
    let inspect = |cancel: &CancellationToken| {
        inspect_expression_encoded(&bytes, DecodeLimits::hard(), semantic, &request, cancel)
    };
    let (accepted, count) = testing::observe(None, || inspect(&cancel));
    let accepted = accepted.unwrap();
    assert_ne!(count, 0);
    for index in 0..count {
        let (failed, _) = testing::observe(Some(index), || inspect(&cancel));
        assert!(failed.is_err(), "reservation {index}");
        let signal = CancellationToken::new();
        let (failed, _) = testing::observe_cancellation(index, &signal, || inspect(&signal));
        assert!(failed.is_err(), "cancellation {index}");
        assert_eq!(accepted.view.roots().count(), 2);
    }
    let recovered = inspect(&cancel).unwrap();
    assert_eq!(recovered.logical_identity, accepted.logical_identity);
    assert_eq!(recovered.interface_identity, accepted.interface_identity);
}
