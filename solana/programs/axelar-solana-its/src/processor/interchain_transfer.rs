//! Module that handles the processing of the `InterchainTransfer` ITS
//! instruction.
use axelar_executable_old::AxelarCallableInstruction;
use axelar_message_primitives::DataPayload;
use interchain_token_transfer_gmp::InterchainTransfer;
use program_utils::{check_rkyv_initialized_pda, StorableArchive};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::Instruction;
use solana_program::msg;
use solana_program::program::{invoke, invoke_signed};
use solana_program::program_error::ProgramError;
use solana_program::sysvar::Sysvar;
use spl_token_2022::extension::transfer_fee::TransferFeeConfig;
use spl_token_2022::extension::{BaseStateWithExtensions, StateWithExtensions};
use spl_token_2022::state::Mint;

use super::LocalAction;
use crate::instructions::Bumps;
use crate::processor::token_manager as token_manager_processor;
use crate::seed_prefixes;
use crate::state::flow_limit::{self, FlowDirection, FlowSlot};
use crate::state::token_manager::{self, TokenManager};

impl LocalAction for InterchainTransfer {
    fn process_local_action<'a>(
        self,
        payer: &'a AccountInfo<'a>,
        accounts: &'a [AccountInfo<'a>],
        bumps: Bumps,
    ) -> ProgramResult {
        process_inbound_transfer(payer, accounts, &self, bumps)
    }
}

/// Processes an incoming [`InterchainTransfer`] GMP message.
///
/// # General Info
///
/// For incoming `InterchainTransfer` messages, the behaviour of the
/// [`NativeInterchainToken`], [`MintBurn`] and [`MintBurnFrom`]
/// [`TokenManager`]s are the same: the token is minted to the destination
/// wallet's associated token account.
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
/// # Errors
///
/// An error occurred when processing the message. The reason can be derived
/// from the logs.
pub fn process_inbound_transfer<'a>(
    payer: &'a AccountInfo<'a>,
    accounts: &'a [AccountInfo<'a>],
    payload: &InterchainTransfer,
    bumps: Bumps,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let _system_account = next_account_info(accounts_iter)?;
    let _its_root_pda = next_account_info(accounts_iter)?;
    let _token_manager_pda = next_account_info(accounts_iter)?;
    let _token_mint = next_account_info(accounts_iter)?;
    let _token_manager_ata = next_account_info(accounts_iter)?;
    let _token_program = next_account_info(accounts_iter)?;
    let _ata_program = next_account_info(accounts_iter)?;
    let destination_wallet = next_account_info(accounts_iter)?;
    let _destination_ata = next_account_info(accounts_iter)?;

    let Ok(converted_amount) = payload.amount.try_into() else {
        msg!("Failed to convert amount");
        return Err(ProgramError::InvalidInstructionData);
    };

    give_token(payer, accounts, converted_amount, bumps)?;

    if !payload.data.is_empty() {
        if !destination_wallet.executable {
            return Err(ProgramError::InvalidInstructionData);
        }

        let program_payload = DataPayload::decode(&payload.data)?;

        let instruction = Instruction {
            program_id: *destination_wallet.key,
            accounts: program_payload.account_meta().to_vec(),
            data: borsh::to_vec(&AxelarCallableInstruction::Native(
                program_payload.payload_without_accounts().to_vec(),
            ))?,
        };

        invoke(&instruction, accounts_iter.as_slice())?;
    }

    Ok(())
}

pub(crate) fn take_token<'a>(
    accounts: &'a [AccountInfo<'a>],
    amount: u64,
    bumps: Bumps,
) -> Result<u64, ProgramError> {
    let accounts = parse_take_token_accounts(accounts)?;

    let (token_manager_type, _, flow_limit) = get_token_manager_info(accounts.token_manager_pda)?;

    token_manager_processor::validate_token_manager_type(
        token_manager_type,
        accounts.token_mint,
        accounts.token_manager_pda,
    )?;

    handle_take_token_transfer(token_manager_type, &accounts, bumps, amount, flow_limit)
}

