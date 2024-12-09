//! This module provides logic to handle user role management instructions.
use program_utils::{close_pda, BorshPda};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{bpf_loader_upgradeable, msg};

use crate::instructions::RoleManagementInstructionInputs;
use crate::seed_prefixes;
use crate::state::{RoleProposal, RolesFlags, UserRoles};

/// Propose a role transfer from one user to another.
///
/// # Errors
///
/// [`ProgramError`] is returned as a result of failed operations.
pub fn propose<F: RolesFlags>(
    program_id: &Pubkey,
    accounts: RoleManagementAccounts<'_>,
    inputs: &RoleManagementInstructionInputs<F>,
    required_payer_roles: F,
) -> ProgramResult {
    let transfer_accounts = RoleTransferWithProposalAccounts::try_from(accounts)?;

    ensure_signer_roles(
        program_id,
        transfer_accounts.resource,
        transfer_accounts.payer,
        transfer_accounts.payer_roles_account,
        required_payer_roles,
    )?;

    ensure_roles(
        program_id,
        transfer_accounts.resource,
        transfer_accounts.origin_user_account,
        transfer_accounts.origin_roles_account,
        inputs.roles,
    )?;

    let proposal = RoleProposal {
        roles: inputs.roles,
    };

    let Some(proposal_pda_bump) = inputs.proposal_pda_bump else {
        return Err(ProgramError::InvalidArgument);
    };

    proposal.init(
        program_id,
        transfer_accounts.system_account,
        transfer_accounts.payer,
        transfer_accounts.proposal_account,
        &[
            seed_prefixes::ROLE_PROPOSAL_SEED,
            transfer_accounts.resource.key.as_ref(),
            transfer_accounts.origin_user_account.key.as_ref(),
            transfer_accounts.destination_user_account.key.as_ref(),
            &[proposal_pda_bump],
        ],
    )?;

    Ok(())
}

/// Accept a role transfer proposal.
///
/// # Errors
///
/// [`ProgramError`] is returned as a result of failed operations.
pub fn accept<F: RolesFlags>(
    program_id: &Pubkey,
    accounts: RoleManagementAccounts<'_>,
    inputs: &RoleManagementInstructionInputs<F>,
    required_payer_roles: F,
) -> ProgramResult {
    let transfer_accounts = RoleTransferWithProposalAccounts::try_from(accounts)?;

    ensure_signer_roles(
        program_id,
        transfer_accounts.resource,
        transfer_accounts.payer,
        transfer_accounts.payer_roles_account,
        required_payer_roles,
    )?;

    if !transfer_accounts.payer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (derived_pda, _) = crate::create_user_roles_pda(
        program_id,
        transfer_accounts.resource.key,
        transfer_accounts.destination_user_account.key,
        inputs.destination_roles_pda_bump,
    );

    if *transfer_accounts.destination_roles_account.key != derived_pda {
        msg!("Derived PDA doesn't match destination roles account");
        return Err(ProgramError::InvalidArgument);
    }

    let Some(proposal_pda_bump) = inputs.proposal_pda_bump else {
        return Err(ProgramError::InvalidArgument);
    };

    let (derived_proposal_pda, _) = crate::create_roles_proposal_pda(
        program_id,
        transfer_accounts.resource.key,
        transfer_accounts.origin_user_account.key,
        transfer_accounts.destination_user_account.key,
        proposal_pda_bump,
    );

    if derived_proposal_pda != *transfer_accounts.proposal_account.key {
        msg!("Derived PDA doesn't match given  proposal account address");
        return Err(ProgramError::InvalidArgument);
    }

    let proposal = RoleProposal::<F>::load(transfer_accounts.proposal_account)?;
    if !proposal.roles.contains(inputs.roles) {
        msg!("Trying to accept a role that hasn't been proposed");
        return Err(ProgramError::InvalidArgument);
    }

    close_pda(
        transfer_accounts.origin_user_account,
        transfer_accounts.proposal_account,
    )?;

    transfer_roles(
        program_id,
        &transfer_accounts.into(),
        inputs.roles,
        inputs.destination_roles_pda_bump,
    )?;

    Ok(())
}

