//! Module that handles the processing of the `InterchainTransfer` ITS
//! instruction.
use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway::executable::AxelarMessagePayload;
use axelar_solana_gateway::state::incoming_message::command_id;
use event_utils::Event as _;
use interchain_token_transfer_gmp::{GMPPayload, InterchainTransfer};
use program_utils::{
    pda::BorshPda, validate_rent_key, validate_spl_associated_token_account_key,
    validate_system_account_key,
};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::AccountMeta;
use solana_program::instruction::Instruction;
use solana_program::msg;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::Sysvar;
use spl_token_2022::extension::transfer_fee::TransferFeeConfig;
use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
use spl_token_2022::state::{Account as TokenAccount, Mint};

use crate::executable::{AxelarInterchainTokenExecutablePayload, AXELAR_INTERCHAIN_TOKEN_EXECUTE};
use crate::processor::token_manager as token_manager_processor;
use crate::state::flow_limit::FlowDirection;
use crate::state::token_manager::{self, TokenManager};
use crate::{
    assert_valid_interchain_transfer_execute_pda, assert_valid_token_manager_pda, event,
    initiate_interchain_execute_pda_if_empty, seed_prefixes, FromAccountInfoSlice, Validate,
};

use super::gmp::{self, GmpAccounts};

/// Checks if an account is a valid Token account for the given mint and owner.
pub(super) fn is_valid_token_account(
    account: &AccountInfo,
    token_program: &Pubkey,
    expected_mint: &Pubkey,
) -> bool {
    // Check account owner is the token program
    if account.owner != token_program {
        return false;
    }

    // Try to unpack as TokenAccount and verify mint/owner
    let account_data = account.data.borrow();
    if let Ok(token_account) = StateWithExtensions::<TokenAccount>::unpack(&account_data) {
        return token_account.base.mint == *expected_mint;
    }

    false
}

