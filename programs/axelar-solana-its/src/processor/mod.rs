#![allow(clippy::too_many_arguments)]
//! Program state processor
use borsh::BorshDeserialize;
use event_utils::Event as _;
use program_utils::{
    pda::{BorshPda, ValidPDA},
    validate_system_account_key,
};
use role_management::processor::{
    ensure_signer_roles, ensure_upgrade_authority, RoleAddAccounts, RoleRemoveAccounts,
    RoleTransferWithProposalAccounts,
};
use role_management::state::UserRoles;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use token_manager::{handover_mint_authority, SetFlowLimitAccounts};

use crate::instruction::InterchainTokenServiceInstruction;
use crate::state::InterchainTokenService;
use crate::{assert_valid_its_root_pda, check_program_account, event, FromAccountInfoSlice, Roles};

pub(crate) mod gmp;
pub(crate) mod interchain_token;
pub(crate) mod interchain_transfer;
pub(crate) mod link_token;
pub(crate) mod token_manager;

/// Processes an instruction.
///
/// # Errors
///
/// A `ProgramError` containing the error that occurred is returned. Log
/// messages are also generated with more detailed information.
#[allow(clippy::too_many_lines)]
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
        InterchainTokenServiceInstruction::Initialize {
            chain_name,
            its_hub_address,
        } => process_initialize(program_id, accounts, chain_name, its_hub_address),
        InterchainTokenServiceInstruction::SetPauseStatus { paused } => {
            process_set_pause_status(accounts, paused)
        }
        InterchainTokenServiceInstruction::ItsGmpPayload { message } => {
            gmp::process_inbound(accounts, message)
        }
        InterchainTokenServiceInstruction::SetTrustedChain { chain_name } => {
            process_set_trusted_chain(accounts, chain_name)
        }
        InterchainTokenServiceInstruction::RemoveTrustedChain { chain_name } => {
            process_remove_trusted_chain(accounts, &chain_name)
        }
        InterchainTokenServiceInstruction::ApproveDeployRemoteInterchainToken {
            deployer,
            salt,
            destination_chain,
            destination_minter,
        } => interchain_token::approve_deploy_remote_interchain_token(
            accounts,
            deployer,
            salt,
            destination_chain,
            destination_minter,
        ),
        InterchainTokenServiceInstruction::RevokeDeployRemoteInterchainToken {
            deployer,
            salt,
            destination_chain,
        } => interchain_token::revoke_deploy_remote_interchain_token(
            accounts,
            deployer,
            salt,
            destination_chain,
        ),
        InterchainTokenServiceInstruction::RegisterCanonicalInterchainToken => {
            link_token::register_canonical_interchain_token(accounts)
        }
        InterchainTokenServiceInstruction::DeployRemoteCanonicalInterchainToken {
            destination_chain,
            gas_value,
            signing_pda_bump,
        } => interchain_token::deploy_remote_canonical_interchain_token(
            accounts,
            destination_chain,
            gas_value,
            signing_pda_bump,
        ),
        InterchainTokenServiceInstruction::DeployInterchainToken {
            salt,
            name,
            symbol,
            decimals,
            initial_supply,
        } => {
            interchain_token::process_deploy(accounts, salt, name, symbol, decimals, initial_supply)
        }
        InterchainTokenServiceInstruction::DeployRemoteInterchainToken {
            salt,
            destination_chain,
            gas_value,
            signing_pda_bump,
        } => interchain_token::deploy_remote_interchain_token(
            accounts,
            salt,
            destination_chain,
            None,
            gas_value,
            signing_pda_bump,
        ),
        InterchainTokenServiceInstruction::DeployRemoteInterchainTokenWithMinter {
            salt,
            destination_chain,
            destination_minter,
            gas_value,
            signing_pda_bump,
        } => interchain_token::deploy_remote_interchain_token(
            accounts,
            salt,
            destination_chain,
            Some(destination_minter),
            gas_value,
            signing_pda_bump,
        ),
        InterchainTokenServiceInstruction::InterchainTransfer {
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
        } => interchain_transfer::process_user_interchain_transfer(
            accounts,
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
            None,
        ),
        InterchainTokenServiceInstruction::ProgramInterchainTransfer {
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
            source_program_id,
            pda_seeds,
        } => interchain_transfer::process_program_interchain_transfer(
            accounts,
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
            Some(source_program_id),
            pda_seeds,
        ),
        InterchainTokenServiceInstruction::RegisterTokenMetadata {
            gas_value,
            signing_pda_bump,
        } => link_token::register_token_metadata(accounts, gas_value, signing_pda_bump),
        InterchainTokenServiceInstruction::RegisterCustomToken {
            salt,
            token_manager_type,
            operator,
        } => link_token::register_custom_token(accounts, salt, token_manager_type, operator),
        InterchainTokenServiceInstruction::LinkToken {
            salt,
            destination_chain,
            destination_token_address,
            token_manager_type,
            link_params,
            gas_value,
            signing_pda_bump,
        } => link_token::process_outbound(
            accounts,
            salt,
            destination_chain,
            destination_token_address,
            token_manager_type,
            link_params,
            gas_value,
            signing_pda_bump,
        ),
        InterchainTokenServiceInstruction::SetFlowLimit { flow_limit } => {
            let instruction_accounts =
                SetFlowLimitAccounts::from_account_info_slice(accounts, &true)?;

            msg!("Instruction: SetFlowLimit");
            ensure_signer_roles(
                &crate::id(),
                instruction_accounts.its_root_pda,
                instruction_accounts.payer,
                instruction_accounts.its_user_roles_pda,
                Roles::OPERATOR,
            )?;

            token_manager::set_flow_limit(&instruction_accounts, flow_limit)
        }
        InterchainTokenServiceInstruction::TransferOperatorship => {
            process_transfer_operatorship(accounts)
        }
        InterchainTokenServiceInstruction::ProposeOperatorship => {
            process_propose_operatorship(accounts)
        }
        InterchainTokenServiceInstruction::AcceptOperatorship => {
            process_accept_operatorship(accounts)
        }
        InterchainTokenServiceInstruction::AddTokenManagerFlowLimiter => {
            token_manager::process_add_flow_limiter(accounts)
        }
        InterchainTokenServiceInstruction::RemoveTokenManagerFlowLimiter => {
            token_manager::process_remove_flow_limiter(accounts)
        }
        InterchainTokenServiceInstruction::SetTokenManagerFlowLimit { flow_limit } => {
            token_manager::process_set_flow_limit(accounts, flow_limit)
        }
        InterchainTokenServiceInstruction::TransferTokenManagerOperatorship => {
            token_manager::process_transfer_operatorship(accounts)
        }
        InterchainTokenServiceInstruction::ProposeTokenManagerOperatorship => {
            token_manager::process_propose_operatorship(accounts)
        }
        InterchainTokenServiceInstruction::AcceptTokenManagerOperatorship => {
            token_manager::process_accept_operatorship(accounts)
        }
        InterchainTokenServiceInstruction::HandoverMintAuthority { token_id } => {
            handover_mint_authority(accounts, token_id)
        }
        InterchainTokenServiceInstruction::MintInterchainToken { amount } => {
            interchain_token::process_mint(accounts, amount)
        }
        InterchainTokenServiceInstruction::TransferInterchainTokenMintership => {
            interchain_token::process_transfer_mintership(accounts)
        }
        InterchainTokenServiceInstruction::ProposeInterchainTokenMintership => {
            interchain_token::process_propose_mintership(accounts)
        }
        InterchainTokenServiceInstruction::AcceptInterchainTokenMintership => {
            interchain_token::process_accept_mintership(accounts)
        }
        InterchainTokenServiceInstruction::CallContractWithInterchainToken {
            token_id,
            destination_chain,
            destination_address,
            amount,
            data,
            gas_value,
            signing_pda_bump,
        } => interchain_transfer::process_user_interchain_transfer(
            accounts,
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
            Some(data),
        ),
    }
}

fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo<'_>],
    chain_name: String,
    its_hub_address: String,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let payer = next_account_info(account_info_iter)?;
    let program_data_account = next_account_info(account_info_iter)?;
    let its_root_pda_account = next_account_info(account_info_iter)?;
    let system_account = next_account_info(account_info_iter)?;
    let operator = next_account_info(account_info_iter)?;
    let user_roles_account = next_account_info(account_info_iter)?;

    msg!("Instruction: Initialize");

    // Check: System Program Account
    validate_system_account_key(system_account.key)?;

    // Check: Upgrade Authority
    ensure_upgrade_authority(program_id, payer, program_data_account)?;

    // Check: PDA Account is not initialized
    its_root_pda_account.check_uninitialized_pda()?;

    let (its_root_pda, its_root_pda_bump) = crate::find_its_root_pda();
    if its_root_pda != *its_root_pda_account.key {
        return Err(ProgramError::InvalidAccountData);
    }

    let its_root_config =
        InterchainTokenService::new(its_root_pda_bump, chain_name, its_hub_address);
    its_root_config.init(
        &crate::id(),
        system_account,
        payer,
        its_root_pda_account,
        &[crate::seed_prefixes::ITS_SEED, &[its_root_pda_bump]],
    )?;

    let (user_roles_pda, user_roles_pda_bump) =
        role_management::find_user_roles_pda(&crate::id(), &its_root_pda, operator.key);
    if user_roles_pda != *user_roles_account.key {
        return Err(ProgramError::InvalidAccountData);
    }

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

