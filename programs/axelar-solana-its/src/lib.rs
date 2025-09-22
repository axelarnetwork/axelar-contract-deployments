//! # `InterchainTokenService` program
use bitflags::bitflags;
use borsh::{BorshDeserialize, BorshSerialize};
use program_utils::ensure_single_feature;
use program_utils::pda::BorshPda;
use program_utils::pda::ValidPDA;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program::invoke;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use state::interchain_transfer_execute::InterchainTransferExecute;
use state::InterchainTokenService;

mod entrypoint;
pub mod event;
pub mod executable;
pub mod instruction;
pub mod processor;
pub mod state;

ensure_single_feature!("devnet-amplifier", "stagenet", "testnet", "mainnet");

#[cfg(feature = "devnet-amplifier")]
solana_program::declare_id!("itsqybuNsChBo3LgVhCWWnTJVJdoVTUJaodmqQcG6z7");

#[cfg(feature = "stagenet")]
solana_program::declare_id!("itsediSVCwwKc6UuxfrsEiF8AEuEFk34RFAscPEDEpJ");

#[cfg(feature = "testnet")]
solana_program::declare_id!("itsZEirFsnRmLejCsRRNZKHqWTzMsKGyYi6Qr962os4");

#[cfg(feature = "mainnet")]
solana_program::declare_id!("its1111111111111111111111111111111111111111");

pub(crate) const ITS_HUB_CHAIN_NAME: &str = "axelar";

// Chain name hash constants for token ID derivation
#[cfg(feature = "devnet-amplifier")]
pub const CHAIN_NAME_HASH: [u8; 32] = [
    10, 171, 102, 67, 72, 176, 161, 92, 42, 179, 148, 228, 13, 72, 172, 178, 168, 16, 138, 252, 99,
    222, 187, 187, 25, 30, 121, 52, 235, 103, 11, 169,
]; // keccak256("solana-devnet")

#[cfg(feature = "stagenet")]
pub const CHAIN_NAME_HASH: [u8; 32] = [
    67, 5, 100, 18, 3, 83, 80, 76, 10, 94, 7, 166, 63, 92, 244, 200, 233, 32, 8, 242, 33, 188, 46,
    11, 38, 32, 244, 151, 37, 161, 40, 0,
]; // keccak256("solana-stagenet")

#[cfg(feature = "testnet")]
pub const CHAIN_NAME_HASH: [u8; 32] = [
    159, 1, 245, 195, 103, 184, 207, 215, 88, 74, 183, 125, 33, 47, 221, 82, 55, 77, 255, 177, 89,
    88, 76, 133, 128, 193, 177, 171, 2, 107, 173, 86,
]; // keccak256("solana-testnet")

#[cfg(feature = "mainnet")]
pub const CHAIN_NAME_HASH: [u8; 32] = [
    110, 239, 41, 235, 176, 58, 162, 20, 74, 26, 107, 98, 18, 206, 116, 245, 4, 163, 77, 183, 153,
    184, 22, 26, 33, 20, 0, 23, 232, 13, 61, 138,
]; // keccak256("solana")

pub(crate) trait Validate {
    fn validate(&self) -> Result<(), ProgramError>;
}

pub(crate) trait FromAccountInfoSlice<'a> {
    type Context;

    fn from_account_info_slice(
        accounts: &'a [AccountInfo<'a>],
        context: &Self::Context,
    ) -> Result<Self, ProgramError>
    where
        Self: Sized + Validate,
    {
        let obj = Self::extract_accounts(accounts, context)?;
        obj.validate()?;
        Ok(obj)
    }

    fn extract_accounts(
        accounts: &'a [AccountInfo<'a>],
        context: &Self::Context,
    ) -> Result<Self, ProgramError>
    where
        Self: Sized + Validate;
}

/// Seed prefixes for different PDAs initialized by the program
pub mod seed_prefixes {
    /// The seed prefix for deriving the ITS root PDA
    pub const ITS_SEED: &[u8] = b"interchain-token-service";

    /// The seed prefix for deriving the token manager PDA
    pub const TOKEN_MANAGER_SEED: &[u8] = b"token-manager";

    /// The seed prefix for deriving the interchain token PDA
    pub const INTERCHAIN_TOKEN_SEED: &[u8] = b"interchain-token";

    /// The seed prefix for deriving an interchain token id
    pub const PREFIX_INTERCHAIN_TOKEN_ID: &[u8] = b"interchain-token-id";

    /// The seed prefix for deriving an interchain token salt
    pub const PREFIX_INTERCHAIN_TOKEN_SALT: &[u8] = b"interchain-token-salt";

    /// The seed prefix for deriving an interchain token id for a canonical token
    pub const PREFIX_CANONICAL_TOKEN_SALT: &[u8] = b"canonical-token-salt";

