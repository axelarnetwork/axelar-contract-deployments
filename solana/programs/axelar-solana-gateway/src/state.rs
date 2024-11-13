//! Module for the Gateway program account structs.

pub mod config;
pub mod incoming_message;
pub mod signature_verification;
pub mod signature_verification_pda;
pub mod verifier_set_tracker;

pub use config::GatewayConfig;