/// Processes an incoming [`InterchainTransfer`] GMP message.
///
/// # General Info
///
/// For incoming `InterchainTransfer` messages, the behaviour of the
/// [`NativeInterchainToken`], [`MintBurn`] and [`MintBurnFrom`]
/// [`TokenManager`]s are the same: the token is minted to the destination token account.
///
/// As for [`LockUnlock`] and [`LockUnlockFee`] [`TokenManager`]s, they are
/// typically used in the home chain of the token, thus, if we're getting an
/// incoming message with these types of [`TokenManager`] , it means that tokens
/// are returning from another chain to the home chain (Solana), and thus, there
/// SHOULD be enough tokens locked in the [`TokenManager`]. It's the
/// responsibility of the user setting up the bridge to make sure correct token
/// manager types are used according to token supply, etc.
///
/// Specifically for [`LockUnlockFee`], we can only support it for mints with
/// the [`TransferFeeConfig`] extension. In this case the fee basis
/// configuration is set when the user creates the mint, we just need to
/// calculate the fee according to the fee configuration and call the correct
/// instruction to keep the fee withheld wherever the user defined they should
/// be withheld.
///
/// # Destination Address
///
/// When processing incoming token transfers, the program handles the destination address as
/// follows:
///
/// 1. **If `destination_address` is a Token Account**: Transfers funds directly to that account.
///
/// 2. **If `destination_address` is NOT a Token Account**: Derives and uses the Associated Token
///    Account (ATA) for that address.
///    
///    For security, the program verifies that the ATA's owner matches the `destination_address`:
///    - **SPL Token 2022 ATAs**: Always safe (have `ImmutableOwner` extension preventing ownership
///    changes)
///    - **SPL Token ATAs**: Can have ownership transferred, creating a security risk
///    
///    If ownership verification fails, the transaction is rejected to prevent funds being sent to
///    accounts controlled by unexpected parties./
///
/// # Errors
///
/// An error occurred when processing the message. The reason can be derived
/// from the logs.
pub(crate) fn process_inbound_transfer<'a>(
    message: Message,
    payer: &'a AccountInfo<'a>,
    message_payload_account: &'a AccountInfo<'a>,
    accounts: &'a [AccountInfo<'a>],
    payload: &InterchainTransfer,
    source_chain: String,
) -> ProgramResult {
    let parsed_accounts =
        GiveTokenAccounts::from_account_info_slice(accounts, &(payer, message_payload_account))?;
    let token_manager = TokenManager::load(parsed_accounts.token_manager_pda)?;
    assert_valid_token_manager_pda(
        parsed_accounts.token_manager_pda,
        parsed_accounts.its_root_pda.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

    let Ok(converted_amount) = payload.amount.try_into() else {
        msg!("Failed to convert amount");
        return Err(ProgramError::InvalidInstructionData);
    };

    // Check if source is already a valid token account for this mint
    let transferred_amount = give_token(&parsed_accounts, &token_manager, converted_amount)?;

    event::InterchainTransferReceived {
        command_id: command_id(&message.cc_id.chain, &message.cc_id.id),
        token_id: token_manager.token_id,
        source_chain,
        source_address: payload.source_address.to_vec(),
        destination_address: *parsed_accounts.destination.key,
        destination_token_account: *parsed_accounts.destination_ata.key,
        amount: transferred_amount,
        data_hash: if payload.data.is_empty() {
            [0; 32]
        } else {
            solana_program::keccak::hash(payload.data.as_ref()).0
        },
    }
    .emit();

    if !payload.data.is_empty() {
        let program_account = parsed_accounts.destination;
        let system_account = parsed_accounts.system_account;
        if !program_account.executable {
            return Err(ProgramError::InvalidInstructionData);
        }

        let destination_payload = AxelarMessagePayload::decode(payload.data.as_ref())?;
        let destination_accounts = destination_payload.account_meta();
        let axelar_executable_accounts =
            AxelarInterchainTokenExecutableAccounts::from_account_info_slice(
                accounts,
                &(parsed_accounts, destination_accounts.len()),
            )?;

        let axelar_transfer_execute_bump = assert_valid_interchain_transfer_execute_pda(
            axelar_executable_accounts.interchain_transfer_execute_pda,
            program_account.key,
        )?;

        let account_infos = [
            &[
                axelar_executable_accounts
                    .interchain_transfer_execute_pda
                    .clone(),
                axelar_executable_accounts.message_payload_pda.clone(),
                axelar_executable_accounts.token_program.clone(),
                axelar_executable_accounts.token_mint.clone(),
                axelar_executable_accounts.program_ata.clone(),
            ],
            axelar_executable_accounts.destination_program_accounts,
        ]
        .concat();

        let its_execute_instruction = build_axelar_interchain_token_execute(
            message,
            &axelar_executable_accounts,
            *program_account.key,
            destination_payload.account_meta(),
            payload,
            transferred_amount,
        )?;

        invoke_signed(
            &its_execute_instruction,
            &account_infos,
            &[&[
                seed_prefixes::INTERCHAIN_TRANSFER_EXECUTE_SEED,
                program_account.key.as_ref(),
                &[axelar_transfer_execute_bump],
            ]],
        )?;

        initiate_interchain_execute_pda_if_empty(
            axelar_executable_accounts.interchain_transfer_execute_pda,
            payer,
            system_account,
            program_account.key,
            axelar_transfer_execute_bump,
        )?;
    }

    Ok(())
}

fn build_axelar_interchain_token_execute(
    message: Message,
    axelar_its_executable_accounts: &AxelarInterchainTokenExecutableAccounts<'_>,
    program_id: Pubkey,
    mut program_accounts: Vec<AccountMeta>,
    payload: &InterchainTransfer,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);
    let source_address = payload.source_address.to_vec();
    let source_chain = message.cc_id.chain;
    let token = axelar_its_executable_accounts.token_mint.key.to_bytes();
    let token_id = payload.token_id.0;

    let mut accounts = vec![
        AccountMeta::new(
            *axelar_its_executable_accounts
                .interchain_transfer_execute_pda
                .key,
            true,
        ),
        AccountMeta::new_readonly(
            *axelar_its_executable_accounts.message_payload_pda.key,
            false,
        ),
        AccountMeta::new_readonly(*axelar_its_executable_accounts.token_program.key, false),
        AccountMeta::new(*axelar_its_executable_accounts.token_mint.key, false),
        AccountMeta::new(*axelar_its_executable_accounts.program_ata.key, false),
    ];
    accounts.append(&mut program_accounts);

    let executable_payload = AxelarInterchainTokenExecutablePayload {
        command_id,
        source_chain,
        source_address,
        data: Vec::new(),
        token_id,
        token,
        amount,
    };

    let mut data = AXELAR_INTERCHAIN_TOKEN_EXECUTE.to_vec();
    let bytes = borsh::to_vec(&executable_payload)?;
    data.extend_from_slice(&bytes);

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

/// Processes a regular interchain transfer initiated by a user account.
///
/// This function handles transfers where the source address should be the sender
/// (user account). It validates that the sender is a user account and not a
/// program or PDA to ensure proper source attribution in the transfer event.
pub(crate) fn process_user_interchain_transfer<'a>(
    accounts: &'a [AccountInfo<'a>],
    token_id: [u8; 32],
    destination_chain: String,
    destination_address: Vec<u8>,
    amount: u64,
    gas_value: u64,
    signing_pda_bump: u8,
    data: Option<Vec<u8>>,
) -> ProgramResult {
    // Check that the sender is a user account, not a program or PDA
    // We get the sender from the first account
    let sender = next_account_info(&mut accounts.iter())?;

    // User accounts should be owned by the System Program
    if sender.owner != &solana_program::system_program::ID {
        msg!(
            "Sender is not owned by System Program, owner: {}",
            sender.owner
        );
        return Err(ProgramError::InvalidAccountData);
    }

    if sender.executable {
        msg!("Sender is executable (program account)");
        return Err(ProgramError::InvalidAccountData);
    }

    process_outbound_transfer(
        accounts,
        token_id,
        destination_chain,
        destination_address,
        amount,
        gas_value,
        signing_pda_bump,
        data,
        *sender.key,
    )
}

