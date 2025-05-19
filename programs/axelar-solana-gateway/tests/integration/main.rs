// Reason: this is the test module
#![allow(clippy::tests_outside_test_module)]
// Reason: we need to panic in tests
#![allow(clippy::unreachable)]
#![allow(clippy::panic)]
#![allow(clippy::unwrap_used)]
#![allow(clippy::indexing_slicing)]
// Reason: we can infer what went wrong without the message
#![allow(clippy::missing_assert_message)]
// No need for documenting "tested" test code
#![allow(clippy::missing_panics_doc)]

mod approve_message;
mod close_message_payload;
mod commit_message_payload;
mod initialize_config;
pub mod initialize_message_payload;
mod initialize_signature_verification;
mod rotate_signers;
mod transfer_operatorship;
mod validate_message;
mod verify_signature;
mod write_message_payload;
