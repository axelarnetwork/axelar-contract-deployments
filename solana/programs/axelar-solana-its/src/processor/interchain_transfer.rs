//! Module that handles the processing of the `InterchainTransfer` ITS
//! instruction.
use axelar_executable::AxelarMessagePayload;
use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway::state::incoming_message::command_id;
use interchain_token_transfer_gmp::InterchainTransfer;
use program_utils::BorshPda;
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
use spl_token_2022::state::Mint;

use super::LocalAction;
use crate::executable::{AxelarInterchainTokenExecutablePayload, AXELAR_INTERCHAIN_TOKEN_EXECUTE};
use crate::instructions::{InterchainTransferInputs, OptionalAccountsFlags};
use crate::processor::token_manager as token_manager_processor;
use crate::state::flow_limit::{self, FlowDirection, FlowSlot};
use crate::state::token_manager::{self, TokenManager};
use crate::state::InterchainTokenService;
use crate::{
    assert_valid_flow_slot_pda, assert_valid_token_manager_pda, seed_prefixes, FromAccountInfoSlice,
};

impl LocalAction for InterchainTransfer {
    fn process_local_action<'a>(
        self,
        payer: &'a AccountInfo<'a>,
        accounts: &'a [AccountInfo<'a>],
        _optional_accounts_flags: OptionalAccountsFlags,
        message: Option<Message>,
    ) -> ProgramResult {
        let message = message.ok_or(ProgramError::InvalidArgument)?;
        process_inbound_transfer(message, payer, accounts, &self)
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
    message: Message,
    payer: &'a AccountInfo<'a>,
    accounts: &'a [AccountInfo<'a>],
    payload: &InterchainTransfer,
) -> ProgramResult {
    let parsed_accounts = GiveTokenAccounts::from_account_info_slice(accounts, &payer)?;
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

    give_token(&parsed_accounts, &token_manager, converted_amount)?;

    if !payload.data.is_empty() {
        let program_account = parsed_accounts.destination_wallet;
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

        let account_infos = [
            &[
                axelar_executable_accounts.gateway_root_pda.clone(),
                axelar_executable_accounts.its_root_pda.clone(),
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
            destination_payload.payload_without_accounts().to_vec(),
            payload.token_id.0,
            converted_amount,
        )?;
        let its_root_bump =
            InterchainTokenService::load(axelar_executable_accounts.its_root_pda)?.bump;

        invoke_signed(
            &its_execute_instruction,
            &account_infos,
            &[&[
                seed_prefixes::ITS_SEED,
                axelar_executable_accounts.gateway_root_pda.key.as_ref(),
                &[its_root_bump],
            ]],
        )?;
    }

    Ok(())
}

fn build_axelar_interchain_token_execute(
    message: Message,
    axelar_its_executable_accounts: &AxelarInterchainTokenExecutableAccounts<'_>,
    program_id: Pubkey,
    mut program_accounts: Vec<AccountMeta>,
    data: Vec<u8>,
    token_id: [u8; 32],
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);
    let source_address = message.source_address;
    let source_chain = message.cc_id.chain;
    let token = axelar_its_executable_accounts.token_mint.key.to_bytes();

    let mut accounts = vec![
        AccountMeta::new_readonly(*axelar_its_executable_accounts.gateway_root_pda.key, false),
        AccountMeta::new_readonly(*axelar_its_executable_accounts.its_root_pda.key, true),
        AccountMeta::new_readonly(*axelar_its_executable_accounts.token_program.key, false),
        AccountMeta::new(*axelar_its_executable_accounts.token_mint.key, false),
        AccountMeta::new(*axelar_its_executable_accounts.program_ata.key, false),
    ];
    accounts.append(&mut program_accounts);

    let executable_payload = AxelarInterchainTokenExecutablePayload {
        command_id,
        source_chain,
        source_address,
        data,
        token_id,
        token,
        amount,
    };

    let mut data = AXELAR_INTERCHAIN_TOKEN_EXECUTE.to_vec();
    let bytes = rkyv::to_bytes::<_, 0>(&executable_payload)
        .map_err(|_err| ProgramError::InvalidInstructionData)?;
    data.extend_from_slice(&bytes);

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

pub(crate) fn process_outbound_transfer<'a>(
    mut inputs: InterchainTransferInputs,
    accounts: &'a [AccountInfo<'a>],
) -> ProgramResult {
    let take_token_accounts = TakeTokenAccounts::from_account_info_slice(accounts, &())?;
    let token_manager = TokenManager::load(take_token_accounts.token_manager_pda)?;
    assert_valid_token_manager_pda(
        take_token_accounts.token_manager_pda,
        take_token_accounts.its_root_pda.key,
        &token_manager.token_id,
        token_manager.bump,
    )?;

    let amount_minus_fees = take_token(&take_token_accounts, &token_manager, inputs.amount)?;
    inputs.amount = amount_minus_fees;
    let maybe_payload_hash = inputs.payload_hash.take();

    let destination_chain = inputs
        .destination_chain
        .take()
        .ok_or(ProgramError::InvalidInstructionData)?;
    let gas_value = inputs.gas_value;
    let payload = inputs
        .try_into()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    let (_other, outbound_message_accounts) = accounts.split_at(3);

    crate::processor::process_outbound_its_gmp_payload(
        outbound_message_accounts,
        &payload,
        destination_chain,
        gas_value.into(),
        maybe_payload_hash,
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
) -> ProgramResult {
    token_manager_processor::validate_token_manager_type(
        token_manager.ty,
        accounts.token_mint,
        accounts.token_manager_pda,
    )?;

    crate::create_associated_token_account_idempotent(
        accounts.payer,
        accounts.token_mint,
        accounts.destination_ata,
        accounts.destination_wallet,
        accounts.system_account,
        accounts.token_program,
    )?;

    handle_give_token_transfer(accounts, token_manager, amount)?;

    Ok(())
}

fn track_token_flow(
    accounts: &FlowTrackingAccounts<'_>,
    flow_limit: u64,
    amount: u64,
    direction: FlowDirection,
) -> ProgramResult {
    if flow_limit == 0 {
        return Ok(());
    }

    let current_flow_epoch = flow_limit::current_flow_epoch()?;
    if let Ok(mut flow_slot) = FlowSlot::load(accounts.flow_slot_pda) {
        assert_valid_flow_slot_pda(
            accounts.flow_slot_pda,
            accounts.token_manager_pda.key,
            current_flow_epoch,
            flow_slot.bump,
        )?;

        flow_slot.add_flow(flow_limit, amount, direction)?;
        flow_slot.store(accounts.flow_slot_pda)?;
    } else {
        let (flow_slot_pda, flow_slot_pda_bump) =
            crate::find_flow_slot_pda(accounts.token_manager_pda.key, current_flow_epoch);

        if flow_slot_pda.ne(accounts.flow_slot_pda.key) {
            msg!("Invalid flow slot PDA provided");
            return Err(ProgramError::InvalidArgument);
        }

        let flow_slot = FlowSlot::new(flow_limit, 0, amount, flow_slot_pda_bump)?;
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
    accounts: &GiveTokenAccounts<'_>,
    token_manager: &TokenManager,
    amount: u64,
) -> ProgramResult {
    use token_manager::Type::{
        LockUnlock, LockUnlockFee, MintBurn, MintBurnFrom, NativeInterchainToken,
    };

    track_token_flow(
        &accounts.into(),
        token_manager.flow_limit,
        amount,
        FlowDirection::In,
    )?;
    let token_id = token_manager.token_id;
    let token_manager_pda_bump = token_manager.bump;

    let signer_seeds = &[
        seed_prefixes::TOKEN_MANAGER_SEED,
        accounts.its_root_pda.key.as_ref(),
        &token_id,
        &[token_manager_pda_bump],
    ];
    match token_manager.ty {
        NativeInterchainToken | MintBurn | MintBurnFrom => mint_to(
            accounts.its_root_pda,
            accounts.token_program,
            accounts.token_mint,
            accounts.destination_ata,
            accounts.token_manager_pda,
            token_manager,
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
    accounts: &TakeTokenAccounts<'_>,
    token_manager: &TokenManager,
    amount: u64,
) -> Result<u64, ProgramError> {
    use token_manager::Type::{
        LockUnlock, LockUnlockFee, MintBurn, MintBurnFrom, NativeInterchainToken,
    };

    track_token_flow(
        &accounts.into(),
        token_manager.flow_limit,
        amount,
        FlowDirection::Out,
    )?;
    let token_id = token_manager.token_id;
    let token_manager_pda_bump = token_manager.bump;

    let token_manager_pda_seeds = &[
        seed_prefixes::TOKEN_MANAGER_SEED,
        accounts.its_root_pda.key.as_ref(),
        &token_id,
        &[token_manager_pda_bump],
    ];

    let signers_seeds: &[&[u8]] = if accounts.authority.key == accounts.token_manager_pda.key {
        token_manager_pda_seeds
    } else {
        &[]
    };

    let transferred = match token_manager.ty {
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
    its_root_pda: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
    token_mint: &AccountInfo<'a>,
    destination_ata: &AccountInfo<'a>,
    token_manager_pda: &AccountInfo<'a>,
    token_manager: &TokenManager,
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

pub(crate) struct TakeTokenAccounts<'a> {
    pub(crate) system_account: &'a AccountInfo<'a>,
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) authority: &'a AccountInfo<'a>,
    pub(crate) _gateway_root_pda: &'a AccountInfo<'a>,
    pub(crate) _gateway: &'a AccountInfo<'a>,
    pub(crate) its_root_pda: &'a AccountInfo<'a>,
    pub(crate) source_account: &'a AccountInfo<'a>,
    pub(crate) token_mint: &'a AccountInfo<'a>,
    pub(crate) token_manager_pda: &'a AccountInfo<'a>,
    pub(crate) token_manager_ata: &'a AccountInfo<'a>,
    pub(crate) token_program: &'a AccountInfo<'a>,
    pub(crate) flow_slot_pda: &'a AccountInfo<'a>,
}

impl<'a> FromAccountInfoSlice<'a> for TakeTokenAccounts<'a> {
    type Context = ();
    fn from_account_info_slice(
        accounts: &'a [AccountInfo<'a>],
        _context: &Self::Context,
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();

        Ok(TakeTokenAccounts {
            system_account: next_account_info(accounts_iter)?,
            payer: next_account_info(accounts_iter)?,
            authority: next_account_info(accounts_iter)?,
            _gateway_root_pda: next_account_info(accounts_iter)?,
            _gateway: next_account_info(accounts_iter)?,
            its_root_pda: next_account_info(accounts_iter)?,
            source_account: next_account_info(accounts_iter)?,
            token_mint: next_account_info(accounts_iter)?,
            token_manager_pda: next_account_info(accounts_iter)?,
            token_manager_ata: next_account_info(accounts_iter)?,
            token_program: next_account_info(accounts_iter)?,
            flow_slot_pda: next_account_info(accounts_iter)?,
        })
    }
}

struct GiveTokenAccounts<'a> {
    payer: &'a AccountInfo<'a>,
    gateway_root_pda: &'a AccountInfo<'a>,
    system_account: &'a AccountInfo<'a>,
    its_root_pda: &'a AccountInfo<'a>,
    token_manager_pda: &'a AccountInfo<'a>,
    token_mint: &'a AccountInfo<'a>,
    token_manager_ata: &'a AccountInfo<'a>,
    token_program: &'a AccountInfo<'a>,
    _ata_program: &'a AccountInfo<'a>,
    _its_roles_pda: &'a AccountInfo<'a>,
    _rent_sysvar: &'a AccountInfo<'a>,
    destination_wallet: &'a AccountInfo<'a>,
    destination_ata: &'a AccountInfo<'a>,
    flow_slot_pda: &'a AccountInfo<'a>,
}

impl<'a> FromAccountInfoSlice<'a> for GiveTokenAccounts<'a> {
    type Context = &'a AccountInfo<'a>;

    fn from_account_info_slice(
        accounts: &'a [AccountInfo<'a>],
        payer: &Self::Context,
    ) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();

        Ok(GiveTokenAccounts {
            payer,
            gateway_root_pda: next_account_info(accounts_iter)?,
            system_account: next_account_info(accounts_iter)?,
            its_root_pda: next_account_info(accounts_iter)?,
            token_manager_pda: next_account_info(accounts_iter)?,
            token_mint: next_account_info(accounts_iter)?,
            token_manager_ata: next_account_info(accounts_iter)?,
            token_program: next_account_info(accounts_iter)?,
            _ata_program: next_account_info(accounts_iter)?,
            _its_roles_pda: next_account_info(accounts_iter)?,
            _rent_sysvar: next_account_info(accounts_iter)?,
            destination_wallet: next_account_info(accounts_iter)?,
            destination_ata: next_account_info(accounts_iter)?,
            flow_slot_pda: next_account_info(accounts_iter)?,
        })
    }
}

