//! Program state processor

use axelar_executable::{validate_with_gmp_metadata, PROGRAM_ACCOUNTS_START_INDEX};
use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway::error::GatewayError;
use axelar_solana_gateway::state::message_payload::ImmutMessagePayload;
use axelar_solana_gateway::state::GatewayConfig;
use borsh::BorshDeserialize;
use interchain_token_transfer_gmp::{GMPPayload, SendToHub};
use program_utils::{BorshPda, BytemuckedPda, ValidPDA};
use role_management::processor::{
    ensure_signer_roles, ensure_upgrade_authority, RoleManagementAccounts,
};
use role_management::state::UserRoles;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{msg, system_program};

use self::interchain_transfer::process_inbound_transfer;
use self::token_manager::SetFlowLimitAccounts;
use crate::instructions::{
    self, InterchainTokenServiceInstruction, OptionalAccountsFlags, OutboundInstructionInputs,
};
use crate::state::InterchainTokenService;
use crate::{assert_valid_its_root_pda, check_program_account, seed_prefixes, Roles};

pub mod interchain_token;
pub mod interchain_transfer;
pub mod token_manager;

const ITS_HUB_TRUSTED_CHAIN_NAME: &str = "axelar";
const ITS_HUB_TRUSTED_CONTRACT_ADDRESS: &str =
    "axelar157hl7gpuknjmhtac2qnphuazv2yerfagva7lsu9vuj2pgn32z22qa26dk4";

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
    let instruction = match InterchainTokenServiceInstruction::try_from_slice(instruction_data) {
        Ok(instruction) => instruction,
        Err(err) => {
            msg!("Failed to deserialize instruction: {:?}", err);
            return Err(ProgramError::InvalidInstructionData);
        }
    };

    match instruction {
        InterchainTokenServiceInstruction::Initialize => {
            process_initialize(program_id, accounts)?;
        }
        InterchainTokenServiceInstruction::SetPauseStatus { paused } => {
            process_set_pause_status(accounts, paused)?;
        }
        InterchainTokenServiceInstruction::ItsGmpPayload {
            message,
            optional_accounts_mask,
        } => {
            process_inbound_its_gmp_payload(accounts, message, &optional_accounts_mask)?;
        }
        InterchainTokenServiceInstruction::DeployInterchainToken { params } => {
            process_its_native_deploy_call(accounts, params, &OptionalAccountsFlags::empty())?;
        }
        InterchainTokenServiceInstruction::DeployTokenManager {
            params,
            optional_accounts_mask,
        } => {
            process_its_native_deploy_call(accounts, params, &optional_accounts_mask)?;
        }
        InterchainTokenServiceInstruction::InterchainTransfer { params } => {
            interchain_transfer::process_outbound_transfer(params, accounts)?;
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
        InterchainTokenServiceInstruction::OperatorInstruction(operator_instruction) => {
            process_operator_instruction(accounts, operator_instruction)?;
        }
        InterchainTokenServiceInstruction::TokenManagerInstruction(token_manager_instruction) => {
            token_manager::process_instruction(accounts, token_manager_instruction)?;
        }
        InterchainTokenServiceInstruction::InterchainTokenInstruction(
            interchain_token_instruction,
        ) => {
            interchain_token::process_instruction(accounts, interchain_token_instruction)?;
        }
        InterchainTokenServiceInstruction::CallContractWithInterchainToken { params }
        | InterchainTokenServiceInstruction::CallContractWithInterchainTokenOffchainData {
            params,
        } => {
            if params.data.is_empty() {
                return Err(ProgramError::InvalidInstructionData);
            }

            interchain_transfer::process_outbound_transfer(params, accounts)?;
        }
    }

    Ok(())
}

