//! This module provides logic to handle user role management instructions.
use program_utils::pda::{close_pda, BorshPda};
use solana_program::account_info::AccountInfo;
use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{bpf_loader_upgradeable, msg};

use crate::seed_prefixes;
use crate::state::{RoleProposal, RolesFlags, UserRoles};

/// Propose a role transfer from one user to another.
///
/// # Errors
///
/// [`ProgramError`] is returned as a result of failed operations.
pub fn propose<F: RolesFlags>(
    program_id: &Pubkey,
    accounts: RoleTransferWithProposalAccounts<'_>,
    roles: F,
) -> ProgramResult {
    if accounts.origin_user_account.key == accounts.destination_user_account.key {
        msg!("Source and destination accounts are the same");
        return Err(ProgramError::InvalidArgument);
    }

    ensure_signer_roles(
        program_id,
        accounts.resource,
        accounts.payer,
        accounts.payer_roles_account,
        roles,
    )?;

    ensure_roles(
        program_id,
        accounts.resource,
        accounts.origin_user_account,
        accounts.origin_roles_account,
        roles,
    )?;

    ensure_proper_account::<F>(
        program_id,
        accounts.resource,
        accounts.destination_user_account,
        accounts.destination_roles_account,
    )?;

    let (proposal_pda, proposal_pda_bump) = crate::find_roles_proposal_pda(
        program_id,
        accounts.resource.key,
        accounts.origin_user_account.key,
        accounts.destination_user_account.key,
    );

    if proposal_pda != *accounts.proposal_account.key {
        msg!("Derived PDA doesn't match given proposal account address");
        return Err(ProgramError::InvalidArgument);
    }

    let proposal = RoleProposal {
        roles,
        bump: proposal_pda_bump,
    };

    proposal.init(
        program_id,
        accounts.system_account,
        accounts.payer,
        accounts.proposal_account,
        &[
            seed_prefixes::ROLE_PROPOSAL_SEED,
            accounts.resource.key.as_ref(),
            accounts.origin_user_account.key.as_ref(),
            accounts.destination_user_account.key.as_ref(),
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
    accounts: RoleTransferWithProposalAccounts<'_>,
    roles: F,
) -> ProgramResult {
    let proposal_pda_bump = RoleProposal::<F>::load(accounts.proposal_account)?.bump;
    let (derived_proposal_pda, _) = crate::create_roles_proposal_pda(
        program_id,
        accounts.resource.key,
        accounts.origin_user_account.key,
        accounts.destination_user_account.key,
        proposal_pda_bump,
    );

    if derived_proposal_pda != *accounts.proposal_account.key {
        msg!("Derived PDA doesn't match given  proposal account address");
        return Err(ProgramError::InvalidArgument);
    }

    let proposal = RoleProposal::<F>::load(accounts.proposal_account)?;
    if !proposal.roles.contains(roles) {
        msg!("Trying to accept a role that hasn't been proposed");
        return Err(ProgramError::InvalidArgument);
    }

    let proposal_account = accounts.proposal_account;
    let role_remove_accounts = RoleRemoveAccounts::from(accounts);
    let role_add_accounts = RoleAddAccounts::from(accounts);

    add(program_id, role_add_accounts, roles, F::empty())?;
    remove(program_id, role_remove_accounts, roles, F::empty())?;

    close_pda(accounts.origin_user_account, proposal_account, program_id)?;

    Ok(())
}

/// Add roles to a user.
///
/// # Errors
///
/// [`ProgramError`] is returned as a result of failed operations.
pub fn add<F: RolesFlags>(
    program_id: &Pubkey,
    accounts: RoleAddAccounts<'_>,
    roles: F,
    required_payer_roles: F,
) -> ProgramResult {
    ensure_signer_roles(
        program_id,
        accounts.resource,
        accounts.payer,
        accounts.payer_roles_account,
        required_payer_roles,
    )?;

    ensure_proper_account::<F>(
        program_id,
        accounts.resource,
        accounts.destination_user_account,
        accounts.destination_roles_account,
    )?;

    if let Ok(mut destination_user_roles) = UserRoles::load(accounts.destination_roles_account) {
        destination_user_roles.add(roles);
        destination_user_roles.store(
            accounts.payer,
            accounts.destination_roles_account,
            accounts.system_account,
        )?;
    } else {
        let (destination_roles_pda, destination_roles_pda_bump) = crate::find_user_roles_pda(
            program_id,
            accounts.resource.key,
            accounts.destination_user_account.key,
        );

        if destination_roles_pda != *accounts.destination_roles_account.key {
            msg!("Derived PDA doesn't match given destination roles account address");
            return Err(ProgramError::InvalidArgument);
        }

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

/// Remove roles from a user.
///
/// # Errors
///
/// [`ProgramError`] is returned as a result of failed operations.
pub fn remove<F: RolesFlags>(
    program_id: &Pubkey,
    accounts: RoleRemoveAccounts<'_>,
    roles: F,
    required_payer_roles: F,
) -> ProgramResult {
    ensure_signer_roles(
        program_id,
        accounts.resource,
        accounts.payer,
        accounts.payer_roles_account,
        required_payer_roles,
    )?;

    ensure_roles(
        program_id,
        accounts.resource,
        accounts.origin_user_account,
        accounts.origin_roles_account,
        roles,
    )?;

    if let Ok(mut destination_user_roles) = UserRoles::load(accounts.origin_roles_account) {
        destination_user_roles.remove(roles);
        destination_user_roles.store(
            accounts.payer,
            accounts.origin_roles_account,
            accounts.system_account,
        )?;
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

    ensure_proper_account::<F>(program_id, resource, user, roles_account)?;

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

    let program_account_key = bpf_loader_upgradeable::get_program_data_address(program_id);

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

/// Ensure the account passed is a role account for the given user and resource.
///
/// # Errors
///
/// If the PDA derived from the user and resource keys is different than the passed role account
/// key.
pub fn ensure_proper_account<F: RolesFlags>(
    program_id: &Pubkey,
    resource: &AccountInfo<'_>,
    user: &AccountInfo<'_>,
    user_roles: &AccountInfo<'_>,
) -> ProgramResult {
    let (derived_pda, _) = crate::user_roles_pda(
        program_id,
        resource.key,
        user.key,
        UserRoles::<F>::load(user_roles).ok().map(|r| r.bump()),
    );

    if *user_roles.key != derived_pda {
        msg!("Derived PDA doesn't match given roles account address");
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
pub struct RoleTransferWithProposalAccounts<'a> {
    pub system_account: &'a AccountInfo<'a>,
    pub payer: &'a AccountInfo<'a>,
    pub payer_roles_account: &'a AccountInfo<'a>,
    pub resource: &'a AccountInfo<'a>,
    pub destination_user_account: &'a AccountInfo<'a>,
    pub destination_roles_account: &'a AccountInfo<'a>,
    pub origin_user_account: &'a AccountInfo<'a>,
    pub origin_roles_account: &'a AccountInfo<'a>,
    pub proposal_account: &'a AccountInfo<'a>,
}

impl<'a> From<RoleTransferWithProposalAccounts<'a>> for RoleRemoveAccounts<'a> {
    fn from(value: RoleTransferWithProposalAccounts<'a>) -> Self {
        Self {
            system_account: value.system_account,
            payer: value.payer,
            payer_roles_account: value.payer_roles_account,
            resource: value.resource,
            origin_user_account: value.origin_user_account,
            origin_roles_account: value.origin_roles_account,
        }
    }
}

impl<'a> From<RoleTransferWithProposalAccounts<'a>> for RoleAddAccounts<'a> {
    fn from(value: RoleTransferWithProposalAccounts<'a>) -> Self {
        Self {
            system_account: value.system_account,
            payer: value.payer,
            payer_roles_account: value.payer_roles_account,
            resource: value.resource,
            destination_user_account: value.destination_user_account,
            destination_roles_account: value.destination_roles_account,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RoleAddAccounts<'a> {
    pub system_account: &'a AccountInfo<'a>,
    pub payer: &'a AccountInfo<'a>,
    pub payer_roles_account: &'a AccountInfo<'a>,
    pub resource: &'a AccountInfo<'a>,
    pub destination_user_account: &'a AccountInfo<'a>,
    pub destination_roles_account: &'a AccountInfo<'a>,
}

#[derive(Debug, Clone, Copy)]
pub struct RoleRemoveAccounts<'a> {
    pub system_account: &'a AccountInfo<'a>,
    pub payer: &'a AccountInfo<'a>,
    pub payer_roles_account: &'a AccountInfo<'a>,
    pub resource: &'a AccountInfo<'a>,
    pub origin_user_account: &'a AccountInfo<'a>,
    pub origin_roles_account: &'a AccountInfo<'a>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use bitflags::bitflags;
    use core::ops::Not;
    use solana_program::{
        bpf_loader_upgradeable::{self, UpgradeableLoaderState},
        pubkey::Pubkey,
    };
    use solana_sdk::account::Account;

    bitflags! {
        /// Possible input variations for the function being tested.
        ///
        /// A set bit means that input bit has valid content.
        pub struct TestCase: u8 {
            const UPGRADE_AUTHORITY    = 0b0001;
            const PROGRAM_DATA_PUBKEY  = 0b0010;
            const PROGRAM_DATA_ACCOUNT = 0b0100;
            const SIGNED              = 0b1000;
            const ALL_VALID           = 0b1111;
        }
    }

    /// Helper struct to hold test fixtures.
    ///
    /// Necessary because `AccountInfo` must hold mutable references to the underlying
    /// data, and correctness depends on all fields being consonant.
    struct TestContext {
        program_id: Pubkey,
        program_data_account_key: Pubkey,
        program_data_account: Account,
        authority_pubkey: Pubkey,
        authority_account: Account,
    }

    impl TestContext {
        /// Creates a test context filled with valid data.
        fn new() -> Self {
            let program_id = Pubkey::new_unique();
            let authority_pubkey = Pubkey::new_unique();
            let program_data_account_key =
                bpf_loader_upgradeable::get_program_data_address(&program_id);
            let program_data_account = Account::new_data(
                0,
                &UpgradeableLoaderState::ProgramData {
                    slot: 0,
                    upgrade_authority_address: Some(authority_pubkey),
                },
                &bpf_loader_upgradeable::id(),
            )
            .unwrap();
            let authority_account = Account::new(0, 0, &authority_pubkey);

            Self {
                program_id,
                program_data_account_key,
                program_data_account,
                authority_pubkey,
                authority_account,
            }
        }

        /// Generates a random account w/ random data
        fn random_account() -> Account {
            Account::new_data(0, &rand::random::<[u8; 32]>(), &Pubkey::new_unique()).unwrap()
        }

        /// Prepares inputs — valid and invalid — based on the given test case and calls
        /// `ensure_upgrade_authority` with them.
        ///
        /// Requires a mutable borrow to self because `AccountInfo` holds mutable
        /// references to the underlying data.
        #[track_caller]
        fn call(&mut self, test_case: &TestCase) -> ProgramResult {
            let program_data_address = test_case
                .contains(TestCase::PROGRAM_DATA_PUBKEY)
                .then_some(self.program_data_account_key)
                .unwrap_or_else(Pubkey::new_unique);

            let authority_pubkey = test_case
                .contains(TestCase::UPGRADE_AUTHORITY)
                .then_some(self.authority_pubkey)
                .unwrap_or_else(Pubkey::new_unique);

            let authority_account_info: AccountInfo<'_> = (
                &authority_pubkey,
                test_case.contains(TestCase::SIGNED),
                &mut self.authority_account,
            )
                .into();

            let mut program_data_account = test_case
                .contains(TestCase::PROGRAM_DATA_ACCOUNT)
                .then(|| self.program_data_account.clone())
                .unwrap_or_else(Self::random_account);

            let program_data_account_info =
                (&program_data_address, &mut program_data_account).into();

            ensure_upgrade_authority(
                &self.program_id,
                &authority_account_info,
                &program_data_account_info,
            )
        }
    }

    #[test]
    fn test_ensure_upgrade_authority_function() {
        let mut ctx = TestContext::new();

        // Valid upgrade authority
        assert!(ctx.call(&TestCase::ALL_VALID).is_ok());

        // Unsigned authority
        assert_eq!(
            ctx.call(&TestCase::SIGNED.not()).unwrap_err(),
            ProgramError::MissingRequiredSignature
        );

        // Invalid program data account address
        assert_eq!(
            ctx.call(&TestCase::PROGRAM_DATA_PUBKEY.not()).unwrap_err(),
            ProgramError::InvalidAccountData
        );

        // Invalid program data account data
        assert_eq!(
            ctx.call(&TestCase::PROGRAM_DATA_ACCOUNT.not()).unwrap_err(),
            ProgramError::InvalidAccountData
        );

        // Invalid authority pubkey
        assert_eq!(
            ctx.call(&TestCase::UPGRADE_AUTHORITY.not()).unwrap_err(),
            ProgramError::InvalidAccountOwner
        );

        // Invalid unknowns
        for bits in 0..15 {
            assert!(
                ctx.call(&TestCase::from_bits(bits).unwrap()).is_err(),
                "Invalid result for bit pattern {bits}",
            );
        }
    }
}
