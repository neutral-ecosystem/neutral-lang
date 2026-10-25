// SPDX-License-Identifier: Apache-2.0

//! Current complete expression-project source capture and compilation.
#![no_main]
use libfuzzer_sys::fuzz_target;
use neutral_compiler::compile_expression_project;
use neutral_core::CancellationToken;
mod composition_support;

fn exercise(bytes: &[u8]) {
    if let Some(captured) = composition_support::capture(bytes) {
        let cancel = CancellationToken::new();
        if let Ok(ir) = compile_expression_project(&captured, &cancel) {
            let authority = neutral_reader::ValidatedExpressionProject::from_ir(
                ir,
                composition_support::semantic_limits(),
                &cancel,
            );
            if let Ok(authority) = authority {
                let encoded =
                    neutral_encoding::expression::encode_expression_project(&authority, &cancel)
                        .unwrap();
                let decoded = neutral_encoding::expression::decode_expression_project(
                    &encoded,
                    neutral_encoding::DecodeLimits::hard(),
                    composition_support::semantic_limits(),
                    &cancel,
                )
                .expect("compiled expression projects must independently round trip");
                assert_eq!(decoded.ir(), authority.ir());
            }
        }
    }
}

fuzz_target!(|bytes: &[u8]| {
    exercise(bytes);
    let mut seeded = include_bytes!("../seeds/composition-source.neu").to_vec();
    let header = b"neu \"1.0\"\nmodule example\n".len();
    for (original, replacement) in seeded[header..].iter_mut().zip(bytes) {
        *original = *replacement;
    }
    exercise(&seeded);
});
