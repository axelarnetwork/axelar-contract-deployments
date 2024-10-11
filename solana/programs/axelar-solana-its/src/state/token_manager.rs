//! State module contains data structures that keep state within the ITS
//! program.

use core::mem::size_of;

use alloy_primitives::{Bytes, FixedBytes, U256};
use alloy_sol_types::SolValue;
use axelar_rkyv_encoding::types::PublicKey;
use rkyv::{Archive, Deserialize, Serialize};
use solana_program::program_error::ProgramError;

/// There are different types of token managers available for developers to
/// offer different types of integrations to ITS.
///
/// Each of these types correspond to an enum value. When deploying a token
/// manager developers must pass in the corresponding value for their desired
/// token manager type.
///
/// NOTE: The Gateway token manager type is not supported on Solana.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq))]
pub enum Type {
    /// For tokens that are deployed directly from ITS itself they use a native
    /// interchain token manager. Tokens that are deployed via the frontend
    /// portal also use this type of manager.
    NativeInterchainToken = 0,

    /// The mint/burnFrom token manager type, allows tokens to be burnt on the
    /// source chain when they are transferred out of that chain and minted they
    /// are transferred back into the source chain. As the name suggests when
    /// the token is burnt on the source chain the manager is looking to trigger
    /// the `burnFrom` function on the token rather than the `burn` function.
    /// The main implication is that ITS must be approved to call `burnFrom` by
    /// the token. The manager must be granted the role to be able to `mint` the
    /// token on the destination chain.
    MintBurnFrom,

    /// Token integrations using the lock/unlock token manager will have their
    /// token locked with their token’s manager. Only a single lock/unlock
    /// manager can exist for a token as having multiple lock/unlock managers
    /// would make it substantially more difficult to manage liquidity across
    /// many different blockchains. These token managers are best used in the
    /// case where a token has a “home chain” where a token can be locked. On
    /// the remote chains users can then use a wrapped version of that token
    /// which derives it’s value from a locked token back on the home chain.
    /// Canonical tokens for example deployed via ITS are examples where a
    /// lock/unlock token manager type is useful. When bridging tokens out of
    /// the destination chain (locking them at the manager) ITS will call the
    /// `transferTokenFrom` function, which in turn will call the
    /// `safeTransferFrom` function. For this transaction to be successful, ITS
    /// must be `approved` to call the `safeTransferFrom` function, otherwise
    /// the call will revert.
    LockUnlock,

    /// This manager type is similar to the lock/unlock token manager, where the
    /// manager locks
    /// the token on it’s “home chain” when it is bridged out and unlocks it
    /// when it is bridged back. The key feature with this token manager is
    /// that you have the option to set a fee that will be deducted when
    /// executing an `interchainTransfer`.
    ///
    /// This token type is currently not supported.
    LockUnlockFee,

    /// The mint/burn token manager type is the most common token manager type
    /// used for integrating tokens to ITS. This token manager type is used when
    /// there is no home chain for your token and allows you to `burn` tokens
    /// from the source chain and `mint` tokens on the destination chain. The
    /// manager will need to be granted the role to be able to execute the
    /// `mint` and `burn` function on the token.
    MintBurn,
}

impl TryFrom<U256> for Type {
    type Error = ProgramError;

    fn try_from(value: U256) -> Result<Self, Self::Error> {
        let value: u64 = value
            .try_into()
            .map_err(|_err| ProgramError::InvalidInstructionData)?;

        let converted = match value {
            0 => Self::NativeInterchainToken,
            1 => Self::MintBurnFrom,
            2 => Self::LockUnlock,
            3 => Self::LockUnlockFee,
            4 => Self::MintBurn,
            _ => return Err(ProgramError::InvalidInstructionData),
        };

        Ok(converted)
    }
}

/// Struct containing state of a `TokenManager`
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq))]
pub struct TokenManager {
    /// The type of `TokenManager`.
    pub ty: Type,

    /// The interchain token id.
    pub token_id: PublicKey,

    /// The token address within the Solana chain.
    pub token_address: PublicKey,

    /// The associated token account owned by the token manager.
    pub associated_token_account: PublicKey,

    /// The token manager PDA bump seed.
    pub bump: u8,

    /// The list of operators that are allowed to manage the flow limiters.
    pub operators: Vec<PublicKey>,

    /// The list of accounts that are allowed to request the `TokenManager` to
    /// mint tokens.
    pub minters: Option<Vec<PublicKey>>,
}

impl TokenManager {
    /// The length of the `TokenManager` struct in bytes.
    pub const LEN: usize = size_of::<Type>()
        + size_of::<PublicKey>()
        + size_of::<PublicKey>()
        + size_of::<PublicKey>()
        + size_of::<u8>();

    /// Creates a new `TokenManager` struct.
    #[must_use]
    pub const fn new(
        ty: Type,
        token_id: PublicKey,
        token_address: PublicKey,
        associated_token_account: PublicKey,
        bump: u8,
        operators: Vec<PublicKey>,
        minters: Option<Vec<PublicKey>>,
    ) -> Self {
        Self {
            ty,
            token_id,
            token_address,
            associated_token_account,
            bump,
            operators,
            minters,
        }
    }
}

/// Decodes the operator and token address from the given data.
///
/// The counterpart on EVM is implemented [here](https://github.com/axelarnetwork/interchain-token-service/blob/main/contracts/token-manager/TokenManager.sol#L191).
///
/// # Errors
///
/// If the data cannot be decoded.
pub fn decode_params(data: &[u8]) -> Result<(Option<PublicKey>, PublicKey), ProgramError> {
    let (operator_bytes, token_address) = <(Bytes, FixedBytes<32>)>::abi_decode(data, true)
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    if operator_bytes.is_empty() {
        return Ok((None, PublicKey::new_ed25519(token_address.0)));
    }

    let operator = operator_bytes
        .as_ref()
        .try_into()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    Ok((
        Some(PublicKey::new_ed25519(operator)),
        PublicKey::new_ed25519(token_address.0),
    ))
}
