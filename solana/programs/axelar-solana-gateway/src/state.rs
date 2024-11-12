//! Module for the Gateway program account structs.

mod approved_command;
pub mod config;
pub mod signature_verification;
pub mod signature_verification_pda;
pub mod verifier_set_tracker;

pub use approved_command::{ApprovedMessageStatus, GatewayApprovedCommand};
pub use config::GatewayConfig;
