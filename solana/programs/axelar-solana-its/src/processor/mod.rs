//! Program state processor

use alloy_primitives::U256;
use axelar_executable_old::{validate_with_gmp_metadata, PROGRAM_ACCOUNTS_START_INDEX};
use axelar_rkyv_encoding::types::GmpMetadata;
use interchain_token_transfer_gmp::{GMPPayload, SendToHub};
use itertools::Itertools;
use program_utils::{check_rkyv_initialized_pda, StorableArchive, ValidPDA};
use role_management::processor::{ensure_signer_roles, RoleManagementAccounts};
use role_management::state::{Roles, UserRoles};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::Sysvar;
use solana_program::{msg, system_program};

use self::token_manager::SetFlowLimitAccounts;
use crate::instructions::{
    self, derive_its_accounts, its_account_indices, Bumps, InterchainTokenServiceInstruction,
    OutboundInstruction,
};
use crate::state::token_manager::TokenManager;
use crate::state::InterchainTokenService;
use crate::{check_program_account, seed_prefixes};

pub mod interchain_token;
pub mod interchain_transfer;
pub mod token_manager;

const ITS_HUB_CHAIN_NAME: &str = "axelar";

pub(crate) trait LocalAction {
    fn process_local_action<'a>(
        self,
        payer: &'a AccountInfo<'a>,
        accounts: &'a [AccountInfo<'a>],
        bumps: Bumps,
    ) -> ProgramResult;
}

impl LocalAction for GMPPayload {
    fn process_local_action<'a>(
        self,
        payer: &'a AccountInfo<'a>,
        accounts: &'a [AccountInfo<'a>],
        bumps: Bumps,
    ) -> ProgramResult {
        match self {
            Self::InterchainTransfer(inner) => inner.process_local_action(payer, accounts, bumps),
            Self::DeployInterchainToken(inner) => {
                inner.process_local_action(payer, accounts, bumps)
            }
            Self::DeployTokenManager(inner) => inner.process_local_action(payer, accounts, bumps),
            Self::SendToHub(_) | Self::ReceiveFromHub(_) => {
                msg!("Unsupported local action");
                Err(ProgramError::InvalidInstructionData)
            }
        }
    }
}

/// Processes an instruction.
///
/// # Errors
///
/// A `ProgramError` containing the error that occurred is returned. Log
/// messages are also generated with more detailed information.
pub fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    check_program_account(*program_id)?;
    let instruction = match InterchainTokenServiceInstruction::from_bytes(instruction_data) {
        Ok(instruction) => instruction,
        Err(err) => {
            msg!("Failed to deserialize instruction: {:?}", err);
            return Err(ProgramError::InvalidInstructionData);
        }
    };

    match instruction {
        InterchainTokenServiceInstruction::Initialize {
            its_root_pda_bump,
            user_roles_pda_bump,
        } => {
            process_initialize(program_id, accounts, its_root_pda_bump, user_roles_pda_bump)?;
        }
        InterchainTokenServiceInstruction::ItsGmpPayload {
            abi_payload,
            gmp_metadata,
            bumps,
        } => {
            process_inbound_its_gmp_payload(accounts, gmp_metadata, &abi_payload, bumps)?;
        }
        InterchainTokenServiceInstruction::DeployInterchainToken { params, bumps } => {
            process_its_native_deploy_call(accounts, params, bumps)?;
        }
        InterchainTokenServiceInstruction::DeployTokenManager { params, bumps } => {
            process_its_native_deploy_call(accounts, params, bumps)?;
        }
        InterchainTokenServiceInstruction::InterchainTransfer { mut params, bumps } => {
            let amount_minus_fees =
                interchain_transfer::take_token(accounts, params.amount, bumps)?;
            params.amount = amount_minus_fees;

            let destination_chain = params
                .destination_chain
                .take()
                .ok_or(ProgramError::InvalidInstructionData)?;
            let gas_value = params.gas_value;
            let payload = params
                .try_into()
                .map_err(|_err| ProgramError::InvalidInstructionData)?;

            let (_other, outbound_message_accounts) = accounts.split_at(3);

            process_outbound_its_gmp_payload(
                outbound_message_accounts,
                &payload,
                destination_chain,
                gas_value.into(),
            )?;
        }
        InterchainTokenServiceInstruction::SetFlowLimit { flow_limit } => {
            let mut instruction_accounts = SetFlowLimitAccounts::try_from(accounts)?;

            ensure_signer_roles(
                &crate::id(),
                instruction_accounts.its_root_pda,
                instruction_accounts.flow_limiter,
                instruction_accounts.its_user_roles_pda,
                Roles::OPERATOR,
            )?;

            instruction_accounts.flow_limiter = instruction_accounts.its_root_pda;
            token_manager::set_flow_limit(&instruction_accounts, flow_limit)?;
        }
        InterchainTokenServiceInstruction::MintTo { amount } => {
            process_mint_to(accounts, amount)?;
        }
        InterchainTokenServiceInstruction::OperatorInstruction(operator_instruction) => {
            process_operator_instruction(accounts, operator_instruction)?;
        }
        InterchainTokenServiceInstruction::TokenManagerInstruction(token_manager_instruction) => {
            token_manager::process_instruction(accounts, token_manager_instruction)?;
        }
    }

    Ok(())
}

fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    its_root_pda_bump: u8,
    user_roles_pda_bump: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let gateway_root_pda = next_account_info(account_info_iter)?;
    let its_root_pda = next_account_info(account_info_iter)?;
    let system_account = next_account_info(account_info_iter)?;
    let operator = next_account_info(account_info_iter)?;
    let user_roles_account = next_account_info(account_info_iter)?;

    // Check: System Program Account
    if !system_program::check_id(system_account.key) {
        return Err(ProgramError::IncorrectProgramId);
    }
    // Check: PDA Account is not initialized
    its_root_pda.check_uninitialized_pda()?;

    // Check the bump seed is correct
    crate::check_initialization_bump(its_root_pda_bump, its_root_pda.key, gateway_root_pda.key)?;
    let data = InterchainTokenService::new(its_root_pda_bump);

    program_utils::init_rkyv_pda::<{ InterchainTokenService::LEN }, _>(
        payer,
        its_root_pda,
        &crate::id(),
        system_account,
        data,
        &[
            crate::seed_prefixes::ITS_SEED,
            gateway_root_pda.key.as_ref(),
            &[its_root_pda_bump],
        ],
    )?;

    let operator_user_roles = UserRoles::new(Roles::OPERATOR, user_roles_pda_bump);
    let signer_seeds = &[
        role_management::seed_prefixes::USER_ROLES_SEED,
        its_root_pda.key.as_ref(),
        operator.key.as_ref(),
        &[user_roles_pda_bump],
    ];

    operator_user_roles.init(
        program_id,
        system_account,
        payer,
        user_roles_account,
        signer_seeds,
    )?;

    Ok(())
}

fn process_inbound_its_gmp_payload<'a>(
    accounts: &'a [AccountInfo<'a>],
    gmp_metadata: GmpMetadata,
    abi_payload: &[u8],
    bumps: Bumps,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;

    let (gateway_accounts, instruction_accounts) = accounts_iter
        .as_slice()
        .split_at(PROGRAM_ACCOUNTS_START_INDEX);

    validate_with_gmp_metadata(&crate::id(), gateway_accounts, gmp_metadata, abi_payload)?;

    let _gateway_approved_message_pda = next_account_info(accounts_iter)?;
    let _signing_pda = next_account_info(accounts_iter)?;
    let gateway_root_pda = next_account_info(accounts_iter)?;
    let _gateway_program_id = next_account_info(accounts_iter)?;

    let GMPPayload::ReceiveFromHub(inner) =
        GMPPayload::decode(abi_payload).map_err(|_err| ProgramError::InvalidInstructionData)?
    else {
        msg!("Unsupported GMP payload");
        return Err(ProgramError::InvalidInstructionData);
    };

    let payload =
        GMPPayload::decode(&inner.payload).map_err(|_err| ProgramError::InvalidInstructionData)?;

    validate_its_accounts(instruction_accounts, gateway_root_pda.key, &payload, bumps)?;
    payload.process_local_action(payer, instruction_accounts, bumps)
}

