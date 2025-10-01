//! Instruction types

use std::fmt::Debug;

use anchor_discriminators_macros::InstructionDiscriminator;
use borsh::to_vec;
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};

use crate::seed_prefixes;

/// Instructions supported by the gateway program.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, InstructionDiscriminator)]
pub enum DummyGatewayInstruction {
    /// Prints the message back to the caller
    Echo {
        /// The message that's to be approved
        message: String,
    },
    RawPDACreation {
        bump: u8,
    },
    PDACreation {
        bump: u8,
    },
}

/// Creates a echo instruction.
pub fn echo(gateway_program_id: Pubkey, message: String) -> Instruction {
    Instruction {
        program_id: gateway_program_id,
        accounts: vec![],
        data: to_vec(&DummyGatewayInstruction::Echo { message }).unwrap(),
    }
}

/// Creates a program PDA using a normal create_account system instruction (see processor).
pub fn create_raw_pda(payer: &Pubkey) -> (Instruction, (Pubkey, u8)) {
    let (key, bump) = Pubkey::find_program_address(&[seed_prefixes::A_PDA], &crate::id());
    (
        Instruction {
            program_id: crate::id(),
            accounts: vec![
                AccountMeta::new_readonly(*payer, true),
                AccountMeta::new(key, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: to_vec(&DummyGatewayInstruction::RawPDACreation { bump }).unwrap(),
        },
        (key, bump),
    )
}

/// Creates a program PDA using the enhanced version.
pub fn create_pda(payer: &Pubkey) -> (Instruction, (Pubkey, u8)) {
    let (key, bump) = Pubkey::find_program_address(&[seed_prefixes::A_PDA], &crate::id());
    (
        Instruction {
            program_id: crate::id(),
            accounts: vec![
                AccountMeta::new_readonly(*payer, true),
                AccountMeta::new(key, false),
                AccountMeta::new_readonly(system_program::id(), false),
            ],
            data: to_vec(&DummyGatewayInstruction::PDACreation { bump }).unwrap(),
        },
        (key, bump),
    )
}
