//! Program state processor

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::pubkey::Pubkey;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program::invoke_signed,
    program_error::ProgramError,
    program_pack::Pack,
    rent::Rent,
    system_instruction, system_program,
    sysvar::Sysvar,
};

use crate::instruction::InterchainAddressTrackerInstruction;
use crate::{
    check_program_account, get_associated_chain_address_and_bump_seed_internal, state::Account,
};

/// Program state handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        let instruction = InterchainAddressTrackerInstruction::try_from_slice(input)?;

        match instruction {
            InterchainAddressTrackerInstruction::CreateRegisteredChain { chain_name } => {
                process_create_registered_chain(program_id, accounts, chain_name)
            }
        }
    }
}

fn process_create_registered_chain(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    chain_name: String,
) -> ProgramResult {
    check_program_account(program_id)?;

    let account_info_iter = &mut accounts.iter();

    let funder_info = next_account_info(account_info_iter)?;
    let associated_chain_account = next_account_info(account_info_iter)?;
    let wallet_account_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    let (associated_token_address, bump_seed) =
        get_associated_chain_address_and_bump_seed_internal(wallet_account_info.key, program_id);
    if associated_token_address != *associated_chain_account.key {
        msg!("Error: Associated address does not match seed derivation");
        return Err(ProgramError::InvalidSeeds);
    }

    if *associated_chain_account.owner != system_program::id() {
        return Err(ProgramError::IllegalOwner);
    }

    let rent = Rent::get()?;

    let associated_token_account_signer_seeds: &[&[_]] =
        &[&wallet_account_info.key.to_bytes(), &[bump_seed]];

    invoke_signed(
        &system_instruction::create_account(
            funder_info.key,
            associated_chain_account.key,
            rent.minimum_balance(Account::get_packed_len()).max(1),
            Account::get_packed_len() as u64,
            program_id,
        ),
        &[
            funder_info.clone(),
            associated_chain_account.clone(),
            system_program_info.clone(),
        ],
        &[associated_token_account_signer_seeds],
    )?;

    let mut account_data = associated_chain_account.try_borrow_mut_data()?;
    let serialized_data = Account {
        chain_name,
        owner: *wallet_account_info.key,
    }
    .try_to_vec()
    .unwrap();
    account_data[..serialized_data.len()].copy_from_slice(&serialized_data);

    Ok(())
}