fn process_its_native_deploy_call<'a, T>(
    accounts: &'a [AccountInfo<'a>],
    mut payload: T,
    bumps: Option<Bumps>,
) -> ProgramResult
where
    T: TryInto<GMPPayload> + OutboundInstruction,
{
    let (payer, other_accounts) = accounts
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    let gas_value = payload.gas_value();
    let destination_chain = payload.destination_chain();

    let payload: GMPPayload = payload
        .try_into()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    match (destination_chain, bumps) {
        (Some(chain), _) => {
            process_outbound_its_gmp_payload(other_accounts, &payload, chain, gas_value.into())?;
        }
        (None, Some(bumps)) => {
            let (_gateway_root_pda, other_accounts) = other_accounts
                .split_first()
                .ok_or(ProgramError::InvalidInstructionData)?;

            payload.process_local_action(payer, other_accounts, bumps)?;
        }
        (None, None) => {
            msg!("Missing ITS PDA bumps");
            return Err(ProgramError::InvalidInstructionData);
        }
    };

    Ok(())
}

/// Processes an outgoing [`InterchainTransfer`], [`DeployInterchainToken`] or
/// [`DeployTokenManager`].
///
/// # Errors
///
/// An error occurred when processing the message. The reason can be derived
/// from the logs.
fn process_outbound_its_gmp_payload(
    accounts: &[AccountInfo<'_>],
    payload: &GMPPayload,
    destination_chain: String,
    _gas_value: U256,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let gateway_root_pda = next_account_info(accounts_iter)?;
    let _gateway_program_id = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let its_root_pda_data = its_root_pda.try_borrow_data()?;
    let its_state = check_rkyv_initialized_pda::<InterchainTokenService>(
        &crate::id(),
        its_root_pda,
        *its_root_pda_data,
    )?;

    // TODO: Get chain's trusted address. It should be ITS Hub address.
    let destination_address = String::new();
    let hub_payload = GMPPayload::SendToHub(SendToHub {
        selector: SendToHub::MESSAGE_TYPE_ID
            .try_into()
            .map_err(|_err| ProgramError::ArithmeticOverflow)?,
        destination_chain,
        payload: payload.encode().into(),
    });

    // TODO: Call gas service to pay gas fee.

    invoke_signed(
        &gateway::instructions::call_contract(
            *gateway_root_pda.key,
            *its_root_pda.key,
            ITS_HUB_CHAIN_NAME.to_owned(),
            destination_address,
            hub_payload.encode(),
        )?,
        &[its_root_pda.clone(), gateway_root_pda.clone()],
        &[&[
            seed_prefixes::ITS_SEED,
            gateway_root_pda.key.as_ref(),
            &[its_state.bump],
        ]],
    )?;

    Ok(())
}

fn process_mint_to(accounts: &[AccountInfo<'_>], amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let mint = next_account_info(accounts_iter)?;
    let destination_account = next_account_info(accounts_iter)?;
    let interchain_token_pda = next_account_info(accounts_iter)?;
    let token_manager_pda = next_account_info(accounts_iter)?;
    let minter = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;

    let token_manager_pda_data = token_manager_pda.try_borrow_data()?;
    let token_manager = check_rkyv_initialized_pda::<TokenManager>(
        &crate::id(),
        token_manager_pda,
        token_manager_pda_data.as_ref(),
    )?;

    if token_manager.token_address.as_ref() != mint.key.as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }

    if mint.owner != token_program.key {
        return Err(ProgramError::IncorrectProgramId);
    }

    if !minter.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // TODO: Check that `minter` is really a minter.

    invoke_signed(
        &spl_token_2022::instruction::mint_to(
            token_program.key,
            mint.key,
            destination_account.key,
            token_manager_pda.key,
            &[],
            amount,
        )?,
        &[
            mint.clone(),
            destination_account.clone(),
            token_manager_pda.clone(),
            token_program.clone(),
        ],
        &[&[
            seed_prefixes::TOKEN_MANAGER_SEED,
            interchain_token_pda.key.as_ref(),
            &[token_manager.bump],
        ]],
    )?;
    Ok(())
}

fn validate_its_accounts(
    accounts: &[AccountInfo<'_>],
    gateway_root_pda: &Pubkey,
    payload: &GMPPayload,
    bumps: Bumps,
) -> ProgramResult {
    // In this case we cannot derive the mint account, so we just use what we got
    // and check later against the mint within the `TokenManager` PDA.
    let maybe_mint = if let GMPPayload::InterchainTransfer(_) = payload {
        accounts
            .get(its_account_indices::TOKEN_MINT_INDEX)
            .map(|account| *account.key)
    } else {
        None
    };

    let token_program = accounts
        .get(its_account_indices::TOKEN_PROGRAM_INDEX)
        .map(|account| *account.key)
        .ok_or(ProgramError::InvalidAccountData)?;

    let (derived_its_accounts, new_bumps) = derive_its_accounts(
        gateway_root_pda,
        payload,
        token_program,
        maybe_mint,
        Some(Clock::get()?.unix_timestamp),
        Some(bumps),
    )?;

    if new_bumps != bumps {
        return Err(ProgramError::InvalidAccountData);
    }

    for element in accounts.iter().zip_longest(derived_its_accounts.iter()) {
        match element {
            itertools::EitherOrBoth::Both(provided, derived) => {
                if provided.key != &derived.pubkey {
                    return Err(ProgramError::InvalidAccountData);
                }
            }
            itertools::EitherOrBoth::Left(_) | itertools::EitherOrBoth::Right(_) => {
                return Err(ProgramError::InvalidAccountData)
            }
        }
    }

    // Now we validate the mint account passed for `InterchainTransfer`
    if let Some(mint) = maybe_mint {
        let token_manager_pda = accounts
            .get(its_account_indices::TOKEN_MANAGER_PDA_INDEX)
            .ok_or(ProgramError::InvalidAccountData)?;
        let token_manager_pda_data = token_manager_pda.try_borrow_data()?;

        let token_manager = check_rkyv_initialized_pda::<TokenManager>(
            &crate::id(),
            token_manager_pda,
            token_manager_pda_data.as_ref(),
        )?;

        if token_manager.token_address.as_ref() != mint.as_ref() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    Ok(())
}

fn process_operator_instruction<'a>(
    accounts: &'a [AccountInfo<'a>],
    instruction: instructions::operator::Instruction,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let gateway_root_pda = next_account_info(accounts_iter)?;
    let role_management_accounts = RoleManagementAccounts::try_from(accounts_iter.as_slice())?;

    let its_config = InterchainTokenService::load(&crate::id(), role_management_accounts.resource)?;
    let (derived_its_root_pda, _) =
        crate::create_its_root_pda(gateway_root_pda.key, its_config.bump);
    if derived_its_root_pda.ne(role_management_accounts.resource.key) {
        return Err(ProgramError::InvalidAccountData);
    }

    match instruction {
        instructions::operator::Instruction::TransferOperatorship(inputs) => {
            if inputs.roles.ne(&Roles::OPERATOR) {
                return Err(ProgramError::InvalidArgument);
            }

            role_management::processor::transfer(
                &crate::id(),
                role_management_accounts,
                &inputs,
                Roles::OPERATOR,
            )?;
        }
        instructions::operator::Instruction::ProposeOperatorship(inputs) => {
            if inputs.roles.ne(&Roles::OPERATOR) {
                return Err(ProgramError::InvalidArgument);
            }
            role_management::processor::propose(
                &crate::id(),
                role_management_accounts,
                &inputs,
                Roles::OPERATOR,
            )?;
        }
        instructions::operator::Instruction::AcceptOperatorship(inputs) => {
            if inputs.roles.ne(&Roles::OPERATOR) {
                return Err(ProgramError::InvalidArgument);
            }
            role_management::processor::accept(
                &crate::id(),
                role_management_accounts,
                &inputs,
                Roles::empty(),
            )?;
        }
    }

    Ok(())
}
