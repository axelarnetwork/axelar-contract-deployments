#![allow(clippy::too_many_arguments)]
//! Program state processor
use borsh::BorshDeserialize;
use event_cpi::EventAccounts;
use event_cpi_macros::{emit_cpi, event_cpi_accounts, event_cpi_handler};
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
use token_manager::handover_mint_authority;

use crate::state::InterchainTokenService;
use crate::{accounts::RemoveTrustedChainAccounts, state::token_manager::TokenManager};
use crate::{accounts::SetTrustedChainAccounts, instruction::InterchainTokenServiceInstruction};
use crate::{assert_valid_its_root_pda, check_program_account, events, Roles};

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

    event_cpi_handler!(instruction_data);

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
        InterchainTokenServiceInstruction::Execute { message } => {
            gmp::process_execute(accounts.try_into()?, message)
        }
        InterchainTokenServiceInstruction::SetTrustedChain { chain_name } => {
            process_set_trusted_chain(accounts.try_into()?, chain_name)
        }
        InterchainTokenServiceInstruction::RemoveTrustedChain { chain_name } => {
            process_remove_trusted_chain(accounts.try_into()?, &chain_name)
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
            link_token::register_canonical_interchain_token(accounts.try_into()?)
        }
        InterchainTokenServiceInstruction::DeployRemoteCanonicalInterchainToken {
            destination_chain,
            gas_value,
            signing_pda_bump,
        } => interchain_token::deploy_remote_canonical_interchain_token(
            accounts.try_into()?,
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
        } => interchain_token::process_deploy(
            accounts.try_into()?,
            salt,
            name,
            symbol,
            decimals,
            initial_supply,
        ),
        InterchainTokenServiceInstruction::DeployRemoteInterchainToken {
            salt,
            destination_chain,
            gas_value,
            signing_pda_bump,
        } => interchain_token::deploy_remote_interchain_token(
            accounts.try_into()?,
            salt,
            destination_chain,
            gas_value,
            signing_pda_bump,
        ),
        InterchainTokenServiceInstruction::DeployRemoteInterchainTokenWithMinter {
            salt,
            destination_chain,
            destination_minter,
            gas_value,
            signing_pda_bump,
        } => interchain_token::deploy_remote_interchain_token_with_minter(
            accounts.try_into()?,
            salt,
            destination_chain,
            destination_minter,
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
            accounts.try_into()?,
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
            None,
        ),
        InterchainTokenServiceInstruction::CpiInterchainTransfer {
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
            source_program_id,
            pda_seeds,
        } => interchain_transfer::process_cpi_interchain_transfer(
            accounts.try_into()?,
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
            source_program_id,
            pda_seeds,
            None,
        ),
        InterchainTokenServiceInstruction::RegisterTokenMetadata {
            gas_value,
            signing_pda_bump,
        } => link_token::register_token_metadata(accounts.try_into()?, gas_value, signing_pda_bump),
        InterchainTokenServiceInstruction::RegisterCustomToken {
            salt,
            token_manager_type,
            operator,
        } => link_token::register_custom_token(
            accounts.try_into()?,
            salt,
            token_manager_type,
            operator,
        ),
        InterchainTokenServiceInstruction::LinkToken {
            salt,
            destination_chain,
            destination_token_address,
            token_manager_type,
            link_params,
            gas_value,
            signing_pda_bump,
        } => link_token::process_outbound(
            accounts.try_into()?,
            salt,
            destination_chain,
            destination_token_address,
            token_manager_type,
            link_params,
            gas_value,
            signing_pda_bump,
        ),
        InterchainTokenServiceInstruction::SetFlowLimit { flow_limit } => {
            let accounts_iter = &mut accounts.iter();
            let payer_account = next_account_info(accounts_iter)?;
            let operator_account = next_account_info(accounts_iter)?;
            let its_root_account = next_account_info(accounts_iter)?;
            let its_roles_account = next_account_info(accounts_iter)?;
            let token_manager_account = next_account_info(accounts_iter)?;
            let system_program_account = next_account_info(accounts_iter)?;

            event_cpi_accounts!(accounts_iter);

            msg!("Instruction: SetFlowLimit");

            let its_config_pda = InterchainTokenService::load(its_root_account)?;
            assert_valid_its_root_pda(its_root_account, its_config_pda.bump)?;

            validate_system_account_key(system_program_account.key)?;

            ensure_signer_roles(
                &crate::id(),
                its_root_account,
                operator_account,
                its_roles_account,
                Roles::OPERATOR,
            )?;

            let token_manager = TokenManager::load(token_manager_account)?;

            token_manager::set_flow_limit(
                payer_account,
                token_manager_account,
                its_root_account,
                system_program_account,
                flow_limit,
            )?;

            if let Some(limit) = flow_limit {
                emit_cpi!(events::FlowLimitSet {
                    token_id: token_manager.token_id,
                    operator: *operator_account.key,
                    flow_limit: limit,
                });
            }

            Ok(())
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
            accounts.try_into()?,
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
            Some(data),
        ),
        InterchainTokenServiceInstruction::CpiCallContractWithInterchainToken {
            token_id,
            destination_chain,
            destination_address,
            amount,
            data,
            gas_value,
            signing_pda_bump,
            source_program_id,
            pda_seeds,
        } => interchain_transfer::process_cpi_interchain_transfer(
            accounts.try_into()?,
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
            source_program_id,
            pda_seeds,
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
    let payer_account = next_account_info(account_info_iter)?;
    let program_data_account = next_account_info(account_info_iter)?;
    let its_root_account = next_account_info(account_info_iter)?;
    let system_program_account = next_account_info(account_info_iter)?;
    let operator_account = next_account_info(account_info_iter)?;
    let user_roles_account = next_account_info(account_info_iter)?;

    msg!("Instruction: Initialize");

    // Check: System Program Account
    validate_system_account_key(system_program_account.key)?;

    // Check: Upgrade Authority
    ensure_upgrade_authority(program_id, payer_account, program_data_account)?;

    // Check: PDA Account is not initialized
    its_root_account.check_uninitialized_pda()?;

    let (its_root_pda, its_root_pda_bump) = crate::find_its_root_pda();
    if its_root_pda != *its_root_account.key {
        return Err(ProgramError::InvalidAccountData);
    }

    let its_root_config =
        InterchainTokenService::new(its_root_pda_bump, chain_name, its_hub_address);
    its_root_config.init(
        &crate::id(),
        system_program_account,
        payer_account,
        its_root_account,
        &[crate::seed_prefixes::ITS_SEED, &[its_root_pda_bump]],
    )?;

    let (user_roles_pda, user_roles_pda_bump) =
        role_management::find_user_roles_pda(&crate::id(), &its_root_pda, operator_account.key);
    if user_roles_pda != *user_roles_account.key {
        return Err(ProgramError::InvalidAccountData);
    }

    let operator_user_roles = UserRoles::new(Roles::OPERATOR, user_roles_pda_bump);
    let signer_seeds = &[
        role_management::seed_prefixes::USER_ROLES_SEED,
        its_root_pda.as_ref(),
        operator_account.key.as_ref(),
        &[user_roles_pda_bump],
    ];

    operator_user_roles.init(
        program_id,
        system_program_account,
        payer_account,
        user_roles_account,
        signer_seeds,
    )?;

    Ok(())
}

