//! Program state processor

use borsh::{BorshDeserialize, BorshSerialize};
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
use solana_program::{entrypoint::ProgramResult, hash::hash};

use crate::{
    check_program_account, get_associated_chain_address_and_bump_seed_internal,
    state::RegisteredChainAccount,
};
use crate::{
    get_associated_trusted_address_account_and_bump_seed_internal,
    instruction::InterchainAddressTrackerInstruction, state::RegisteredTrustedAddressAccount,
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
            InterchainAddressTrackerInstruction::SetTrustedAddress {
                chain_name,
                address,
            } => process_set_trusted_address(program_id, accounts, chain_name, address),
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
    let owner_account_info = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    let bump_seed =
        assert_associated_chain_account(owner_account_info, program_id, associated_chain_account)?;
    if *associated_chain_account.owner != system_program::id() {
        return Err(ProgramError::IllegalOwner);
    }

    let rent = Rent::get()?;

    let associated_account_signer_seeds: &[&[_]] =
        &[&owner_account_info.key.to_bytes(), &[bump_seed]];

    invoke_signed(
        &system_instruction::create_account(
            funder_info.key,
            associated_chain_account.key,
            rent.minimum_balance(RegisteredChainAccount::get_packed_len())
                .max(1),
            RegisteredChainAccount::get_packed_len() as u64,
            program_id,
        ),
        &[
            funder_info.clone(),
            associated_chain_account.clone(),
            system_program_info.clone(),
        ],
        &[associated_account_signer_seeds],
    )?;

    let mut account_data = associated_chain_account.try_borrow_mut_data()?;
    let serialized_data = RegisteredChainAccount {
        chain_name,
        owner: *owner_account_info.key,
    }
    .try_to_vec()
    .unwrap();
    account_data[..serialized_data.len()].copy_from_slice(&serialized_data);

    Ok(())
}

fn process_set_trusted_address(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    chain_name: String,
    address: String,
) -> ProgramResult {
    check_program_account(program_id)?;

    let account_info_iter = &mut accounts.iter();

    let funder_info = next_account_info(account_info_iter)?;
    let associated_chain_account = next_account_info(account_info_iter)?;
    let owner_account_info = next_account_info(account_info_iter)?;
    let associated_trusted_address_account = next_account_info(account_info_iter)?;
    let system_program_info = next_account_info(account_info_iter)?;

    assert!(funder_info.is_writable);
    assert!(funder_info.is_signer);
    assert!(owner_account_info.is_signer);
    assert!(associated_trusted_address_account.is_writable);

    let _ =
        assert_associated_chain_account(owner_account_info, program_id, associated_chain_account)?;
    if associated_chain_account.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }
    let bump_seed = assert_associated_address_account(
        associated_chain_account,
        &chain_name,
        program_id,
        associated_trusted_address_account,
    )?;
    let rent = Rent::get()?;

    let address_h = hash(chain_name.as_bytes());
    let signer_seeds: &[&[_]] = &[
        &address_h.to_bytes(),
        &associated_chain_account.key.to_bytes(),
        &[bump_seed],
    ];
    invoke_signed(
        &system_instruction::create_account(
            funder_info.key,
            associated_trusted_address_account.key,
            rent.minimum_balance(RegisteredTrustedAddressAccount::get_packed_len())
                .max(1),
            RegisteredTrustedAddressAccount::get_packed_len() as u64,
            program_id,
        ),
        &[
            funder_info.clone(),
            associated_trusted_address_account.clone(),
            system_program_info.clone(),
        ],
        &[signer_seeds],
    )?;

    let mut account_data = associated_trusted_address_account.try_borrow_mut_data()?;
    let serialized_data = RegisteredTrustedAddressAccount { address }
        .try_to_vec()
        .unwrap();
    account_data[..serialized_data.len()].copy_from_slice(&serialized_data);

    Ok(())
}

fn assert_associated_chain_account(
    owner_account_info: &AccountInfo<'_>,
    program_id: &Pubkey,
    associated_chain_account: &AccountInfo<'_>,
) -> Result<u8, ProgramError> {
    let (associated_chain_account_derived, bump_seed) =
        get_associated_chain_address_and_bump_seed_internal(owner_account_info.key, program_id);
    if associated_chain_account_derived != *associated_chain_account.key {
        msg!("Error: Associated address does not match seed derivation");
        return Err(ProgramError::InvalidSeeds);
    }

    Ok(bump_seed)
}

fn assert_associated_address_account(
    associated_chain_account: &AccountInfo<'_>,
    chain_name: &String,
    program_id: &Pubkey,
    associated_trusted_address_account: &AccountInfo<'_>,
) -> Result<u8, ProgramError> {
    let (associated_trusted_address, bump_seed) =
        get_associated_trusted_address_account_and_bump_seed_internal(
            associated_chain_account.key,
            chain_name.as_str(),
            program_id,
        );
    if associated_trusted_address != *associated_trusted_address_account.key {
        return Err(ProgramError::InvalidSeeds);
    }
    if *associated_trusted_address_account.owner != system_program::id() {
        return Err(ProgramError::IllegalOwner);
    }
    Ok(bump_seed)
}
