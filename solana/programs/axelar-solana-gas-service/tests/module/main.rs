#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::missing_errors_doc,
    clippy::str_to_string,
    clippy::tests_outside_test_module,
    clippy::unwrap_used,
    clippy::panic,
    clippy::arithmetic_side_effects,
    unused_must_use
)]

mod initialize;

mod native {
    mod add_gas;
    mod collect_fees;
    mod pay_for_contract_call;
    mod refund_gas;
}

mod spl {
    mod add_gas;
    mod collet_fees;
    mod pay_for_contract_call;
    mod refund_gas;
}
