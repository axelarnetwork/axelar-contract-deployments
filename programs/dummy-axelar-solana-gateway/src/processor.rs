//! Program state processor.

use std::any::type_name;

use anchor_discriminators::Discriminator;
use anchor_discriminators_macros::account;
use borsh::{BorshDeserialize, BorshSerialize};
use core::mem::size_of;
use program_utils::pda::init_pda;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::{Pack, Sealed};
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_instruction};

use crate::instructions::DummyGatewayInstruction;
use crate::seed_prefixes;

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    #[allow(clippy::disallowed_methods)]
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = DummyGatewayInstruction::try_from_slice(input)?;
        match instruction {
            DummyGatewayInstruction::Echo { message } => {
                msg!("Echo: {}", message);
                Ok(())
            }
            DummyGatewayInstruction::RawPDACreation { bump } => {
                let accounts_iter = &mut accounts.iter();

                let payer = next_account_info(accounts_iter)?;
                let a_pda = next_account_info(accounts_iter)?;
                let system_account = next_account_info(accounts_iter)?;

                let rent = Rent::get()?;
                let ix = &system_instruction::create_account(
                    payer.key,
                    a_pda.key,
                    rent.minimum_balance(1).max(1),
                    1,
                    program_id,
                );
                invoke_signed(
                    ix,
                    &[payer.clone(), a_pda.clone(), system_account.clone()],
                    &[&[seed_prefixes::A_PDA, &[bump]]],
                )?;
                let mut account_data = a_pda.try_borrow_mut_data()?;
                account_data.fill(0); // Initialize the account data to zero
                Ok(())
            }
            DummyGatewayInstruction::PDACreation { bump } => {
                let accounts_iter = &mut accounts.iter();

                let payer = next_account_info(accounts_iter)?;
                let a_pda = next_account_info(accounts_iter)?;
                let system_account = next_account_info(accounts_iter)?;

                let data = PDASampleData { number: 1 };

                init_pda(
                    payer,
                    a_pda,
                    &crate::ID,
                    system_account,
                    data,
                    &[seed_prefixes::A_PDA, &[bump]],
                )
            }
        }
    }
}

#[account]
#[derive(Debug, Clone, PartialEq)]
struct PDASampleData {
    number: u64,
}

impl Sealed for PDASampleData {}

impl Pack for PDASampleData {
    const LEN: usize = Self::DISCRIMINATOR.len() + size_of::<u64>();

    fn pack_into_slice(&self, mut dst: &mut [u8]) {
        self.serialize(&mut dst)
            .expect("should pack data into slice");
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let mut mut_src: &[u8] = src;
        Self::deserialize(&mut mut_src).map_err(|err| {
            msg!(
                "Error: failed to deserialize account as {}: {}",
                type_name::<Self>(),
                err
            );
            ProgramError::InvalidAccountData
        })
    }
}