fn give_token<'a>(
    payer: &'a AccountInfo<'a>,
    accounts: &'a [AccountInfo<'a>],
    amount: u64,
    bumps: Bumps,
) -> ProgramResult {
    let accounts = parse_give_token_accounts(payer, accounts)?;

    let (token_manager_type, token_id, flow_limit) =
        get_token_manager_info(accounts.token_manager_pda)?;
    let (interchain_token_pda, _) = crate::create_interchain_token_pda(
        accounts.its_root_pda.key,
        &token_id,
        bumps.interchain_token_pda_bump,
    );

    token_manager_processor::validate_token_manager_type(
        token_manager_type,
        accounts.token_mint,
        accounts.token_manager_pda,
    )?;

    crate::create_associated_token_account_idempotent(
        payer,
        accounts.token_mint,
        accounts.destination_ata,
        accounts.destination_wallet,
        accounts.system_account,
        accounts.token_program,
    )?;

    handle_give_token_transfer(
        token_manager_type,
        &accounts,
        interchain_token_pda.as_ref(),
        bumps,
        amount,
        flow_limit,
    )?;

    Ok(())
}

fn parse_take_token_accounts<'a>(
    accounts: &'a [AccountInfo<'a>],
) -> Result<TakeTokenAccounts<'a>, ProgramError> {
    let accounts_iter = &mut accounts.iter();

    Ok(TakeTokenAccounts {
        system_account: next_account_info(accounts_iter)?,
        payer: next_account_info(accounts_iter)?,
        authority: next_account_info(accounts_iter)?,
        _gateway_root_pda: next_account_info(accounts_iter)?,
        _gateway: next_account_info(accounts_iter)?,
        _its_root_pda: next_account_info(accounts_iter)?,
        interchain_token_pda: next_account_info(accounts_iter)?,
        source_account: next_account_info(accounts_iter)?,
        token_mint: next_account_info(accounts_iter)?,
        token_manager_pda: next_account_info(accounts_iter)?,
        token_manager_ata: next_account_info(accounts_iter)?,
        token_program: next_account_info(accounts_iter)?,
        flow_slot_pda: next_account_info(accounts_iter)?,
    })
}

fn parse_give_token_accounts<'a>(
    payer: &'a AccountInfo<'a>,
    accounts: &'a [AccountInfo<'a>],
) -> Result<GiveTokenAccounts<'a>, ProgramError> {
    let accounts_iter = &mut accounts.iter();

    Ok(GiveTokenAccounts {
        payer,
        system_account: next_account_info(accounts_iter)?,
        its_root_pda: next_account_info(accounts_iter)?,
        token_manager_pda: next_account_info(accounts_iter)?,
        token_mint: next_account_info(accounts_iter)?,
        token_manager_ata: next_account_info(accounts_iter)?,
        token_program: next_account_info(accounts_iter)?,
        _ata_program: next_account_info(accounts_iter)?,
        destination_wallet: next_account_info(accounts_iter)?,
        destination_ata: next_account_info(accounts_iter)?,
        flow_slot_pda: next_account_info(accounts_iter)?,
    })
}

fn get_token_manager_info(
    token_manager_pda: &AccountInfo<'_>,
) -> Result<(token_manager::Type, Vec<u8>, u64), ProgramError> {
    let token_manager_pda_data = token_manager_pda.try_borrow_data()?;
    let token_manager = check_rkyv_initialized_pda::<TokenManager>(
        &crate::id(),
        token_manager_pda,
        token_manager_pda_data.as_ref(),
    )?;
    Ok((
        token_manager.ty.into(),
        token_manager.token_id.to_bytes(),
        token_manager.flow_limit,
    ))
}

fn track_token_flow(
    accounts: &FlowTrackingAccounts<'_>,
    bumps: Bumps,
    flow_limit: u64,
    amount: u64,
    direction: FlowDirection,
) -> ProgramResult {
    if flow_limit == 0 {
        return Ok(());
    }

    let current_flow_epoch = flow_limit::current_flow_epoch()?;
    let (flow_slot_key, flow_slot_pda_bump) = crate::flow_slot_pda(
        accounts.token_manager_pda.key,
        current_flow_epoch,
        bumps.flow_slot_pda_bump,
    );

    if flow_slot_key != *accounts.flow_slot_pda.key {
        return Err(ProgramError::InvalidArgument);
    }

    if let Ok(mut flow_slot) = FlowSlot::load(&crate::id(), accounts.flow_slot_pda) {
        flow_slot.add_flow(flow_limit, amount, direction)?;
        flow_slot.store(accounts.flow_slot_pda)?;
    } else {
        let flow_slot = FlowSlot::new(flow_limit, 0, amount)?;
        flow_slot.init(
            &crate::id(),
            accounts.system_account,
            accounts.payer,
            accounts.flow_slot_pda,
            &[
                seed_prefixes::FLOW_SLOT_SEED,
                accounts.token_manager_pda.key.as_ref(),
                &current_flow_epoch.to_ne_bytes(),
                &[flow_slot_pda_bump],
            ],
        )?;
    }

    Ok(())
}