    /// The seed prefix for deriving an interchain token id for a canonical token
    pub const PREFIX_CUSTOM_TOKEN_SALT: &[u8] = b"solana-custom-token-salt";

    /// The seed prefix for deriving the flow slot PDA
    pub const FLOW_SLOT_SEED: &[u8] = b"flow-slot";

    /// The seed prefix for deriving the deployment approval PDA
    pub const DEPLOYMENT_APPROVAL_SEED: &[u8] = b"deployment-approval";

    /// The seed prefix for deriving the interchain transfer execute signing PDA
    pub const INTERCHAIN_TRANSFER_EXECUTE_SEED: &[u8] = b"interchain-transfer-execute";
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

impl BorshSerialize for Roles {
    fn serialize<W: std::io::prelude::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.bits().serialize(writer)
    }
}

impl BorshDeserialize for Roles {
    fn deserialize_reader<R: std::io::prelude::Read>(reader: &mut R) -> std::io::Result<Self> {
        let byte = u8::deserialize_reader(reader)?;
        Ok(Self::from_bits_truncate(byte))
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

/// Tries to create the ITS root PDA using the provided bump, falling back to
/// `find_program_address` if the bump invalid.
///
/// # Errors
///
/// If the bump is invalid.
pub fn create_its_root_pda(bump: u8) -> Result<Pubkey, ProgramError> {
    Ok(Pubkey::create_program_address(
        &[seed_prefixes::ITS_SEED, &[bump]],
        &crate::id(),
    )?)
}

/// Derives interchain token service root PDA
#[inline]
#[must_use]
pub fn find_its_root_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[seed_prefixes::ITS_SEED], &crate::id())
}

pub(crate) fn assert_valid_its_root_pda(
    its_root_pda_account: &AccountInfo<'_>,
    canonical_bump: u8,
) -> ProgramResult {
    let expected_its_root_pda = create_its_root_pda(canonical_bump)?;

    if expected_its_root_pda.ne(its_root_pda_account.key) {
        msg!("Invalid ITS root PDA provided");
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}

pub(crate) fn assert_its_not_paused(its_config: &InterchainTokenService) -> ProgramResult {
    if its_config.paused {
        msg!("The Interchain Token Service is currently paused.");
        return Err(ProgramError::Immutable);
    }

    Ok(())
}

/// Tries to create the PDA for a [`Tokenmanager`] using the provided bump,
/// falling back to `find_program_address` if the bump is invalid.
///
/// # Errors
///
/// If the bump is invalid.
pub fn create_token_manager_pda(
    its_root_pda: &Pubkey,
    token_id: &[u8; 32],
    bump: u8,
) -> Result<Pubkey, ProgramError> {
    Ok(Pubkey::create_program_address(
        &[
            seed_prefixes::TOKEN_MANAGER_SEED,
            its_root_pda.as_ref(),
            token_id,
            &[bump],
        ],
        &crate::id(),
    )?)
}

/// Derives the PDA for a [`TokenManager`].
#[inline]
#[must_use]
pub fn find_token_manager_pda(its_root_pda: &Pubkey, token_id: &[u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            seed_prefixes::TOKEN_MANAGER_SEED,
            its_root_pda.as_ref(),
            token_id,
        ],
        &crate::id(),
    )
}

pub(crate) fn assert_valid_token_manager_pda(
    token_manager_pda_account: &AccountInfo<'_>,
    its_root_pda: &Pubkey,
    token_id: &[u8; 32],
    canonical_bump: u8,
) -> ProgramResult {
    let expected_token_manager_pda =
        create_token_manager_pda(its_root_pda, token_id, canonical_bump)?;
    if expected_token_manager_pda.ne(token_manager_pda_account.key) {
        msg!("Invalid TokenManager PDA provided");
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}

pub(crate) fn assert_valid_deploy_approval_pda(
    deploy_approval_pda_account: &AccountInfo<'_>,
    minter: &Pubkey,
    token_id: &[u8; 32],
    destination_chain: &str,
    canonical_bump: u8,
) -> ProgramResult {
    let expected_deploy_approval_pda =
        create_deployment_approval_pda(minter, token_id, destination_chain, canonical_bump)?;

    if expected_deploy_approval_pda.ne(deploy_approval_pda_account.key) {
        msg!("Invalid DeploymentApproval PDA provided");
        return Err(ProgramError::InvalidArgument);
    }

    Ok(())
}

/// Tries to create the PDA for an `InterchainToken` using the provided bump,
/// falling back to `find_program_address` if the bump is invalid.
///
/// The Interchain Token PDA is used as the mint account for native Interchain Tokens
///
/// # Errors
///
/// If the bump is invalid.
#[inline]
pub fn create_interchain_token_pda(
    its_root_pda: &Pubkey,
    token_id: &[u8],
    bump: u8,
) -> Result<Pubkey, ProgramError> {
    Ok(Pubkey::create_program_address(
        &[
            seed_prefixes::INTERCHAIN_TOKEN_SEED,
            its_root_pda.as_ref(),
            token_id,
            &[bump],
        ],
        &crate::id(),
    )?)
}

/// Derives the PDA for an interchain token account.
///
/// The Interchain Token PDA is used as the mint account for native Interchain Tokens
#[inline]
#[must_use]
pub fn find_interchain_token_pda(its_root_pda: &Pubkey, token_id: &[u8]) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            seed_prefixes::INTERCHAIN_TOKEN_SEED,
            its_root_pda.as_ref(),
            token_id,
        ],
        &crate::id(),
    )
}

