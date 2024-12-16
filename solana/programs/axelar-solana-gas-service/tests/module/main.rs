#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::missing_errors_doc,
    clippy::str_to_string,
    clippy::tests_outside_test_module,
    clippy::unwrap_used,
    clippy::panic,
    unused_must_use
)]

mod collect_fees_native;
mod initialize;
mod native_add_gas;
mod pay_native_for_contract_call;
mod refund_native_gas;