fn process_initialize(program_id: &Pubkey, accounts: &[AccountInfo<'_>]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let program_data_account = next_account_info(account_info_iter)?;
    let gateway_root_pda_account = next_account_info(account_info_iter)?;
    let its_root_pda_account = next_account_info(account_info_iter)?;
    let system_account = next_account_info(account_info_iter)?;
    let operator = next_account_info(account_info_iter)?;
    let user_roles_account = next_account_info(account_info_iter)?;

    // Check: System Program Account
    if !system_program::check_id(system_account.key) {
        return Err(ProgramError::IncorrectProgramId);
    }

    // Check: Upgrade Authority
    ensure_upgrade_authority(program_id, payer, program_data_account)?;

    // Check: PDA Account is not initialized
    its_root_pda_account.check_uninitialized_pda()?;

    // Check: Gateway Root PDA Account is valid.
    let gateway_config_data = gateway_root_pda_account.try_borrow_data()?;
    let gateway_config =
        GatewayConfig::read(&gateway_config_data).ok_or(GatewayError::BytemuckDataLenInvalid)?;
    axelar_solana_gateway::assert_valid_gateway_root_pda(
        gateway_config.bump,
        gateway_root_pda_account.key,
    )?;

    let (its_root_pda, its_root_pda_bump) = crate::find_its_root_pda(gateway_root_pda_account.key);
    let its_root_config = InterchainTokenService::new(its_root_pda_bump);
    its_root_config.init(
        &crate::id(),
        system_account,
        payer,
        its_root_pda_account,
        &[
            crate::seed_prefixes::ITS_SEED,
            gateway_root_pda_account.key.as_ref(),
            &[its_root_pda_bump],
        ],
    )?;

    let (_user_roles_pda, user_roles_pda_bump) =
        role_management::find_user_roles_pda(&crate::id(), &its_root_pda, operator.key);
    let operator_user_roles = UserRoles::new(Roles::OPERATOR, user_roles_pda_bump);
    let signer_seeds = &[
        role_management::seed_prefixes::USER_ROLES_SEED,
        its_root_pda.as_ref(),
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
    message: Message,
    optional_accounts_flags: &OptionalAccountsFlags,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;

    let (gateway_accounts, instruction_accounts) = accounts_iter
        .as_slice()
        .split_at(PROGRAM_ACCOUNTS_START_INDEX);

    if message.source_address != ITS_HUB_TRUSTED_CONTRACT_ADDRESS {
        msg!("Untrusted source address: {}", message.source_address);
        return Err(ProgramError::InvalidInstructionData);
    }

    validate_with_gmp_metadata(gateway_accounts, &message)?;

    let _gateway_approved_message_pda = next_account_info(accounts_iter)?;
    let payload_account = next_account_info(accounts_iter)?;
    let _signing_pda = next_account_info(accounts_iter)?;
    let _gateway_program_id = next_account_info(accounts_iter)?;
    let gateway_root_pda_account = next_account_info(accounts_iter)?;
    let _system_program = next_account_info(accounts_iter)?;
    let its_root_pda_account = next_account_info(accounts_iter)?;

    let its_root_config = InterchainTokenService::load(its_root_pda_account)?;
    assert_valid_its_root_pda(
        its_root_pda_account,
        gateway_root_pda_account.key,
        its_root_config.bump,
    )?;

    if its_root_config.paused {
        msg!("The Interchain Token Service is currently paused.");
        return Err(ProgramError::Immutable);
    }

    let payload_account_data = payload_account.try_borrow_data()?;
    let message_payload: ImmutMessagePayload<'_> = (**payload_account_data).try_into()?;

    let GMPPayload::ReceiveFromHub(inner) = GMPPayload::decode(message_payload.raw_payload)
        .map_err(|_err| ProgramError::InvalidInstructionData)?
    else {
        msg!("Unsupported GMP payload");
        return Err(ProgramError::InvalidInstructionData);
    };

    let payload =
        GMPPayload::decode(&inner.payload).map_err(|_err| ProgramError::InvalidInstructionData)?;

    match payload {
        GMPPayload::InterchainTransfer(transfer) => process_inbound_transfer(
            message,
            payer,
            payload_account,
            instruction_accounts,
            &transfer,
        ),
        GMPPayload::DeployInterchainToken(deploy) => {
            interchain_token::process_deploy(payer, instruction_accounts, deploy)
        }
        GMPPayload::DeployTokenManager(deploy) => token_manager::process_deploy(
            payer,
            instruction_accounts,
            &deploy,
            optional_accounts_flags,
        ),
        GMPPayload::SendToHub(_) | GMPPayload::ReceiveFromHub(_) => {
            Err(ProgramError::InvalidInstructionData)
        }
    }
}

fn process_its_native_deploy_call<'a, T>(
    accounts: &'a [AccountInfo<'a>],
    mut payload: T,
    optional_accounts_flags: &OptionalAccountsFlags,
) -> ProgramResult
where
    T: TryInto<GMPPayload> + OutboundInstructionInputs,
{
    let (payer, other_accounts) = accounts
        .split_first()
        .ok_or(ProgramError::InvalidInstructionData)?;

    let gas_value = payload.gas_value();
    let destination_chain = payload.destination_chain();

    let payload: GMPPayload = payload
        .try_into()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    match destination_chain {
        Some(chain) => {
            process_outbound_its_gmp_payload(
                payer,
                other_accounts,
                &payload,
                chain,
                gas_value,
                None,
            )?;
        }
        None => match payload {
            GMPPayload::DeployInterchainToken(deploy) => {
                interchain_token::process_deploy(payer, other_accounts, deploy)?;
            }
            GMPPayload::DeployTokenManager(deploy) => token_manager::process_deploy(
                payer,
                other_accounts,
                &deploy,
                optional_accounts_flags,
            )?,
            GMPPayload::SendToHub(_)
            | GMPPayload::ReceiveFromHub(_)
            | GMPPayload::InterchainTransfer(_) => {
                return Err(ProgramError::InvalidInstructionData)
            }
        },
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
fn process_outbound_its_gmp_payload<'a>(
    payer: &'a AccountInfo<'a>,
    accounts: &'a [AccountInfo<'a>],
    payload: &GMPPayload,
    destination_chain: String,
    gas_value: u64,
    payload_hash: Option<[u8; 32]>,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let gateway_root_account = next_account_info(accounts_iter)?;
    let _gateway_program_id = next_account_info(accounts_iter)?;
    let gas_service_config_account = next_account_info(accounts_iter)?;
    let gas_service = next_account_info(accounts_iter)?;
    let system_program = next_account_info(accounts_iter)?;
    let its_root_account = next_account_info(accounts_iter)?;
    let its_root_config = InterchainTokenService::load(its_root_account)?;
    assert_valid_its_root_pda(
        its_root_account,
        gateway_root_account.key,
        its_root_config.bump,
    )?;
    if its_root_config.paused {
        msg!("The Interchain Token Service is currently paused.");
        return Err(ProgramError::Immutable);
    }

    let (payload_hash, call_contract_ix) = if let Some(payload_hash) = payload_hash {
        let ix = axelar_solana_gateway::instructions::call_contract_offchain_data(
            axelar_solana_gateway::id(),
            *gateway_root_account.key,
            *its_root_account.key,
            ITS_HUB_TRUSTED_CHAIN_NAME.to_owned(),
            ITS_HUB_TRUSTED_CONTRACT_ADDRESS.to_owned(),
            payload_hash,
        )?;

        (payload_hash, ix)
    } else {
        let hub_payload = GMPPayload::SendToHub(SendToHub {
            selector: SendToHub::MESSAGE_TYPE_ID
                .try_into()
                .map_err(|_err| ProgramError::ArithmeticOverflow)?,
            destination_chain,
            payload: payload.encode().into(),
        })
        .encode();
        let payload_hash = if gas_value > 0 {
            solana_program::keccak::hashv(&[&hub_payload]).0
        } else {
            [0; 32]
        };

        let ix = axelar_solana_gateway::instructions::call_contract(
            axelar_solana_gateway::id(),
            *gateway_root_account.key,
            *its_root_account.key,
            ITS_HUB_TRUSTED_CHAIN_NAME.to_owned(),
            ITS_HUB_TRUSTED_CONTRACT_ADDRESS.to_owned(),
            hub_payload,
        )?;

        (payload_hash, ix)
    };

    if gas_value > 0 {
        let gas_payment_ix =
            axelar_solana_gas_service::instructions::pay_native_for_contract_call_instruction(
                gas_service.key,
                payer.key,
                gas_service_config_account.key,
                ITS_HUB_TRUSTED_CHAIN_NAME.to_owned(),
                ITS_HUB_TRUSTED_CONTRACT_ADDRESS.to_owned(),
                payload_hash,
                *payer.key,
                vec![],
                gas_value,
            )?;

        invoke(
            &gas_payment_ix,
            &[
                payer.clone(),
                gas_service_config_account.clone(),
                system_program.clone(),
            ],
        )?;
    }

    invoke_signed(
        &call_contract_ix,
        &[its_root_account.clone(), gateway_root_account.clone()],
        &[&[
            seed_prefixes::ITS_SEED,
            gateway_root_account.key.as_ref(),
            &[its_root_config.bump],
        ]],
    )?;

    Ok(())
}

fn process_operator_instruction<'a>(
    accounts: &'a [AccountInfo<'a>],
    instruction: instructions::operator::Instruction,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let gateway_root_pda = next_account_info(accounts_iter)?;
    let role_management_accounts = RoleManagementAccounts::try_from(accounts_iter.as_slice())?;

    let its_config = InterchainTokenService::load(role_management_accounts.resource)?;
    assert_valid_its_root_pda(
        role_management_accounts.resource,
        gateway_root_pda.key,
        its_config.bump,
    )?;

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

fn process_set_pause_status(accounts: &[AccountInfo<'_>], paused: bool) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let program_data_account = next_account_info(accounts_iter)?;
    let gateway_root_pda_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;

    ensure_upgrade_authority(&crate::id(), payer, program_data_account)?;
    let mut its_root_config = InterchainTokenService::load(its_root_pda)?;
    assert_valid_its_root_pda(
        its_root_pda,
        gateway_root_pda_account.key,
        its_root_config.bump,
    )?;
    its_root_config.paused = paused;
    its_root_config.store(its_root_pda)?;

    Ok(())
}