fn transfer_roles<F: RolesFlags>(
    program_id: &Pubkey,
    accounts: &RoleTransferAccounts<'_>,
    roles: F,
    destination_roles_pda_bump: u8,
) -> ProgramResult {
    ensure_roles(
        program_id,
        accounts.resource,
        accounts.origin_user_account,
        accounts.origin_roles_account,
        roles,
    )?;

    let mut origin_user_roles = UserRoles::load(accounts.origin_roles_account)?;
    origin_user_roles.remove(roles);
    origin_user_roles.store(accounts.origin_roles_account)?;

    if let Ok(mut destination_user_roles) = UserRoles::load(accounts.destination_roles_account) {
        destination_user_roles.add(roles);
        destination_user_roles.store(accounts.destination_roles_account)?;
    } else {
        let signer_seeds = &[
            seed_prefixes::USER_ROLES_SEED,
            accounts.resource.key.as_ref(),
            accounts.destination_user_account.key.as_ref(),
            &[destination_roles_pda_bump],
        ];

        UserRoles::new(roles, destination_roles_pda_bump).init(
            program_id,
            accounts.system_account,
            accounts.payer,
            accounts.destination_roles_account,
            signer_seeds,
        )?;
    }
    Ok(())
}

/// Transfer roles from one user to another.
///
/// # Errors
///
/// [`ProgramError`] is returned as a result of failed operations.
pub fn transfer<F: RolesFlags>(
    program_id: &Pubkey,
    accounts: RoleManagementAccounts<'_>,
    inputs: &RoleManagementInstructionInputs<F>,
    required_payer_roles: F,
) -> ProgramResult {
    let transfer_accounts = RoleTransferAccounts::try_from(accounts)?;

    ensure_signer_roles(
        program_id,
        transfer_accounts.resource,
        transfer_accounts.payer,
        transfer_accounts.payer_roles_account,
        required_payer_roles,
    )?;

    transfer_roles(
        program_id,
        &transfer_accounts,
        inputs.roles,
        inputs.destination_roles_pda_bump,
    )?;

    Ok(())
}

/// Add roles to a user.
///
/// # Errors
///
/// [`ProgramError`] is returned as a result of failed operations.
pub fn add<F: RolesFlags>(
    program_id: &Pubkey,
    accounts: RoleManagementAccounts<'_>,
    inputs: &RoleManagementInstructionInputs<F>,
    required_payer_roles: F,
) -> ProgramResult {
    let add_accounts = RoleAddAccounts::try_from(accounts)?;

    ensure_signer_roles(
        program_id,
        add_accounts.resource,
        add_accounts.payer,
        add_accounts.payer_roles_account,
        required_payer_roles,
    )?;

    if let Ok(mut destination_user_roles) = UserRoles::load(add_accounts.destination_roles_account)
    {
        destination_user_roles.add(inputs.roles);
        destination_user_roles.store(add_accounts.destination_roles_account)?;
    } else {
        let signer_seeds = &[
            seed_prefixes::USER_ROLES_SEED,
            add_accounts.resource.key.as_ref(),
            add_accounts.destination_user_account.key.as_ref(),
            &[inputs.destination_roles_pda_bump],
        ];

        UserRoles::new(inputs.roles, inputs.destination_roles_pda_bump).init(
            program_id,
            add_accounts.system_account,
            add_accounts.payer,
            add_accounts.destination_roles_account,
            signer_seeds,
        )?;
    }

    Ok(())
}