fn process_transfer_operatorship<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let system_program_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let origin_user_account = next_account_info(accounts_iter)?;
    let origin_roles_account = next_account_info(accounts_iter)?;
    let resource_account = next_account_info(accounts_iter)?;
    let destination_user_account = next_account_info(accounts_iter)?;
    let destination_roles_account = next_account_info(accounts_iter)?;

    validate_system_account_key(system_program_account.key)?;

    if origin_user_account.key == destination_user_account.key {
        msg!("Source and destination accounts are the same");
        return Err(ProgramError::InvalidArgument);
    }

    msg!("Instruction: TransferOperatorship");

    let its_config = InterchainTokenService::load(resource_account)?;
    assert_valid_its_root_pda(resource_account, its_config.bump)?;

    let role_add_accounts = RoleAddAccounts {
        system_account: system_program_account,
        payer: payer_account,
        authority_user_account: origin_user_account,
        authority_roles_account: origin_roles_account,
        resource: resource_account,
        target_user_account: destination_user_account,
        target_roles_account: destination_roles_account,
    };

    let role_remove_accounts = RoleRemoveAccounts {
        system_account: system_program_account,
        payer: payer_account,
        authority_user_account: origin_user_account,
        authority_roles_account: origin_roles_account,
        resource: resource_account,
        target_user_account: origin_user_account,
        target_roles_account: origin_roles_account,
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

    let system_program_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let proposer_user_account = next_account_info(accounts_iter)?;
    let proposer_roles_account = next_account_info(accounts_iter)?;
    let resource_account = next_account_info(accounts_iter)?;
    let destination_user_account = next_account_info(accounts_iter)?;
    let destination_roles_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    msg!("Instruction: ProposeOperatorship");

    validate_system_account_key(system_program_account.key)?;

    let its_config = InterchainTokenService::load(resource_account)?;
    assert_valid_its_root_pda(resource_account, its_config.bump)?;

    let role_management_accounts = RoleTransferWithProposalAccounts {
        system_account: system_program_account,
        payer: payer_account,
        origin_user_account: proposer_user_account,
        origin_roles_account: proposer_roles_account,
        resource: resource_account,
        destination_user_account,
        destination_roles_account,
        proposal_account,
    };

    role_management::processor::propose(&crate::id(), role_management_accounts, Roles::OPERATOR)
}

