//! Program state processor

use axelar_executable::{validate_message, AxelarMessagePayload, PROGRAM_ACCOUNTS_START_INDEX};
use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway::state::message_payload::ImmutMessagePayload;
use axelar_solana_its::executable::{
    AxelarInterchainTokenExecutablePayload, MaybeAxelarInterchainTokenExecutablePayload,
};
use borsh::BorshDeserialize;
use mpl_token_metadata::accounts::Metadata;
use program_utils::{check_program_account, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};
use std::str::from_utf8;

use crate::assert_counter_pda_seeds;
use crate::instruction::AxelarMemoInstruction;
use crate::state::Counter;

/// Instruction processor
pub fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id, crate::check_id)?;

    if let Some(message) = axelar_executable::parse_axelar_message(input).transpose()? {
        msg!("Instruction: AxelarExecute");
        return process_message_from_axelar(program_id, accounts, &message);
    }

    if let Some(payload) = input
        .try_get_axelar_interchain_token_executable_payload(accounts)
        .transpose()?
    {
        msg!("Instruction: AxelarInterchainTokenExecute");
        return process_message_from_axelar_with_token(program_id, accounts, &payload);
    }

    msg!("Instruction: Native");
    let instruction = AxelarMemoInstruction::try_from_slice(input)?;
    process_native_ix(program_id, accounts, instruction)
}

/// Process a message submitted by the relayer which originates from the Axelar
/// network
pub fn process_message_from_axelar_with_token<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    payload: &AxelarInterchainTokenExecutablePayload,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let _gateway_root_pda = next_account_info(accounts_iter)?;
    let _its_root_pda = next_account_info(accounts_iter)?;
    let _message_payload_account = next_account_info(accounts_iter)?;
    let _token_program = next_account_info(accounts_iter)?;
    let _token_mint = next_account_info(accounts_iter)?;
    let _ata_account = next_account_info(accounts_iter)?;
    let _mpl_token_metadata_program = next_account_info(accounts_iter)?;
    let mpl_token_metadata_account = next_account_info(accounts_iter)?;
    let instruction_accounts = accounts_iter.as_slice();
    let token_metadata = Metadata::from_bytes(&mpl_token_metadata_account.try_borrow_data()?)?;

    msg!("Processing memo with tokens:");
    msg!("amount: {}", payload.amount);
    msg!("symbol: {}", token_metadata.symbol);
    msg!("name: {}", token_metadata.name);

    let instruction: AxelarMemoInstruction = borsh::from_slice(&payload.data)?;

    process_native_ix(program_id, instruction_accounts, instruction)
}

/// Process a message submitted by the relayer which originates from the Axelar
/// network
pub fn process_message_from_axelar(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    message: &Message,
) -> ProgramResult {
    validate_message(accounts, message)?;
    let (protocol_accounts, accounts) = accounts.split_at(PROGRAM_ACCOUNTS_START_INDEX);

    // Access the payload from the MessagePayload account.
    // It should be considered safe otherwise `validate_message` would have reverted.
    let message_payload_account = &protocol_accounts[1];
    let account_data = message_payload_account.try_borrow_data()?;
    let message_payload: ImmutMessagePayload<'_> = (**account_data).try_into()?;
    let axelar_payload = AxelarMessagePayload::decode(message_payload.raw_payload)?;
    let payload = axelar_payload.payload_without_accounts();

    let memo = from_utf8(payload).map_err(|err| {
        msg!("Invalid UTF-8, from byte {}", err.valid_up_to());
        ProgramError::InvalidInstructionData
    })?;

    process_memo(program_id, accounts, memo)?;

    Ok(())
}