/// Processes an interchain transfer initiated via Cross-Program Invocation (CPI) by a PDA.
pub(crate) fn process_cpi_interchain_transfer<'a>(
    accounts: &'a [AccountInfo<'a>],
    token_id: [u8; 32],
    destination_chain: String,
    destination_address: Vec<u8>,
    amount: u64,
    gas_value: u64,
    signing_pda_bump: u8,
    source_program_id: Option<Pubkey>,
    pda_seeds: Vec<Vec<u8>>,
) -> ProgramResult {
    let source_id = source_program_id.ok_or_else(|| {
        msg!("Source program ID is required for CPI-initiated transfers");
        ProgramError::InvalidInstructionData
    })?;

    // The sender should be a PDA owned by the source program
    let sender = next_account_info(&mut accounts.iter())?;
    if sender.owner != &source_id {
        msg!(
            "Sender account must be owned by the source program. Expected: {}, Got: {}",
            source_id,
            sender.owner
        );
        return Err(ProgramError::InvalidAccountData);
    }

    if sender.executable {
        msg!("PDA should not be executable");
        return Err(ProgramError::InvalidAccountData);
    }

    // Validate that the PDA can be derived using the provided seeds
    let seeds_refs: Vec<&[u8]> = pda_seeds.iter().map(std::vec::Vec::as_slice).collect();
    let (expected_pda, _bump) =
        solana_program::pubkey::Pubkey::find_program_address(&seeds_refs, &source_id);

    if expected_pda != *sender.key {
        msg!(
            "PDA derivation mismatch. Expected: {}, Got: {}",
            expected_pda,
            sender.key
        );
        return Err(ProgramError::InvalidAccountData);
    }

    process_outbound_transfer(
        accounts,
        token_id,
        destination_chain,
        destination_address,
        amount,
        gas_value,
        signing_pda_bump,
        None,
        source_id,
    )
}

