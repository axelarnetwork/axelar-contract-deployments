//! Fake Axelar Gateway program for the Solana blockchain
//! This is a dummy program that echoes a message back to the caller
//! It is used for testing the Solana Gateway program upgrade process
//! through governance scheduled time-lock proposals.

pub mod entrypoint;
pub mod instructions;
pub mod processor;

solana_program::declare_id!("gtwgM94UYHwBh3g7rWi1tcpkgELxHQRLPpPHsaECW57");