/// Process a native instruction submitted by another program or user ON the
/// Solana network
pub fn process_native_ix(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    payload: AxelarMemoInstruction,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    match payload {
        AxelarMemoInstruction::SendToGateway {
            memo,
            destination_chain,
            destination_address,
        } => {
            msg!("Instruction: SendToGateway");
            let counter_pda = next_account_info(account_info_iter)?;
            let gateway_root_pda = next_account_info(account_info_iter)?;
            let gateway_program = next_account_info(account_info_iter)?;
            let counter_pda_account = counter_pda.check_initialized_pda::<Counter>(program_id)?;
            assert_counter_pda_seeds(&counter_pda_account, counter_pda.key, gateway_root_pda.key);
            invoke_signed(
                &axelar_solana_gateway::instructions::call_contract(
                    *gateway_program.key,
                    *gateway_root_pda.key,
                    *counter_pda.key,
                    destination_chain,
                    destination_address,
                    memo.into_bytes(),
                )?,
                &[counter_pda.clone(), gateway_root_pda.clone()],
                &[&[gateway_root_pda.key.as_ref(), &[counter_pda_account.bump]]],
            )?;
        }
        AxelarMemoInstruction::SendToGatewayOffchainMemo {
            memo_hash,
            destination_chain,
            destination_address,
        } => {
            msg!("Instruction: SendToGatewayOffchainMemo");
            let counter_pda = next_account_info(account_info_iter)?;
            let gateway_root_pda = next_account_info(account_info_iter)?;
            let gateway_program = next_account_info(account_info_iter)?;
            let counter_pda_account = counter_pda.check_initialized_pda::<Counter>(program_id)?;
            assert_counter_pda_seeds(&counter_pda_account, counter_pda.key, gateway_root_pda.key);
            invoke_signed(
                &axelar_solana_gateway::instructions::call_contract_offchain_data(
                    *gateway_program.key,
                    *gateway_root_pda.key,
                    *counter_pda.key,
                    destination_chain,
                    destination_address,
                    memo_hash,
                )?,
                &[counter_pda.clone(), gateway_root_pda.clone()],
                &[&[gateway_root_pda.key.as_ref(), &[counter_pda_account.bump]]],
            )?;
        }
        AxelarMemoInstruction::Initialize { counter_pda_bump } => {
            msg!("Instruction: Initialize");
            process_initialize_memo_program_counter(program_id, accounts, counter_pda_bump)?;
        }
        AxelarMemoInstruction::ProcessMemo { memo } => {
            msg!("Instruction: Process Memo");
            process_memo(program_id, accounts, &memo)?
        }
    }

    Ok(())
}

fn process_memo(program_id: &Pubkey, accounts: &[AccountInfo<'_>], memo: &str) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let counter_pda = next_account_info(account_info_iter)?;

    // Iterate over the rest of the provided accounts
    for account_info in account_info_iter {
        // NOTE: The accounts WILL NEVER be signers, but they MAY be writable
        msg!(
            "Provided account {:?}-{}-{}",
            account_info.key,
            account_info.is_signer,
            account_info.is_writable
        );
    }

    log_memo(memo);

    let mut counter_pda_account = counter_pda.check_initialized_pda::<Counter>(program_id)?;
    counter_pda_account.counter += 1;
    let mut data = counter_pda.try_borrow_mut_data()?;
    counter_pda_account.pack_into_slice(&mut data);

    Ok(())
}

#[inline]
fn log_memo(memo: &str) {
    // If memo is longer than 10 characters, log just the first character.
    let char_count = memo.chars().count();
    if char_count > 10 {
        msg!(
            "Memo (len {}): {:?} x {} (too big to log)",
            memo.len(),
            memo.chars().next().unwrap(),
            char_count
        );
    } else {
        msg!("Memo (len {}): {:?}", memo.len(), memo);
    }
}

/// This function is used to initialize the program.
pub fn process_initialize_memo_program_counter(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    bump: u8,
) -> Result<(), ProgramError> {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let gateway_root_pda = next_account_info(accounts_iter)?;
    let counter_pda = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;

    let counter = crate::state::Counter { counter: 0, bump };

    // Check: System Program Account
    if !system_program::check_id(system_account.key) {
        return Err(ProgramError::IncorrectProgramId);
    }
    // Check: Memo counter PDA Account is not initialized
    counter_pda.check_uninitialized_pda()?;
    // Check: counter PDA account uses the canonical bump.
    assert_counter_pda_seeds(&counter, counter_pda.key, gateway_root_pda.key);

    program_utils::init_pda(
        payer,
        counter_pda,
        program_id,
        system_account,
        counter,
        &[gateway_root_pda.key.as_ref(), &[bump]],
    )
}