pub(crate) fn process_outbound_transfer<'a>(
    accounts: &'a [AccountInfo<'a>],
    token_id: [u8; 32],
    destination_chain: String,
    destination_address: Vec<u8>,
    mut amount: u64,
    gas_value: u64,
    signing_pda_bump: u8,
    data: Option<Vec<u8>>,
    source_address: Pubkey,
) -> ProgramResult {
    const GMP_ACCOUNTS_IDX: usize = 6;
    let take_token_accounts = TakeTokenAccounts::from_account_info_slice(accounts, &())?;
    let (_other, outbound_message_accounts) = accounts.split_at(GMP_ACCOUNTS_IDX);
    let gmp_accounts = GmpAccounts::from_account_info_slice(outbound_message_accounts, &())?;

    msg!("Instruction: OutboundTransfer");

    let token_manager = TokenManager::load(take_token_accounts.token_manager_pda)?;

    assert_valid_token_manager_pda(
        take_token_accounts.token_manager_pda,
        take_token_accounts.its_root_pda.key,
        &token_id,
        token_manager.bump,
    )?;

    let expected_token_manager_ata =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            take_token_accounts.token_manager_pda.key,
            take_token_accounts.token_mint.key,
            take_token_accounts.token_program.key,
        );
    if *take_token_accounts.token_manager_ata.key != expected_token_manager_ata {
        msg!("Provided token_manager_ata doesn't match expected derivation");
        return Err(ProgramError::InvalidAccountData);
    }

    if token_manager.token_address != *take_token_accounts.token_mint.key {
        msg!("Mint and token ID don't match");
        return Err(ProgramError::InvalidAccountData);
    }

    let amount_minus_fees = take_token(&take_token_accounts, &token_manager, amount)?;
    amount = amount_minus_fees;

    let transfer_event = event::InterchainTransfer {
        token_id,
        source_address,
        source_token_account: *take_token_accounts.source_ata.key,
        destination_chain,
        destination_address,
        amount,
        data_hash: if let Some(data) = &data {
            if data.is_empty() {
                [0; 32]
            } else {
                solana_program::keccak::hash(data.as_ref()).0
            }
        } else {
            [0; 32]
        },
    };
    transfer_event.emit();

    let payload = GMPPayload::InterchainTransfer(InterchainTransfer {
        selector: InterchainTransfer::MESSAGE_TYPE_ID
            .try_into()
            .map_err(|_err| ProgramError::ArithmeticOverflow)?,
        token_id: token_id.into(),
        source_address: source_address.to_bytes().into(),
        destination_address: transfer_event.destination_address.into(),
        amount: alloy_primitives::U256::from(amount),
        data: data.unwrap_or_default().into(),
    });

    gmp::process_outbound(
        take_token_accounts.payer,
        &gmp_accounts,
        &payload,
        transfer_event.destination_chain,
        gas_value,
        signing_pda_bump,
        true,
    )
}

pub(crate) fn take_token(
    accounts: &TakeTokenAccounts<'_>,
    token_manager: &TokenManager,
    amount: u64,
) -> Result<u64, ProgramError> {
    token_manager_processor::validate_token_manager_type(
        token_manager.ty,
        accounts.token_mint,
        accounts.token_manager_pda,
    )?;

    handle_take_token_transfer(accounts, token_manager, amount)
}

fn give_token(
    accounts: &GiveTokenAccounts<'_>,
    token_manager: &TokenManager,
    amount: u64,
) -> Result<u64, ProgramError> {
    token_manager_processor::validate_token_manager_type(
        token_manager.ty,
        accounts.token_mint,
        accounts.token_manager_pda,
    )?;

    let transferred_amount = handle_give_token_transfer(accounts, token_manager, amount)?;

    Ok(transferred_amount)
}