/// Remove roles from a user.
///
/// # Errors
///
/// [`ProgramError`] is returned as a result of failed operations.
pub fn remove<F: RolesFlags>(
    program_id: &Pubkey,
    accounts: RoleManagementAccounts<'_>,
    inputs: &RoleManagementInstructionInputs<F>,
    required_payer_roles: F,
) -> ProgramResult {
    let remove_accounts = RoleRemoveAccounds::try_from(accounts)?;
    ensure_signer_roles(
        program_id,
        remove_accounts.resource,
        remove_accounts.payer,
        remove_accounts.payer_roles_account,
        required_payer_roles,
    )?;

    if let Ok(mut destination_user_roles) =
        UserRoles::load(remove_accounts.destination_roles_account)
    {
        destination_user_roles.remove(inputs.roles);
        destination_user_roles.store(remove_accounts.destination_roles_account)?;
    } else {
        msg!("Trying to remove roles from a user that doesn't have any");
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}

/// Ensure a user has the required roles on a resource.
///
/// # Errors
///
/// [`ProgramError`] is returned as a result of failed operations.
pub fn ensure_roles<F: RolesFlags>(
    program_id: &Pubkey,
    resource: &AccountInfo<'_>,
    user: &AccountInfo<'_>,
    roles_account: &AccountInfo<'_>,
    roles: F,
) -> ProgramResult {
    let Ok(user_roles) = UserRoles::load(roles_account) else {
        if roles.eq(&F::empty()) {
            return Ok(());
        }

        msg!("User roles account not found");
        return Err(ProgramError::InvalidArgument);
    };

    if !user_roles.contains(roles) {
        msg!("User doesn't have the required roles");
        return Err(ProgramError::InvalidArgument);
    }
    let (derived_pda, _) =
        crate::create_user_roles_pda(program_id, resource.key, user.key, user_roles.bump());

    if *roles_account.key != derived_pda {
        msg!("Derived PDA doesn't match given roles account address");
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}

/// Ensure a user has the required roles on a resource and is a signer.
///
/// # Errors
///
/// [`ProgramError`] is returned as a result of failed operations.
pub fn ensure_signer_roles<F: RolesFlags>(
    program_id: &Pubkey,
    resource: &AccountInfo<'_>,
    signer: &AccountInfo<'_>,
    roles_account: &AccountInfo<'_>,
    roles: F,
) -> ProgramResult {
    if !signer.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    ensure_roles(program_id, resource, signer, roles_account, roles)
}

/// Ensure the given account is the upgrade authority of the program.
///
/// This is the Solana equivalent of a contract owner.
///
/// # Errors
///
/// If a deserilization error occurs, the program account is invalid, or the given account is not the owner of the program.
pub fn ensure_upgrade_authority(
    program_id: &Pubkey,
    authority: &AccountInfo<'_>,
    program_data: &AccountInfo<'_>,
) -> ProgramResult {
    if !authority.is_signer {
        msg!("Authority must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (program_account_key, _) =
        Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::id());

    if program_data.key.ne(&program_account_key) {
        return Err(ProgramError::InvalidAccountData);
    }

    let program_data = program_data.try_borrow_data()?;
    let Some(program_bytes) =
        program_data.get(0..UpgradeableLoaderState::size_of_programdata_metadata())
    else {
        return Err(ProgramError::InvalidAccountData);
    };

    let loader_state =
        bincode::deserialize::<UpgradeableLoaderState>(program_bytes).map_err(|err| {
            msg!("UpgradeableLoaderState deserialization error: {:?}", err);
            ProgramError::InvalidAccountData
        })?;

    let UpgradeableLoaderState::ProgramData {
        upgrade_authority_address: Some(upgrade_authority_address),
        ..
    } = loader_state
    else {
        msg!("Unable to get upgrade authority address. Program data is invalid");
        return Err(ProgramError::InvalidAccountData);
    };

    if upgrade_authority_address.ne(authority.key) {
        msg!("Given authority is not the program upgrade authority");
        return Err(ProgramError::InvalidAccountOwner);
    }

    Ok(())
}

/// Accounts used by role management instructions.
pub struct RoleManagementAccounts<'a> {
    /// System account.
    pub system_account: &'a AccountInfo<'a>,

    /// Payer account.
    pub payer: &'a AccountInfo<'a>,

    /// Payer roles account.
    pub payer_roles_account: &'a AccountInfo<'a>,

    /// Resource account.
    pub resource: &'a AccountInfo<'a>,

    /// Destination user account.
    pub destination_user_account: Option<&'a AccountInfo<'a>>,

    /// Destination roles account.
    pub destination_roles_account: Option<&'a AccountInfo<'a>>,

    /// Origin user account.
    pub origin_user_account: Option<&'a AccountInfo<'a>>,

    /// Origin roles account.
    pub origin_roles_account: Option<&'a AccountInfo<'a>>,

    /// Proposal account.
    pub proposal_account: Option<&'a AccountInfo<'a>>,
}

impl<'a> TryFrom<&'a [AccountInfo<'a>]> for RoleManagementAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a [AccountInfo<'a>]) -> Result<Self, Self::Error> {
        let account_iter = &mut value.iter();
        Ok(Self {
            system_account: next_account_info(account_iter)?,
            payer: next_account_info(account_iter)?,
            payer_roles_account: next_account_info(account_iter)?,
            resource: next_account_info(account_iter)?,
            destination_user_account: next_account_info(account_iter).ok(),
            destination_roles_account: next_account_info(account_iter).ok(),
            origin_user_account: next_account_info(account_iter).ok(),
            origin_roles_account: next_account_info(account_iter).ok(),
            proposal_account: next_account_info(account_iter).ok(),
        })
    }
}