fn process_transfer_operatorship<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let system_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let payer_roles_account = next_account_info(accounts_iter)?;
    let resource = next_account_info(accounts_iter)?;
    let destination_user_account = next_account_info(accounts_iter)?;
    let destination_roles_account = next_account_info(accounts_iter)?;

    validate_system_account_key(system_account.key)?;

    if payer.key == destination_user_account.key {
        msg!("Source and destination accounts are the same");
        return Err(ProgramError::InvalidArgument);
    }

    msg!("Instruction: TransferOperatorship");

    let its_config = InterchainTokenService::load(resource)?;
    assert_valid_its_root_pda(resource, its_config.bump)?;

    let role_add_accounts = RoleAddAccounts {
        system_account,
        payer,
        payer_roles_account,
        resource,
        destination_user_account,
        destination_roles_account,
    };

    let role_remove_accounts = RoleRemoveAccounts {
        system_account,
        payer,
        payer_roles_account,
        resource,
        origin_user_account: payer,
        origin_roles_account: payer_roles_account,
    };

    role_management::processor::add(
        &crate::id(),
        role_add_accounts,
        Roles::OPERATOR,
        Roles::OPERATOR,
    )?;

    role_management::processor::remove(
        &crate::id(),
        role_remove_accounts,
        Roles::OPERATOR,
        Roles::OPERATOR,
    )
}

fn process_propose_operatorship<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let system_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let payer_roles_account = next_account_info(accounts_iter)?;
    let resource = next_account_info(accounts_iter)?;
    let destination_user_account = next_account_info(accounts_iter)?;
    let destination_roles_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    msg!("Instruction: ProposeOperatorship");

    validate_system_account_key(system_account.key)?;

    let its_config = InterchainTokenService::load(resource)?;
    assert_valid_its_root_pda(resource, its_config.bump)?;

    let role_management_accounts = RoleTransferWithProposalAccounts {
        system_account,
        payer,
        payer_roles_account,
        resource,
        destination_user_account,
        destination_roles_account,
        origin_user_account: payer,
        origin_roles_account: payer_roles_account,
        proposal_account,
    };

    role_management::processor::propose(&crate::id(), role_management_accounts, Roles::OPERATOR)
}

fn process_accept_operatorship<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let system_account = next_account_info(accounts_iter)?;
    let payer = next_account_info(accounts_iter)?;
    let payer_roles_account = next_account_info(accounts_iter)?;
    let resource = next_account_info(accounts_iter)?;
    let origin_user_account = next_account_info(accounts_iter)?;
    let origin_roles_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    msg!("Instruction: AcceptOperatorship");

    validate_system_account_key(system_account.key)?;

    let role_management_accounts = RoleTransferWithProposalAccounts {
        system_account,
        payer,
        payer_roles_account,
        resource,
        destination_user_account: payer,
        destination_roles_account: payer_roles_account,
        origin_user_account,
        origin_roles_account,
        proposal_account,
    };

    role_management::processor::accept(&crate::id(), role_management_accounts, Roles::OPERATOR)
}