fn track_token_flow(
    accounts: &FlowTrackingAccounts<'_>,
    amount: u64,
    direction: FlowDirection,
) -> ProgramResult {
    let mut token_manager = TokenManager::load(accounts.token_manager_pda)?;

    if token_manager.flow_slot.flow_limit.is_none() {
        return Ok(());
    }

    // Reset the flow slot upon epoch change.
    let current_epoch = crate::state::flow_limit::current_flow_epoch()?;
    if token_manager.flow_slot.epoch != current_epoch {
        msg!("Flow slot reset");
        token_manager.flow_slot.flow_in = 0;
        token_manager.flow_slot.flow_out = 0;
        token_manager.flow_slot.epoch = current_epoch;
    }

    token_manager.flow_slot.add_flow(amount, direction)?;
    token_manager.store(
        accounts.payer,
        accounts.token_manager_pda,
        accounts.system_account,
    )?;

    Ok(())
}

fn handle_give_token_transfer(
    accounts: &GiveTokenAccounts<'_>,
    token_manager: &TokenManager,
    amount: u64,
) -> Result<u64, ProgramError> {
    use token_manager::Type::{
        LockUnlock, LockUnlockFee, MintBurn, MintBurnFrom, NativeInterchainToken,
    };

    track_token_flow(&accounts.into(), amount, FlowDirection::In)?;
    let token_id = token_manager.token_id;
    let token_manager_pda_bump = token_manager.bump;

    let signer_seeds = &[
        seed_prefixes::TOKEN_MANAGER_SEED,
        accounts.its_root_pda.key.as_ref(),
        &token_id,
        &[token_manager_pda_bump],
    ];
    let transferred = match token_manager.ty {
        NativeInterchainToken | MintBurn | MintBurnFrom => {
            mint_to(
                accounts.its_root_pda,
                accounts.token_program,
                accounts.token_mint,
                accounts.destination_ata,
                accounts.token_manager_pda,
                token_manager,
                amount,
            )?;
            amount
        }
        LockUnlock => {
            let decimals = get_mint_decimals(accounts.token_mint)?;
            let transfer_info =
                create_give_token_transfer_info(accounts, amount, decimals, None, signer_seeds);
            transfer_to(&transfer_info)?;

            amount
        }
        LockUnlockFee => {
            let (fee, decimals) = get_fee_and_decimals(accounts.token_mint, amount)?;
            let transfer_info = create_give_token_transfer_info(
                accounts,
                amount,
                decimals,
                Some(fee),
                signer_seeds,
            );
            transfer_with_fee_to(&transfer_info)?;
            amount
                .checked_sub(fee)
                .ok_or(ProgramError::ArithmeticOverflow)?
        }
    };

    Ok(transferred)
}

fn handle_take_token_transfer(
    accounts: &TakeTokenAccounts<'_>,
    token_manager: &TokenManager,
    amount: u64,
) -> Result<u64, ProgramError> {
    use token_manager::Type::{
        LockUnlock, LockUnlockFee, MintBurn, MintBurnFrom, NativeInterchainToken,
    };

    track_token_flow(&accounts.into(), amount, FlowDirection::Out)?;

    let transferred = match token_manager.ty {
        NativeInterchainToken | MintBurn | MintBurnFrom => {
            burn(
                accounts.payer,
                accounts.token_program,
                accounts.token_mint,
                accounts.source_ata,
                amount,
                &[],
            )?;
            amount
        }
        LockUnlock => {
            let decimals = get_mint_decimals(accounts.token_mint)?;
            let transfer_info =
                create_take_token_transfer_info(accounts, amount, decimals, None, &[]);
            transfer_to(&transfer_info)?;
            amount
        }
        LockUnlockFee => {
            let (fee, decimals) = get_fee_and_decimals(accounts.token_mint, amount)?;
            let transfer_info =
                create_take_token_transfer_info(accounts, amount, decimals, Some(fee), &[]);
            transfer_with_fee_to(&transfer_info)?;
            amount
                .checked_sub(fee)
                .ok_or(ProgramError::ArithmeticOverflow)?
        }
    };

    Ok(transferred)
}

