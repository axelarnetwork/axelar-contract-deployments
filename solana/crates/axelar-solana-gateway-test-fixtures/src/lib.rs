//! Test utilities for the Solana Gateway
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::multiple_inherent_impl)]
#![allow(clippy::wildcard_enum_match_arm)]
#![allow(clippy::unimplemented)]

pub mod base;
pub mod gas_service;
pub mod gateway;
pub mod test_signer;

pub use gateway::{SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata};