/// Tries to create the PDA for a `DeploymentApproval` using the provided bump,
/// falling back to `find_program_address` if the bump is invalid.
///
/// # Errors
///
/// If the bump is invalid.
#[inline]
pub fn create_deployment_approval_pda(
    minter: &Pubkey,
    token_id: &[u8],
    destination_chain: &str,
    bump: u8,
) -> Result<Pubkey, ProgramError> {
    Ok(Pubkey::create_program_address(
        &[
            seed_prefixes::DEPLOYMENT_APPROVAL_SEED,
            minter.as_ref(),
            token_id,
            destination_chain.as_bytes(),
            &[bump],
        ],
        &crate::id(),
    )?)
}

/// Derives the PDA for a `DeploymentApproval`.
#[inline]
#[must_use]
pub fn find_deployment_approval_pda(
    minter: &Pubkey,
    token_id: &[u8],
    destination_chain: &str,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            seed_prefixes::DEPLOYMENT_APPROVAL_SEED,
            minter.as_ref(),
            token_id,
            destination_chain.as_bytes(),
        ],
        &crate::id(),
    )
}

/// Tries to create the PDA for a `DeploymentApproval` using the provided bump,
/// falling back to `find_program_address` if the bump is `None` or invalid.
///
/// # Errors
///
/// If the bump is invalid.
pub fn deployment_approval_pda(
    minter: &Pubkey,
    token_id: &[u8],
    destination_chain: &str,
    maybe_bump: Option<u8>,
) -> Result<(Pubkey, u8), ProgramError> {
    if let Some(bump) = maybe_bump {
        create_deployment_approval_pda(minter, token_id, destination_chain, bump)
            .map(|pubkey| (pubkey, bump))
    } else {
        Ok(find_deployment_approval_pda(
            minter,
            token_id,
            destination_chain,
        ))
    }
}

/// Tries to create the PDA for a [`InterchainTransferExecute`] using the provided bump.
///
/// # Errors
///
/// If the bump is invalid.
pub fn create_interchain_transfer_execute_pda(
    destination_program: &Pubkey,
    bump: u8,
) -> Result<Pubkey, ProgramError> {
    Ok(Pubkey::create_program_address(
        &[
            seed_prefixes::INTERCHAIN_TRANSFER_EXECUTE_SEED,
            &destination_program.to_bytes(),
            &[bump],
        ],
        &crate::id(),
    )?)
}

/// Derives the PDA for a [`InterchainTransferExecute`].
#[inline]
#[must_use]
pub fn find_interchain_transfer_execute_pda(destination_program: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[
            seed_prefixes::INTERCHAIN_TRANSFER_EXECUTE_SEED,
            destination_program.as_ref(),
        ],
        &crate::id(),
    )
}

/// Either create the interchain_transfer_execute pda or read it, and ensure it is derived properly.
pub(crate) fn assert_valid_interchain_transfer_execute_pda<'a>(
    interchain_transfer_execute_pda_account: &AccountInfo<'a>,
    destination_program: &Pubkey,
) -> Result<u8, ProgramError> {
    let bump = if interchain_transfer_execute_pda_account.is_initialized_pda(&crate::id()) {
        let interchain_transfer_execute =
            InterchainTransferExecute::load(interchain_transfer_execute_pda_account)?;

        let expected_token_manager_pda = create_interchain_transfer_execute_pda(
            destination_program,
            interchain_transfer_execute.bump,
        )?;
        if expected_token_manager_pda.ne(interchain_transfer_execute_pda_account.key) {
            msg!("Invalid InterchainTransferExecute PDA provided");
            return Err(ProgramError::InvalidArgument);
        }
        interchain_transfer_execute.bump
    } else {
        let (expected_token_manager_pda, bump) =
            find_interchain_transfer_execute_pda(destination_program);
        if expected_token_manager_pda.ne(interchain_transfer_execute_pda_account.key) {
            msg!("Invalid InterchainTransferExecute PDA provided");
            return Err(ProgramError::InvalidArgument);
        }
        bump
    };

    Ok(bump)
}

