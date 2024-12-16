//! Program entrypoint

#![cfg(all(target_os = "solana", not(feature = "no-entrypoint")))]

use crate::processor::process_instruction;

solana_program::entrypoint!(process_instruction);