fn get_mint_decimals(token_mint: &AccountInfo<'_>) -> Result<u8, ProgramError> {
    let mint_data = token_mint.try_borrow_data()?;
    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data)?;
    Ok(mint_state.base.decimals)
}

fn get_fee_and_decimals(
    token_mint: &AccountInfo<'_>,
    amount: u64,
) -> Result<(u64, u8), ProgramError> {
    let mint_data = token_mint.try_borrow_data()?;
    let mint_state = StateWithExtensions::<Mint>::unpack(&mint_data)?;
    let fee_config = mint_state.get_extension::<TransferFeeConfig>()?;
    let epoch = Clock::get()?.epoch;

    let fee = fee_config
        .calculate_epoch_fee(epoch, amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    Ok((fee, mint_state.base.decimals))
}

fn create_take_token_transfer_info<'a, 'b>(
    accounts: &TakeTokenAccounts<'a>,
    amount: u64,
    decimals: u8,
    fee: Option<u64>,
    signers_seeds: &'b [&[u8]],
) -> TransferInfo<'a, 'b> {
    TransferInfo {
        token_program: accounts.token_program,
        token_mint: accounts.token_mint,
        destination: accounts.token_manager_ata,
        authority: accounts.payer,
        source: accounts.source_ata,
        signers_seeds,
        amount,
        decimals,
        fee,
    }
}

fn create_give_token_transfer_info<'a, 'b>(
    accounts: &GiveTokenAccounts<'a>,
    amount: u64,
    decimals: u8,
    fee: Option<u64>,
    signers_seeds: &'b [&[u8]],
) -> TransferInfo<'a, 'b> {
    TransferInfo {
        token_program: accounts.token_program,
        token_mint: accounts.token_mint,
        destination: accounts.destination_ata,
        authority: accounts.token_manager_pda,
        source: accounts.token_manager_ata,
        signers_seeds,
        amount,
        decimals,
        fee,
    }
}

fn mint_to<'a>(
    its_root_pda: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    token_mint: &AccountInfo<'a>,
    destination: &AccountInfo<'a>,
    token_manager_pda: &AccountInfo<'a>,
    token_manager: &TokenManager,
    amount: u64,
) -> ProgramResult {
    invoke_signed(
        &spl_token_2022::instruction::mint_to(
            token_program.key,
            token_mint.key,
            destination.key,
            token_manager_pda.key,
            &[],
            amount,
        )?,
        &[
            token_mint.clone(),
            destination.clone(),
            token_manager_pda.clone(),
        ],
        &[&[
            seed_prefixes::TOKEN_MANAGER_SEED,
            its_root_pda.key.as_ref(),
            &token_manager.token_id,
            &[token_manager.bump],
        ]],
    )?;

    Ok(())
}

fn burn<'a>(
    authority: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    token_mint: &AccountInfo<'a>,
    source_account: &AccountInfo<'a>,
    amount: u64,
    signer_seeds: &[&[u8]],
) -> ProgramResult {
    invoke_signed(
        &spl_token_2022::instruction::burn(
            token_program.key,
            source_account.key,
            token_mint.key,
            authority.key,
            &[],
            amount,
        )?,
        &[
            source_account.clone(),
            token_mint.clone(),
            authority.clone(),
        ],
        &[signer_seeds],
    )?;
    Ok(())
}

struct TransferInfo<'a, 'b> {
    token_program: &'b AccountInfo<'a>,
    token_mint: &'b AccountInfo<'a>,
    destination: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    source: &'b AccountInfo<'a>,
    signers_seeds: &'b [&'b [u8]],
    amount: u64,
    decimals: u8,
    fee: Option<u64>,
}

fn transfer_to(info: &TransferInfo<'_, '_>) -> ProgramResult {
    invoke_signed(
        &spl_token_2022::instruction::transfer_checked(
            info.token_program.key,
            info.source.key,
            info.token_mint.key,
            info.destination.key,
            info.authority.key,
            &[],
            info.amount,
            info.decimals,
        )?,
        &[
            info.token_mint.clone(),
            info.source.clone(),
            info.authority.clone(),
            info.destination.clone(),
        ],
        &[info.signers_seeds],
    )?;
    Ok(())
}

