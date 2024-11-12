//! Test utilities for the Solana Gateway
#![allow(clippy::unwrap_used)]
#![allow(clippy::expect_used)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::missing_errors_doc)]

pub mod base;
pub mod gateway;
pub mod test_signer;

pub use gateway::{SolanaAxelarIntegration, SolanaAxelarIntegrationMetadata};