fn handle_give_token_transfer(
    token_manager_type: token_manager::Type,
    accounts: &GiveTokenAccounts<'_>,
    interchain_token_pda_bytes: &[u8],
    bumps: Bumps,
    amount: u64,
    flow_limit: u64,
) -> ProgramResult {
    use token_manager::Type::{
        LockUnlock, LockUnlockFee, MintBurn, MintBurnFrom, NativeInterchainToken,
    };

    track_token_flow(
        &accounts.into(),
        bumps,
        flow_limit,
        amount,
        FlowDirection::In,
    )?;

    let signer_seeds = &[
        seed_prefixes::TOKEN_MANAGER_SEED,
        interchain_token_pda_bytes,
        &[bumps.token_manager_pda_bump],
    ];
    match token_manager_type {
        NativeInterchainToken | MintBurn | MintBurnFrom => mint_to(
            accounts.token_program,
            accounts.token_mint,
            accounts.destination_ata,
            accounts.token_manager_pda,
            interchain_token_pda_bytes,
            bumps.token_manager_pda_bump,
            amount,
        ),
        LockUnlock => {
            let decimals = get_mint_decimals(accounts.token_mint)?;
            let transfer_info =
                create_give_token_transfer_info(accounts, amount, decimals, None, signer_seeds);
            transfer_to(&transfer_info)
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
            transfer_with_fee_to(&transfer_info)
        }
    }
}

