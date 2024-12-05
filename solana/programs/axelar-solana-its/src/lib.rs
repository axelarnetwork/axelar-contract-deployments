//! # `InterchainTokenService` program
use bitflags::bitflags;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program::invoke;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

mod entrypoint;
pub mod executable;
pub mod instructions;
pub mod processor;
pub mod state;

solana_program::declare_id!("itswMJtRUe2vd46rb5kDmYzfBHHej4PyX4twgnbT1TG");

pub(crate) trait FromAccountInfoSlice<'a> {
    type Context;

    fn from_account_info_slice(
        accounts: &'a [AccountInfo<'a>],
        context: &Self::Context,
    ) -> Result<Self, ProgramError>
    where
        Self: Sized;
}

/// Seed prefixes for different PDAs initialized by the program
pub mod seed_prefixes {
    /// The seed prefix for deriving the ITS root PDA
    pub const ITS_SEED: &[u8] = b"interchain-token-service";

    /// The seed prefix for deriving the token manager PDA
    pub const TOKEN_MANAGER_SEED: &[u8] = b"token-manager";

    /// The seed prefix for deriving the interchain token PDA
    pub const INTERCHAIN_TOKEN_SEED: &[u8] = b"interchain-token";

    /// The seed prefix for deriving the flow slot PDA
    pub const FLOW_SLOT_SEED: &[u8] = b"flow-slot";
}

bitflags! {
    /// Roles that can be assigned to a user.
    #[derive(Debug, Eq, PartialEq, Clone, Copy)]
    pub struct Roles: u8 {
        /// Can mint new tokens.
        const MINTER = 0b0000_0001;

        /// Can perform operations on the resource.
        const OPERATOR = 0b0000_0010;

        /// Can change the limit to the flow of tokens.
        const FLOW_LIMITER = 0b0000_0100;
    }
}

impl PartialEq<u8> for Roles {
    fn eq(&self, other: &u8) -> bool {
        self.bits().eq(other)
    }
}

impl PartialEq<Roles> for u8 {
    fn eq(&self, other: &Roles) -> bool {
        self.eq(&other.bits())
    }
}

/// Checks that the supplied program ID is the correct one
///
/// # Errors
///
/// If the program ID passed doesn't match the current program ID
#[inline]
pub fn check_program_account(program_id: Pubkey) -> ProgramResult {
    if program_id != crate::ID {
        return Err(ProgramError::IncorrectProgramId);
    }

    Ok(())
}

