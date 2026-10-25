// SPDX-License-Identifier: Apache-2.0
//! Coverage-guided traversal of independently decoded current expression artifacts.
#![no_main]
use libfuzzer_sys::fuzz_target;
use neutral_core::{CancellationToken, profile::LanguageProfile};
use neutral_encoding::DecodeLimits;
use neutral_ir::{
    LogicalModuleIdentity, ModuleSymbolIdentity, project_interface::ProjectPublicType,
};
use neutral_probe::expression::{ExpressionEntryProbeRequest, inspect_expression_encoded};
use neutral_reader::{
    consumer::{ENTRY_REQUEST_SCHEMA, EntryRequest, EntrySelection},
    expression::{ExpressionViewLimits, profile::SemanticProfiles},
};
mod composition_support;
fuzz_target!(|bytes: &[u8]| {
    let profiles = SemanticProfiles::try_frozen().expect("frozen profile is valid");
    let symbol = ModuleSymbolIdentity::new(
        LogicalModuleIdentity::new(LanguageProfile::V1_0.source_version(), "example"),
        "entry",
    );
    let contract = ProjectPublicType::Nominal(symbol);
    let request = ExpressionEntryProbeRequest {
        semantic_profiles: &profiles,
        entry: EntryRequest {
            schema: ENTRY_REQUEST_SCHEMA,
            contract: &contract,
            selection: EntrySelection::Enumerate,
        },
        limits: ExpressionViewLimits::default(),
    };
    for input in [bytes, &composition_support::mutated_seed(bytes)] {
        let _ = inspect_expression_encoded(
            input,
            DecodeLimits::hard(),
            composition_support::semantic_limits(),
            &request,
            &CancellationToken::new(),
        );
    }
});