struct AxelarInterchainTokenExecutableAccounts<'a> {
    gateway_root_pda: &'a AccountInfo<'a>,
    its_root_pda: &'a AccountInfo<'a>,
    token_program: &'a AccountInfo<'a>,
    token_mint: &'a AccountInfo<'a>,
    program_ata: &'a AccountInfo<'a>,
    destination_program_accounts: &'a [AccountInfo<'a>],
}

impl<'a> FromAccountInfoSlice<'a> for AxelarInterchainTokenExecutableAccounts<'a> {
    type Context = (GiveTokenAccounts<'a>, usize);

    fn from_account_info_slice(
        accounts: &'a [AccountInfo<'a>],
        context: &Self::Context,
    ) -> Result<Self, ProgramError>
    where
        Self: Sized,
    {
        let give_token_accounts = &context.0;
        let destination_accounts_len = context.1;
        let destination_accounts_index = accounts
            .len()
            .checked_sub(destination_accounts_len)
            .ok_or(ProgramError::ArithmeticOverflow)?;

        let destination_program_accounts = accounts
            .get(destination_accounts_index..)
            .ok_or(ProgramError::InvalidAccountData)?;

        Ok(Self {
            gateway_root_pda: give_token_accounts.gateway_root_pda,
            its_root_pda: give_token_accounts.its_root_pda,
            token_program: give_token_accounts.token_program,
            token_mint: give_token_accounts.token_mint,
            program_ata: give_token_accounts.destination_ata,
            destination_program_accounts,
        })
    }
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