fn process_accept_operatorship<'a>(accounts: &'a [AccountInfo<'a>]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let system_program_account = next_account_info(accounts_iter)?;
    let payer_account = next_account_info(accounts_iter)?;
    let role_receiver_account = next_account_info(accounts_iter)?;
    let role_receiver_roles_account = next_account_info(accounts_iter)?;
    let resource_account = next_account_info(accounts_iter)?;
    let origin_user_account = next_account_info(accounts_iter)?;
    let origin_roles_account = next_account_info(accounts_iter)?;
    let proposal_account = next_account_info(accounts_iter)?;

    msg!("Instruction: AcceptOperatorship");

    validate_system_account_key(system_program_account.key)?;

    let role_management_accounts = RoleTransferWithProposalAccounts {
        system_account: system_program_account,
        payer: payer_account,
        resource: resource_account,
        destination_user_account: role_receiver_account,
        destination_roles_account: role_receiver_roles_account,
        origin_user_account,
        origin_roles_account,
        proposal_account,
    };

    role_management::processor::accept(&crate::id(), role_management_accounts, Roles::OPERATOR)
}

fn process_set_pause_status<'a>(accounts: &'a [AccountInfo<'a>], paused: bool) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let owner_account = next_account_info(accounts_iter)?;
    let program_data_account = next_account_info(accounts_iter)?;
    let its_root_account = next_account_info(accounts_iter)?;
    let system_program_account = next_account_info(accounts_iter)?;

    validate_system_account_key(system_program_account.key)?;

    msg!("Instruction: SetPauseStatus");

    ensure_upgrade_authority(&crate::id(), owner_account, program_data_account)?;

    let mut its_root_config = InterchainTokenService::load(its_root_account)?;
    assert_valid_its_root_pda(its_root_account, its_root_config.bump)?;

    its_root_config.paused = paused;
    its_root_config.store(owner_account, its_root_account, system_program_account)?;

    Ok(())
}

fn process_set_trusted_chain(
    accounts: SetTrustedChainAccounts,
    chain_name: String,
) -> ProgramResult {
    msg!("Instruction: SetTrustedChain");

    let event_accounts = &mut accounts.event_accounts().into_iter();
    event_cpi_accounts!(event_accounts);

    if ensure_upgrade_authority(&crate::id(), accounts.authority, accounts.program_data).is_err()
        && ensure_signer_roles(
            &crate::id(),
            accounts.its_root,
            accounts.authority,
            accounts.authority_roles,
            Roles::OPERATOR,
        )
        .is_err()
    {
        msg!("Account passed as authority is neither upgrade authority nor operator");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut its_root = InterchainTokenService::load(accounts.its_root)?;
    assert_valid_its_root_pda(accounts.its_root, its_root.bump)?;

    let trusted_chain_event = events::TrustedChainSet { chain_name };
    emit_cpi!(trusted_chain_event);
    its_root.add_trusted_chain(trusted_chain_event.chain_name);
    its_root.store(accounts.payer, accounts.its_root, accounts.system_program)?;

    Ok(())
}

fn process_remove_trusted_chain(
    accounts: RemoveTrustedChainAccounts,
    chain_name: &str,
) -> ProgramResult {
    msg!("Instruction: RemoveTrustedChain");

    let event_accounts = &mut accounts.event_accounts().into_iter();
    event_cpi_accounts!(event_accounts);

    if ensure_upgrade_authority(&crate::id(), accounts.authority, accounts.program_data).is_err()
        && ensure_signer_roles(
            &crate::id(),
            accounts.its_root,
            accounts.authority,
            accounts.authority_roles,
            Roles::OPERATOR,
        )
        .is_err()
    {
        msg!("Account passed as authority is neither upgrade authority nor operator");
        return Err(ProgramError::MissingRequiredSignature);
    }
    let mut its_root = InterchainTokenService::load(accounts.its_root)?;
    assert_valid_its_root_pda(accounts.its_root, its_root.bump)?;

    emit_cpi!(events::TrustedChainRemoved {
        chain_name: chain_name.to_owned(),
    });

    its_root.remove_trusted_chain(chain_name)?;
    its_root.store(accounts.payer, accounts.its_root, accounts.system_program)?;

    Ok(())
}
