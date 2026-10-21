// SPDX-License-Identifier: Apache-2.0

//! Nested assembly must debit the same request that resolved the source.

use super::*;

/// Exhausted expansion scratch rejects root/token copies before the allocator is reached.
#[test]
fn expansion_syntax_copies_preflight_storage_and_restore_context() {
    use neutral_core::allocation::testing::observe;
    let captured = capture(
        "neu \"1.0\"\nmodule example::expressions\npublic string result = \"retained payload\"\n",
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let cancel = CancellationToken::new();
    let mut state = initial_state(&captured, &cancel).unwrap();
    for source in captured.sources() {
        parser::parse_source(source, &mut state).unwrap();
    }
    let owner = symbol("example::expressions", "result").unwrap();
    state.budget.expansion_retained = state.budget.expansion.retained_bytes;
    let (rejected, allocations) = observe(None, || {
        state.in_expansion(|state| state.budget.copy_root(&state.resolver.roots[&owner]))
    });
    assert_eq!(rejected.err().unwrap().code, ec::TPL_LIMIT);
    assert_eq!(allocations, 0);
    assert!(!state.budget.expanding);
    assert!(
        state
            .budget
            .copy_root(&state.resolver.roots[&owner])
            .is_ok()
    );
}

/// Deferred checking of a static argument still belongs to its expansion request.
#[test]
fn static_argument_kernel_obeys_independent_expansion_work() {
    let operand = "9".repeat(200);
    let body = format!(
        "template Item(static num value) -> num {{ value }}\nnum result = instantiate Item(value: {operand} * {operand})"
    );
    assert_eq!(
        bounded(
            &body,
            ExpressionLimits::hard(),
            ExpansionLimits {
                work: 20_000,
                ..ExpansionLimits::hard()
            }
        )
        .unwrap_err()
        .code,
        ec::TPL_LIMIT
    );
    assert!(compile(&body).is_ok());
}

/// A one-node expanded result may still contain an oversized string payload.
#[test]
fn expansion_output_counts_retained_payload_not_only_expression_slots() {
    let ceiling = std::mem::size_of::<Expression>() as u64;
    let payload = "x".repeat(usize::try_from(ceiling + 1).unwrap());
    let body = format!(
        "template Text() -> string {{ \"{payload}\" }}\nstring result = instantiate Text()"
    );
    assert_eq!(
        bounded(
            &body,
            ExpressionLimits::hard(),
            ExpansionLimits {
                output_bytes: ceiling,
                ..ExpansionLimits::hard()
            }
        )
        .unwrap_err()
        .code,
        ec::TPL_LIMIT
    );
    assert!(compile(&body).is_ok());
}

/// Filter kernels are expansion work even when expression work has ample headroom.
#[test]
fn matrix_filter_kernel_obeys_independent_expansion_work() {
    let operand = "9".repeat(100);
    let body = format!(
        "template Item(static num value) -> num {{ value }}\nexpand items = Item(value: item) for (item in [1, 2, 3, 4]) where {operand} * {operand} > 0"
    );
    assert_eq!(
        bounded(
            &body,
            ExpressionLimits::hard(),
            ExpansionLimits {
                work: 20_000,
                ..ExpansionLimits::hard()
            }
        )
        .unwrap_err()
        .code,
        ec::TPL_LIMIT
    );
    assert!(compile(&body).is_ok());
}

/// Dropping or resetting copy charges allows repeated whole-model assembly to escape retention.
#[test]
fn assembly_copies_debit_retention_and_reject_exhausted_storage() {
    let captured = capture(
        "neu \"1.0\"\nmodule example::expressions\npublic string result = \"retained payload\"\n",
        ExpressionLimits::hard(),
        ExpansionLimits::hard(),
    );
    let cancel = CancellationToken::new();
    let mut state = initial_state(&captured, &cancel).unwrap();
    for source in captured.sources() {
        parser::parse_source(source, &mut state).unwrap();
    }
    let roots = state.resolver.retained_roots().unwrap();
    state.resolver.signatures(&roots).unwrap();
    let owner = symbol("example::expressions", "result").unwrap();
    state.ensure_binding(&owner).unwrap();
    let before = state.budget.bytes;
    state.assemble(true, None).unwrap();
    assert!(
        state.budget.bytes > before,
        "assembly retained data without charging storage"
    );
    state.budget.bytes = state.budget.expression.retained_bytes;
    assert_eq!(state.assemble(true, None).unwrap_err().code, ec::EXP_LIMIT);
}