pub(crate) struct RoleTransferAccounts<'a> {
    system_account: &'a AccountInfo<'a>,
    payer: &'a AccountInfo<'a>,
    payer_roles_account: &'a AccountInfo<'a>,
    resource: &'a AccountInfo<'a>,
    destination_user_account: &'a AccountInfo<'a>,
    destination_roles_account: &'a AccountInfo<'a>,
    origin_user_account: &'a AccountInfo<'a>,
    origin_roles_account: &'a AccountInfo<'a>,
}

impl<'a> TryFrom<RoleManagementAccounts<'a>> for RoleTransferAccounts<'a> {
    type Error = ProgramError;
    fn try_from(value: RoleManagementAccounts<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            system_account: value.system_account,
            payer: value.payer,
            payer_roles_account: value.payer_roles_account,
            resource: value.resource,
            destination_user_account: value
                .destination_user_account
                .ok_or(ProgramError::InvalidArgument)?,
            destination_roles_account: value
                .destination_roles_account
                .ok_or(ProgramError::InvalidArgument)?,
            origin_user_account: value
                .origin_user_account
                .ok_or(ProgramError::InvalidArgument)?,
            origin_roles_account: value
                .origin_roles_account
                .ok_or(ProgramError::InvalidArgument)?,
        })
    }
}

pub(crate) struct RoleTransferWithProposalAccounts<'a> {
    system_account: &'a AccountInfo<'a>,
    payer: &'a AccountInfo<'a>,
    payer_roles_account: &'a AccountInfo<'a>,
    resource: &'a AccountInfo<'a>,
    destination_user_account: &'a AccountInfo<'a>,
    destination_roles_account: &'a AccountInfo<'a>,
    origin_user_account: &'a AccountInfo<'a>,
    origin_roles_account: &'a AccountInfo<'a>,
    proposal_account: &'a AccountInfo<'a>,
}

impl<'a> TryFrom<RoleManagementAccounts<'a>> for RoleTransferWithProposalAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: RoleManagementAccounts<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            system_account: value.system_account,
            payer: value.payer,
            payer_roles_account: value.payer_roles_account,
            resource: value.resource,
            destination_user_account: value
                .destination_user_account
                .ok_or(ProgramError::InvalidArgument)?,
            destination_roles_account: value
                .destination_roles_account
                .ok_or(ProgramError::InvalidArgument)?,
            origin_user_account: value
                .origin_user_account
                .ok_or(ProgramError::InvalidArgument)?,
            origin_roles_account: value
                .origin_roles_account
                .ok_or(ProgramError::InvalidArgument)?,
            proposal_account: value
                .proposal_account
                .ok_or(ProgramError::InvalidArgument)?,
        })
    }
}

impl<'a> From<RoleTransferWithProposalAccounts<'a>> for RoleTransferAccounts<'a> {
    fn from(value: RoleTransferWithProposalAccounts<'a>) -> Self {
        Self {
            system_account: value.system_account,
            payer: value.payer,
            payer_roles_account: value.payer_roles_account,
            resource: value.resource,
            destination_user_account: value.destination_user_account,
            destination_roles_account: value.destination_roles_account,
            origin_user_account: value.origin_user_account,
            origin_roles_account: value.origin_roles_account,
        }
    }
}

pub(crate) struct RoleAddAccounts<'a> {
    system_account: &'a AccountInfo<'a>,
    payer: &'a AccountInfo<'a>,
    payer_roles_account: &'a AccountInfo<'a>,
    resource: &'a AccountInfo<'a>,
    destination_user_account: &'a AccountInfo<'a>,
    destination_roles_account: &'a AccountInfo<'a>,
}

impl<'a> TryFrom<RoleManagementAccounts<'a>> for RoleAddAccounts<'a> {
    type Error = ProgramError;
    fn try_from(value: RoleManagementAccounts<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            system_account: value.system_account,
            payer: value.payer,
            payer_roles_account: value.payer_roles_account,
            resource: value.resource,
            destination_user_account: value
                .destination_user_account
                .ok_or(ProgramError::InvalidArgument)?,
            destination_roles_account: value
                .destination_roles_account
                .ok_or(ProgramError::InvalidArgument)?,
        })
    }
}

pub(crate) type RoleRemoveAccounds<'a> = RoleAddAccounts<'a>;