fn transfer_with_fee_to(info: &TransferInfo<'_, '_>) -> ProgramResult {
    invoke_signed(
        &spl_token_2022::extension::transfer_fee::instruction::transfer_checked_with_fee(
            info.token_program.key,
            info.source.key,
            info.token_mint.key,
            info.destination.key,
            info.authority.key,
            &[],
            info.amount,
            info.decimals,
            info.fee.ok_or(ProgramError::InvalidArgument)?,
        )?,
        &[
            info.token_mint.clone(),
            info.source.clone(),
            info.authority.clone(),
            info.destination.clone(),
        ],
        &[info.signers_seeds],
    )?;
    Ok(())
}

#[derive(Debug)]
pub(crate) struct TakeTokenAccounts<'a> {
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) source_ata: &'a AccountInfo<'a>,
    pub(crate) token_mint: &'a AccountInfo<'a>,
    pub(crate) token_manager_pda: &'a AccountInfo<'a>,
    pub(crate) token_manager_ata: &'a AccountInfo<'a>,
    pub(crate) token_program: &'a AccountInfo<'a>,
    pub(crate) system_account: &'a AccountInfo<'a>,
    pub(crate) its_root_pda: &'a AccountInfo<'a>,
}

impl Validate for TakeTokenAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        validate_system_account_key(self.system_account.key)?;
        spl_token_2022::check_spl_token_program_account(self.token_program.key)?;

        if !self.payer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if self.token_mint.owner != self.token_program.key {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

impl<'a> FromAccountInfoSlice<'a> for TakeTokenAccounts<'a> {
    type Context = ();
    fn extract_accounts(
        accounts: &'a [AccountInfo<'a>],
        _context: &Self::Context,
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();

        Ok(TakeTokenAccounts {
            payer: next_account_info(accounts_iter)?,
            source_ata: next_account_info(accounts_iter)?,
            token_mint: next_account_info(accounts_iter)?,
            token_manager_pda: next_account_info(accounts_iter)?,
            token_manager_ata: next_account_info(accounts_iter)?,
            token_program: next_account_info(accounts_iter)?,
            system_account: {
                next_account_info(accounts_iter)?;
                next_account_info(accounts_iter)?;
                next_account_info(accounts_iter)?;
                next_account_info(accounts_iter)?;
                next_account_info(accounts_iter)?
            },
            its_root_pda: next_account_info(accounts_iter)?,
        })
    }
}

#[derive(Debug)]
struct GiveTokenAccounts<'a> {
    payer: &'a AccountInfo<'a>,
    system_account: &'a AccountInfo<'a>,
    its_root_pda: &'a AccountInfo<'a>,
    message_payload_pda: &'a AccountInfo<'a>,
    token_manager_pda: &'a AccountInfo<'a>,
    token_mint: &'a AccountInfo<'a>,
    token_manager_ata: &'a AccountInfo<'a>,
    token_program: &'a AccountInfo<'a>,
    ata_program: &'a AccountInfo<'a>,
    _its_roles_pda: &'a AccountInfo<'a>,
    rent_sysvar: &'a AccountInfo<'a>,
    destination: &'a AccountInfo<'a>,
    destination_ata: &'a AccountInfo<'a>,
    interchain_transfer_execute_pda: Option<&'a AccountInfo<'a>>,
}

impl Validate for GiveTokenAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        validate_system_account_key(self.system_account.key)?;
        validate_spl_associated_token_account_key(self.ata_program.key)?;
        validate_rent_key(self.rent_sysvar.key)?;
        spl_token_2022::check_spl_token_program_account(self.token_program.key)?;

        if self.token_mint.owner != self.token_program.key {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }
}