fn handle_take_token_transfer(
    token_manager_type: token_manager::Type,
    accounts: &TakeTokenAccounts<'_>,
    bumps: Bumps,
    amount: u64,
    flow_limit: u64,
) -> Result<u64, ProgramError> {
    use token_manager::Type::{
        LockUnlock, LockUnlockFee, MintBurn, MintBurnFrom, NativeInterchainToken,
    };

    track_token_flow(
        &accounts.into(),
        bumps,
        flow_limit,
        amount,
        FlowDirection::Out,
    )?;

    let token_manager_pda_seeds = &[
        seed_prefixes::TOKEN_MANAGER_SEED,
        accounts.interchain_token_pda.key.as_ref(),
        &[bumps.token_manager_pda_bump],
    ];

    let signers_seeds: &[&[u8]] = if accounts.authority.key == accounts.token_manager_pda.key {
        token_manager_pda_seeds
    } else {
        &[]
    };

    let transferred = match token_manager_type {
        NativeInterchainToken | MintBurn | MintBurnFrom => {
            burn(
                accounts.authority,
                accounts.token_program,
                accounts.token_mint,
                accounts.source_account,
                amount,
                signers_seeds,
            )?;
            amount
        }
        LockUnlock => {
            let decimals = get_mint_decimals(accounts.token_mint)?;
            let transfer_info =
                create_take_token_transfer_info(accounts, amount, decimals, None, signers_seeds);
            transfer_to(&transfer_info)?;
            amount
        }
        LockUnlockFee => {
            let (fee, decimals) = get_fee_and_decimals(accounts.token_mint, amount)?;
            let transfer_info = create_take_token_transfer_info(
                accounts,
                amount,
                decimals,
                Some(fee),
                signers_seeds,
            );
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

const fn create_take_token_transfer_info<'a, 'b>(
    accounts: &TakeTokenAccounts<'a>,
    amount: u64,
    decimals: u8,
    fee: Option<u64>,
    signers_seeds: &'b [&[u8]],
) -> TransferInfo<'a, 'b> {
    TransferInfo {
        token_program: accounts.token_program,
        token_mint: accounts.token_mint,
        destination_ata: accounts.token_manager_ata,
        authority: accounts.authority,
        source_ata: accounts.source_account,
        signers_seeds,
        amount,
        decimals,
        fee,
    }
}

const fn create_give_token_transfer_info<'a, 'b>(
    accounts: &GiveTokenAccounts<'a>,
    amount: u64,
    decimals: u8,
    fee: Option<u64>,
    signers_seeds: &'b [&[u8]],
) -> TransferInfo<'a, 'b> {
    TransferInfo {
        token_program: accounts.token_program,
        token_mint: accounts.token_mint,
        destination_ata: accounts.destination_ata,
        authority: accounts.token_manager_pda,
        source_ata: accounts.token_manager_ata,
        signers_seeds,
        amount,
        decimals,
        fee,
    }
}

fn mint_to<'a>(
    token_program: &AccountInfo<'a>,
    token_mint: &AccountInfo<'a>,
    destination_ata: &AccountInfo<'a>,
    token_manager_pda: &AccountInfo<'a>,
    interchain_token_pda_bytes: &[u8],
    token_manager_pda_bump: u8,
    amount: u64,
) -> ProgramResult {
    invoke_signed(
        &spl_token_2022::instruction::mint_to(
            token_program.key,
            token_mint.key,
            destination_ata.key,
            token_manager_pda.key,
            &[],
            amount,
        )?,
        &[
            token_mint.clone(),
            destination_ata.clone(),
            token_manager_pda.clone(),
        ],
        &[&[
            seed_prefixes::TOKEN_MANAGER_SEED,
            interchain_token_pda_bytes,
            &[token_manager_pda_bump],
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
    destination_ata: &'b AccountInfo<'a>,
    authority: &'b AccountInfo<'a>,
    source_ata: &'b AccountInfo<'a>,
    signers_seeds: &'b [&'b [u8]],
    amount: u64,
    decimals: u8,
    fee: Option<u64>,
}

fn transfer_to(info: &TransferInfo<'_, '_>) -> ProgramResult {
    invoke_signed(
        &spl_token_2022::instruction::transfer_checked(
            info.token_program.key,
            info.source_ata.key,
            info.token_mint.key,
            info.destination_ata.key,
            info.authority.key,
            &[],
            info.amount,
            info.decimals,
        )?,
        &[
            info.token_mint.clone(),
            info.source_ata.clone(),
            info.authority.clone(),
            info.destination_ata.clone(),
        ],
        &[info.signers_seeds],
    )?;
    Ok(())
}

fn transfer_with_fee_to(info: &TransferInfo<'_, '_>) -> ProgramResult {
    invoke_signed(
        &spl_token_2022::extension::transfer_fee::instruction::transfer_checked_with_fee(
            info.token_program.key,
            info.source_ata.key,
            info.token_mint.key,
            info.destination_ata.key,
            info.authority.key,
            &[],
            info.amount,
            info.decimals,
            info.fee.ok_or(ProgramError::InvalidArgument)?,
        )?,
        &[
            info.token_mint.clone(),
            info.source_ata.clone(),
            info.authority.clone(),
            info.destination_ata.clone(),
        ],
        &[info.signers_seeds],
    )?;
    Ok(())
}

struct TakeTokenAccounts<'a> {
    system_account: &'a AccountInfo<'a>,
    payer: &'a AccountInfo<'a>,
    authority: &'a AccountInfo<'a>,
    _gateway_root_pda: &'a AccountInfo<'a>,
    _gateway: &'a AccountInfo<'a>,
    _its_root_pda: &'a AccountInfo<'a>,
    interchain_token_pda: &'a AccountInfo<'a>,
    source_account: &'a AccountInfo<'a>,
    token_mint: &'a AccountInfo<'a>,
    token_manager_pda: &'a AccountInfo<'a>,
    token_manager_ata: &'a AccountInfo<'a>,
    token_program: &'a AccountInfo<'a>,
    flow_slot_pda: &'a AccountInfo<'a>,
}

struct GiveTokenAccounts<'a> {
    system_account: &'a AccountInfo<'a>,
    payer: &'a AccountInfo<'a>,
    its_root_pda: &'a AccountInfo<'a>,
    token_manager_pda: &'a AccountInfo<'a>,
    token_mint: &'a AccountInfo<'a>,
    token_manager_ata: &'a AccountInfo<'a>,
    token_program: &'a AccountInfo<'a>,
    _ata_program: &'a AccountInfo<'a>,
    destination_wallet: &'a AccountInfo<'a>,
    destination_ata: &'a AccountInfo<'a>,
    flow_slot_pda: &'a AccountInfo<'a>,
}

struct FlowTrackingAccounts<'a> {
    system_account: &'a AccountInfo<'a>,
    payer: &'a AccountInfo<'a>,
    token_manager_pda: &'a AccountInfo<'a>,
    flow_slot_pda: &'a AccountInfo<'a>,
}

impl<'a> From<&TakeTokenAccounts<'a>> for FlowTrackingAccounts<'a> {
    fn from(value: &TakeTokenAccounts<'a>) -> Self {
        Self {
            system_account: value.system_account,
            payer: value.payer,
            token_manager_pda: value.token_manager_pda,
            flow_slot_pda: value.flow_slot_pda,
        }
    }
}

impl<'a> From<&GiveTokenAccounts<'a>> for FlowTrackingAccounts<'a> {
    fn from(value: &GiveTokenAccounts<'a>) -> Self {
        Self {
            system_account: value.system_account,
            payer: value.payer,
            token_manager_pda: value.token_manager_pda,
            flow_slot_pda: value.flow_slot_pda,
        }
    }
}
