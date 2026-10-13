// SPDX-License-Identifier: Apache-2.0

//! Independent deep-storage bound before any view reservation or cloning.

use super::*;
use neutral_ir::composition::{CompositionField, FieldPresence, FieldRestrictions};

#[test]
fn security_unused_default_preflight_is_allocation_free() {
    let body = Body::Record(vec![CompositionField {
        name: "text".into(),
        ty: T::String,
        presence: FieldPresence::Defaulted,
        restrictions: FieldRestrictions::default(),
        default: Some(V::String("a".repeat(8192))),
    }]);
    let cancel = CancellationToken::new();
    let (result, reservations) = neutral_core::allocation::testing::observe(None, || {
        Retention::new(4096, &cancel).body(&body, &mut 100_000)
    });
    assert_eq!(result, Err(E::Limit));
    assert_eq!(reservations, 0);
}