/// Either create the interchain_transfer_execute pda or read it, and ensure it is derived properly.
pub(crate) fn initiate_interchain_execute_pda_if_empty<'a>(
    interchain_transfer_execute_pda_account: &AccountInfo<'a>,
    payer: &AccountInfo<'a>,
    system_account: &AccountInfo<'a>,
    destination_program: &Pubkey,
    bump: u8,
) -> Result<(), ProgramError> {
    if !interchain_transfer_execute_pda_account.is_initialized_pda(&crate::id()) {
        let interchain_transfer_execute = InterchainTransferExecute::new(bump);
        interchain_transfer_execute.init(
            &crate::id(),
            system_account,
            payer,
            interchain_transfer_execute_pda_account,
            &[
                seed_prefixes::INTERCHAIN_TRANSFER_EXECUTE_SEED,
                destination_program.as_ref(),
                &[bump],
            ],
        )?;
    };

    Ok(())
}

/// Asserts the given ATA is associated with the given token program, mint and wallet
///
/// # Errors
///
/// If the ATA does not match address derived from mint, owner, and token program.
pub fn assert_valid_ata(
    ata: &Pubkey,
    token_program: &Pubkey,
    mint: &Pubkey,
    owner: &Pubkey,
) -> ProgramResult {
    let associated_account_address =
        spl_associated_token_account::get_associated_token_address_with_program_id(
            owner,
            mint,
            token_program,
        );

    if *ata != associated_account_address {
        msg!("Invalid Associated Token Account");
        return Err(ProgramError::InvalidAccountData);
    }

    Ok(())
}

/// Creates an associated token account for the given program address and token
/// mint, if it doesn't already exist. If it exists, it ensures the wallet is the owner of the
/// given ATA.
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

#[must_use]
pub(crate) fn canonical_interchain_token_deploy_salt(mint: &Pubkey) -> [u8; 32] {
    solana_program::keccak::hashv(&[
        seed_prefixes::PREFIX_CANONICAL_TOKEN_SALT,
        &CHAIN_NAME_HASH,
        mint.as_ref(),
    ])
    .to_bytes()
}

pub(crate) fn interchain_token_deployer_salt(deployer: &Pubkey, salt: &[u8; 32]) -> [u8; 32] {
    solana_program::keccak::hashv(&[
        seed_prefixes::PREFIX_INTERCHAIN_TOKEN_SALT,
        &CHAIN_NAME_HASH,
        deployer.as_ref(),
        salt,
    ])
    .to_bytes()
}

pub(crate) fn linked_token_deployer_salt(deployer: &Pubkey, salt: &[u8; 32]) -> [u8; 32] {
    solana_program::keccak::hashv(&[
        seed_prefixes::PREFIX_CUSTOM_TOKEN_SALT,
        &CHAIN_NAME_HASH,
        deployer.as_ref(),
        salt,
    ])
    .to_bytes()
}

pub(crate) fn interchain_token_id_internal(salt: &[u8; 32]) -> [u8; 32] {
    solana_program::keccak::hashv(&[seed_prefixes::PREFIX_INTERCHAIN_TOKEN_ID, salt]).to_bytes()
}

/// Calculates the tokenId that would correspond to a link for a given deployer
/// with a specified salt
#[must_use]
pub fn interchain_token_id(deployer: &Pubkey, salt: &[u8; 32]) -> [u8; 32] {
    let deploy_salt = interchain_token_deployer_salt(deployer, salt);

    interchain_token_id_internal(&deploy_salt)
}

/// Computes the ID for a canonical interchain token based on its address
#[must_use]
pub fn canonical_interchain_token_id(mint: &Pubkey) -> [u8; 32] {
    let salt = canonical_interchain_token_deploy_salt(mint);

    interchain_token_id_internal(&salt)
}

/// Computes the ID for a linked custom token based on its deployer and salt
#[must_use]
pub fn linked_token_id(deployer: &Pubkey, salt: &[u8; 32]) -> [u8; 32] {
    let salt = linked_token_deployer_salt(deployer, salt);

    interchain_token_id_internal(&salt)
}
#[cfg(test)]
mod tests {
    use super::CHAIN_NAME_HASH;

    #[test]
    fn test_chain_name_hash_constants() {
        #[cfg(feature = "mainnet")]
        let chain_name = "solana";
        #[cfg(feature = "testnet")]
        let chain_name = "solana-stagenet";
        #[cfg(feature = "stagenet")]
        let chain_name = "solana-testnet";
        #[cfg(feature = "devnet-amplifier")]
        let chain_name = "solana-devnet";

        let actual = solana_program::keccak::hash(chain_name.as_bytes()).to_bytes();
        assert_eq!(CHAIN_NAME_HASH, actual, "hash constant mismatch");
    }
}
