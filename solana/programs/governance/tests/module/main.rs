#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::tests_outside_test_module,
    clippy::str_to_string,
    clippy::print_stdout,
    clippy::shadow_unrelated,
    clippy::use_debug,
    clippy::as_conversions,
    clippy::cast_possible_wrap,
    clippy::arithmetic_side_effects,
    clippy::missing_assert_message,
    clippy::cast_lossless
)]

mod execute_operator_proposal;
mod execute_proposal;
mod fixtures;
mod gmp;
mod helpers;
mod initialize_config;
mod transfer_operatorship;
mod withdraw_tokens;