fn process_set_pause_status<'a>(accounts: &'a [AccountInfo<'a>], paused: bool) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let program_data_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;

    validate_system_account_key(system_account.key)?;

    msg!("Instruction: SetPauseStatus");

    ensure_upgrade_authority(&crate::id(), payer, program_data_account)?;

    let mut its_root_config = InterchainTokenService::load(its_root_pda)?;
    assert_valid_its_root_pda(its_root_pda, its_root_config.bump)?;

    its_root_config.paused = paused;
    its_root_config.store(payer, its_root_pda, system_account)?;

    Ok(())
}

fn process_set_trusted_chain<'a>(
    accounts: &'a [AccountInfo<'a>],
    chain_name: String,
) -> ProgramResult {
    let (payer, payer_roles, program_data_account, its_root_pda, system_account) =
        get_trusted_chain_accounts(accounts)?;
    msg!("Instruction: SetTrustedChain");

    if ensure_upgrade_authority(&crate::id(), payer, program_data_account).is_err()
        && ensure_signer_roles(
            &crate::id(),
            its_root_pda,
            payer,
            payer_roles,
            Roles::OPERATOR,
        )
        .is_err()
    {
        msg!("Payer is neither upgrade authority nor operator");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut its_root_config = InterchainTokenService::load(its_root_pda)?;
    assert_valid_its_root_pda(its_root_pda, its_root_config.bump)?;

    let trusted_chain_event = event::TrustedChainSet { chain_name };
    trusted_chain_event.emit();
    its_root_config.add_trusted_chain(trusted_chain_event.chain_name);
    its_root_config.store(payer, its_root_pda, system_account)?;

    Ok(())
}

fn process_remove_trusted_chain<'a>(
    accounts: &'a [AccountInfo<'a>],
    chain_name: &str,
) -> ProgramResult {
    let (payer, payer_roles, program_data_account, its_root_pda, system_account) =
        get_trusted_chain_accounts(accounts)?;

    msg!("Instruction: RemoveTrustedChain");

    if ensure_upgrade_authority(&crate::id(), payer, program_data_account).is_err()
        && ensure_signer_roles(
            &crate::id(),
            its_root_pda,
            payer,
            payer_roles,
            Roles::OPERATOR,
        )
        .is_err()
    {
        msg!("Payer is neither upgrade authority nor operator");
        return Err(ProgramError::MissingRequiredSignature);
    }
    let mut its_root_config = InterchainTokenService::load(its_root_pda)?;
    assert_valid_its_root_pda(its_root_pda, its_root_config.bump)?;

    event::TrustedChainRemoved {
        chain_name: chain_name.to_owned(),
    }
    .emit();

    its_root_config.remove_trusted_chain(chain_name);
    its_root_config.store(payer, its_root_pda, system_account)?;

    Ok(())
}

fn get_trusted_chain_accounts<'a>(
    accounts: &'a [AccountInfo<'a>],
) -> Result<
    (
        &'a AccountInfo<'a>,
        &'a AccountInfo<'a>,
        &'a AccountInfo<'a>,
        &'a AccountInfo<'a>,
        &'a AccountInfo<'a>,
    ),
    ProgramError,
> {
    let accounts_iter = &mut accounts.iter();
    let payer = next_account_info(accounts_iter)?;
    let roles_account = next_account_info(accounts_iter)?;
    let program_data_account = next_account_info(accounts_iter)?;
    let its_root_pda = next_account_info(accounts_iter)?;
    let system_account = next_account_info(accounts_iter)?;

    validate_system_account_key(system_account.key)?;
    Ok((
        payer,
        roles_account,
        program_data_account,
        its_root_pda,
        system_account,
    ))
}
