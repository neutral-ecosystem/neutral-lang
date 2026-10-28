// SPDX-License-Identifier: Apache-2.0

//! Current complete expression-project source capture and compilation.
#![no_main]
use libfuzzer_sys::fuzz_target;
use neutral_compiler::compile_expression_project;
use neutral_core::CancellationToken;
mod expression_support;

/// Successful compilation must survive independent validation, transport and consumer inspection.
fn exercise(bytes: &[u8]) {
    if let Some(captured) = expression_support::capture(bytes) {
        let cancel = CancellationToken::new();
        if let Ok(ir) = compile_expression_project(&captured, &cancel) {
            let authority = neutral_reader::ValidatedExpressionProject::from_ir(
                ir,
                expression_support::semantic_limits(),
                &cancel,
            );
            let authority =
                authority.expect("compiled project must pass independent reader validation");
            let encoded =
                neutral_encoding::expression::encode_expression_project(&authority, &cancel)
                    .unwrap();
            let decoded = neutral_encoding::expression::decode_expression_project(
                &encoded,
                neutral_encoding::DecodeLimits::hard(),
                expression_support::semantic_limits(),
                &cancel,
            )
            .expect("compiled expression projects must independently round trip");
            assert_eq!(decoded.ir(), authority.ir());
            expression_support::inspect_public(&decoded);
            let _ = expression_support::consumer_support::consume(&decoded, bytes);
        }
    }
}

fuzz_target!(|bytes: &[u8]| {
    exercise(bytes);
    exercise(&expression_support::mutated_source(bytes));
});