/// Checks that the supplied program ID is the correct one
///
/// # Errors
///
/// If the bump cannot be used to derive the correct PDA
pub fn check_initialization_bump(
    bump: u8,
    its_pda: &Pubkey,
    gateway_root_pda: &Pubkey,
) -> ProgramResult {
    let derived = Pubkey::create_program_address(
        &[seed_prefixes::ITS_SEED, gateway_root_pda.as_ref(), &[bump]],
        &crate::ID,
    )?;

    if derived != *its_pda {
        msg!("Derived PDA does not match expected PDA");
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

/// Tries to create the ITS root PDA using the provided bump, falling back to
/// `find_program_address` if the bump is `None` or invalid.
#[must_use]
pub fn its_root_pda(gateway_root_pda: &Pubkey, maybe_bump: Option<u8>) -> (Pubkey, u8) {
    maybe_bump
        .and_then(|bump| {
            Pubkey::create_program_address(
                &[seed_prefixes::ITS_SEED, gateway_root_pda.as_ref(), &[bump]],
                &crate::id(),
            )
            .map(|pubkey| (pubkey, bump))
            .ok()
        })
        .unwrap_or_else(|| {
            Pubkey::find_program_address(
                &[seed_prefixes::ITS_SEED, gateway_root_pda.as_ref()],
                &crate::id(),
            )
        })
}

/// Tries to create the ITS root PDA using the provided bump, falling back to
/// `find_program_address` if the bump invalid.
#[inline]
#[must_use]
pub fn create_its_root_pda(gateway_root_pda: &Pubkey, bump: u8) -> (Pubkey, u8) {
    its_root_pda(gateway_root_pda, Some(bump))
}

/// Derives interchain token service root PDA
#[inline]
#[must_use]
pub fn find_its_root_pda(gateway_root_pda: &Pubkey) -> (Pubkey, u8) {
    its_root_pda(gateway_root_pda, None)
}

/// Tries to create the PDA for a [`Tokenmanager`] using the provided bump,
/// falling back to `find_program_address` if the bump is `None` or invalid.
#[must_use]
pub fn token_manager_pda(interchain_token_pda: &Pubkey, maybe_bump: Option<u8>) -> (Pubkey, u8) {
    maybe_bump
        .and_then(|bump| {
            Pubkey::create_program_address(
                &[
                    seed_prefixes::TOKEN_MANAGER_SEED,
                    interchain_token_pda.as_ref(),
                    &[bump],
                ],
                &crate::id(),
            )
            .map(|pubkey| (pubkey, bump))
            .ok()
        })
        .unwrap_or_else(|| {
            Pubkey::find_program_address(
                &[
                    seed_prefixes::TOKEN_MANAGER_SEED,
                    interchain_token_pda.as_ref(),
                ],
                &crate::id(),
            )
        })
}

/// Tries to create the PDA for a [`Tokenmanager`] using the provided bump,
/// falling back to `find_program_address` if the bump is invalid.
#[inline]
#[must_use]
pub fn create_token_manager_pda(interchain_token_pda: &Pubkey, bump: u8) -> (Pubkey, u8) {
    token_manager_pda(interchain_token_pda, Some(bump))
}

/// Derives the PDA for a [`TokenManager`].
#[inline]
#[must_use]
pub fn find_token_manager_pda(interchain_token_pda: &Pubkey) -> (Pubkey, u8) {
    token_manager_pda(interchain_token_pda, None)
}

/// Tries to create the PDA for an `InterchainToken` using the provided bump,
/// falling back to `find_program_address` if the bump is `None` or invalid.
#[must_use]
pub fn interchain_token_pda(
    its_root_pda: &Pubkey,
    token_id: &[u8],
    maybe_bump: Option<u8>,
) -> (Pubkey, u8) {
    maybe_bump
        .and_then(|bump| {
            Pubkey::create_program_address(
                &[
                    seed_prefixes::INTERCHAIN_TOKEN_SEED,
                    its_root_pda.as_ref(),
                    token_id,
                    &[bump],
                ],
                &crate::id(),
            )
            .map(|pubkey| (pubkey, bump))
            .ok()
        })
        .unwrap_or_else(|| {
            Pubkey::find_program_address(
                &[
                    seed_prefixes::INTERCHAIN_TOKEN_SEED,
                    its_root_pda.as_ref(),
                    token_id,
                ],
                &crate::id(),
            )
        })
}

/// Tries to create the PDA for a `FlowSlot` using the provided bump,
/// falling back to `find_program_address` if the bump is invalid.
#[inline]
#[must_use]
pub fn create_flow_slot_pda(token_manager_pda: &Pubkey, epoch: u64, bump: u8) -> (Pubkey, u8) {
    flow_slot_pda(token_manager_pda, epoch, Some(bump))
}

/// Derives the PDA for a `FlowSlot`.
#[inline]
#[must_use]
pub fn find_flow_slot_pda(token_manager_pda: &Pubkey, epoch: u64) -> (Pubkey, u8) {
    flow_slot_pda(token_manager_pda, epoch, None)
}

/// Tries to create the PDA for a `FlowSlot` using the provided bump,
/// falling back to `find_program_address` if the bump is `None` or invalid.
#[must_use]
pub fn flow_slot_pda(
    token_manager_pda: &Pubkey,
    epoch: u64,
    maybe_bump: Option<u8>,
) -> (Pubkey, u8) {
    maybe_bump
        .and_then(|bump| {
            Pubkey::create_program_address(
                &[
                    seed_prefixes::FLOW_SLOT_SEED,
                    token_manager_pda.as_ref(),
                    &epoch.to_ne_bytes(),
                    &[bump],
                ],
                &crate::id(),
            )
            .map(|pubkey| (pubkey, bump))
            .ok()
        })
        .unwrap_or_else(|| {
            Pubkey::find_program_address(
                &[
                    seed_prefixes::FLOW_SLOT_SEED,
                    token_manager_pda.as_ref(),
                    &epoch.to_ne_bytes(),
                ],
                &crate::id(),
            )
        })
}

/// Tries to create the PDA for an `InterchainToken` using the provided bump,
/// falling back to `find_program_address` if the bump is invalid.
#[inline]
#[must_use]
pub fn create_interchain_token_pda(
    its_root_pda: &Pubkey,
    token_id: &[u8],
    bump: u8,
) -> (Pubkey, u8) {
    interchain_token_pda(its_root_pda, token_id, Some(bump))
}

/// Derives the PDA for an interchain token account.
#[inline]
#[must_use]
pub fn find_interchain_token_pda(its_root_pda: &Pubkey, token_id: &[u8]) -> (Pubkey, u8) {
    interchain_token_pda(its_root_pda, token_id, None)
}

/// Creates an associated token account for the given wallet address and token
/// mint.
///
/// # Errors
///
/// Returns an error if the account already exists.
pub(crate) fn create_associated_token_account<'a>(
    payer: &AccountInfo<'a>,
    token_mint_account: &AccountInfo<'a>,
    associated_token_account: &AccountInfo<'a>,
    wallet: &AccountInfo<'a>,
    system_account: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
) -> ProgramResult {
    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        payer.key,
        wallet.key,
        token_mint_account.key,
        token_program.key,
    );

    invoke(
        &create_ata_ix,
        &[
            payer.clone(),
            associated_token_account.clone(),
            wallet.clone(),
            token_mint_account.clone(),
            system_account.clone(),
            token_program.clone(),
        ],
    )?;

    Ok(())
}

/// Creates an associated token account for the given wallet address and token
/// mint, if it doesn't already exist.
///
/// # Errors
///
/// Returns an error if the account already exists, but with a different owner.
pub(crate) fn create_associated_token_account_idempotent<'a>(
    payer: &AccountInfo<'a>,
    token_mint_account: &AccountInfo<'a>,
    associated_token_account: &AccountInfo<'a>,
    wallet: &AccountInfo<'a>,
    system_account: &AccountInfo<'a>,
    token_program: &AccountInfo<'a>,
) -> ProgramResult {
    let create_ata_ix =
        spl_associated_token_account::instruction::create_associated_token_account_idempotent(
            payer.key,
            wallet.key,
            token_mint_account.key,
            token_program.key,
        );

    invoke(
        &create_ata_ix,
        &[
            payer.clone(),
            associated_token_account.clone(),
            wallet.clone(),
            token_mint_account.clone(),
            system_account.clone(),
            token_program.clone(),
        ],
    )?;

    Ok(())
}

/// Calculates the tokenId that would correspond to a link for a given deployer
/// with a specified salt
#[must_use]
pub fn interchain_token_id(deployer: &Pubkey, salt: &[u8]) -> [u8; 32] {
    solana_program::keccak::hashv(&[deployer.as_ref(), salt]).to_bytes()
}