impl<'a> FromAccountInfoSlice<'a> for GiveTokenAccounts<'a> {
    type Context = (&'a AccountInfo<'a>, &'a AccountInfo<'a>);

    fn extract_accounts(
        accounts: &'a [AccountInfo<'a>],
        payer_and_payload: &Self::Context,
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let mut extracted = GiveTokenAccounts {
            payer: payer_and_payload.0,
            message_payload_pda: payer_and_payload.1,
            system_account: next_account_info(accounts_iter)?,
            its_root_pda: next_account_info(accounts_iter)?,
            token_manager_pda: next_account_info(accounts_iter)?,
            token_mint: next_account_info(accounts_iter)?,
            token_manager_ata: next_account_info(accounts_iter)?,
            token_program: next_account_info(accounts_iter)?,
            ata_program: next_account_info(accounts_iter)?,
            _its_roles_pda: next_account_info(accounts_iter)?,
            rent_sysvar: next_account_info(accounts_iter)?,
            destination: next_account_info(accounts_iter)?,
            destination_ata: next_account_info(accounts_iter)?,
            interchain_transfer_execute_pda: next_account_info(accounts_iter).ok(),
        };

        if is_valid_token_account(
            extracted.destination,
            extracted.token_program.key,
            extracted.token_mint.key,
        ) {
            extracted.destination_ata = extracted.destination;
        } else {
            crate::create_associated_token_account_idempotent(
                extracted.payer,
                extracted.token_mint,
                extracted.destination_ata,
                extracted.destination,
                extracted.system_account,
                extracted.token_program,
            )?;
        }

        Ok(extracted)
    }
}

struct AxelarInterchainTokenExecutableAccounts<'a> {
    message_payload_pda: &'a AccountInfo<'a>,
    token_program: &'a AccountInfo<'a>,
    token_mint: &'a AccountInfo<'a>,
    program_ata: &'a AccountInfo<'a>,
    destination_program_accounts: &'a [AccountInfo<'a>],
    interchain_transfer_execute_pda: &'a AccountInfo<'a>,
}

impl Validate for AxelarInterchainTokenExecutableAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        Ok(())
    }
}

impl<'a> FromAccountInfoSlice<'a> for AxelarInterchainTokenExecutableAccounts<'a> {
    type Context = (GiveTokenAccounts<'a>, usize);

    fn extract_accounts(
        accounts: &'a [AccountInfo<'a>],
        context: &Self::Context,
    ) -> Result<Self, ProgramError>
    where
        Self: Sized + Validate,
    {
        let give_token_accounts = &context.0;
        let destination_accounts_len = context.1;
        let destination_accounts_index = accounts
            .len()
            .checked_sub(destination_accounts_len)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let destination_program_accounts = accounts
            .get(destination_accounts_index..)
            .ok_or(ProgramError::NotEnoughAccountKeys)?;

        Ok(Self {
            message_payload_pda: give_token_accounts.message_payload_pda,
            token_program: give_token_accounts.token_program,
            token_mint: give_token_accounts.token_mint,
            program_ata: give_token_accounts.destination_ata,
            destination_program_accounts,
            interchain_transfer_execute_pda: give_token_accounts
                .interchain_transfer_execute_pda
                .ok_or(ProgramError::NotEnoughAccountKeys)?,
        })
    }
}

struct FlowTrackingAccounts<'a> {
    system_account: &'a AccountInfo<'a>,
    payer: &'a AccountInfo<'a>,
    token_manager_pda: &'a AccountInfo<'a>,
}

impl<'a> From<&TakeTokenAccounts<'a>> for FlowTrackingAccounts<'a> {
    fn from(value: &TakeTokenAccounts<'a>) -> Self {
        Self {
            system_account: value.system_account,
            payer: value.payer,
            token_manager_pda: value.token_manager_pda,
        }
    }
}

impl<'a> From<&GiveTokenAccounts<'a>> for FlowTrackingAccounts<'a> {
    fn from(value: &GiveTokenAccounts<'a>) -> Self {
        Self {
            system_account: value.system_account,
            payer: value.payer,
            token_manager_pda: value.token_manager_pda,
        }
    }
}
