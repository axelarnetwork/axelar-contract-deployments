//! State module contains data structures that keep state within the ITS
//! program.

use core::mem::size_of;

use alloy_primitives::{Bytes, FixedBytes, U256};
use alloy_sol_types::SolValue;
use axelar_rkyv_encoding::types::PublicKey;
use program_utils::StorableArchive;
use rkyv::{bytecheck, Archive, CheckBytes, Deserialize, Serialize};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// There are different types of token managers available for developers to
/// offer different types of integrations to ITS.
///
/// Each of these types correspond to an enum value. When deploying a token
/// manager developers must pass in the corresponding value for their desired
/// token manager type.
///
/// NOTE: The Gateway token manager type is not supported on Solana.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone, Copy)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, Clone, Copy, CheckBytes))]
#[non_exhaustive]
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

impl From<ArchivedType> for Type {
    fn from(value: ArchivedType) -> Self {
        match value {
            ArchivedType::NativeInterchainToken => Self::NativeInterchainToken,
            ArchivedType::MintBurnFrom => Self::MintBurnFrom,
            ArchivedType::LockUnlock => Self::LockUnlock,
            ArchivedType::LockUnlockFee => Self::LockUnlockFee,
            ArchivedType::MintBurn => Self::MintBurn,
        }
    }
}

impl From<Type> for U256 {
    fn from(value: Type) -> Self {
        match value {
            Type::NativeInterchainToken => Self::from(0_u8),
            Type::MintBurnFrom => Self::from(1_u8),
            Type::LockUnlock => Self::from(2_u8),
            Type::LockUnlockFee => Self::from(3_u8),
            Type::MintBurn => Self::from(4_u8),
        }
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
    pub token_id: [u8; 32],

    /// The token address within the Solana chain.
    pub token_address: [u8; 32],

    /// The associated token account owned by the token manager.
    pub associated_token_account: [u8; 32],

    /// The flow limit for the token manager
    pub flow_limit: u64,

    /// The token manager PDA bump seed.
    pub bump: u8,
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
        token_id: [u8; 32],
        token_address: [u8; 32],
        associated_token_account: [u8; 32],
        bump: u8,
    ) -> Self {
        Self {
            ty,
            token_id,
            token_address,
            associated_token_account,
            flow_limit: 0,
            bump,
        }
    }
}

impl StorableArchive<0> for TokenManager {}

impl ArchivedTokenManager {
    /// Deserializes the `TokenManager` from the given bytes using `rkyv` for
    /// zero-copy deserialization.
    #[must_use]
    pub fn from_bytes(data: &[u8]) -> &Self {
        // SAFETY: The data is assumed to be a valid archived `TokenManager`. The
        // `TokenManager` is always serialized as an archived struct.
        unsafe { rkyv::archived_root::<TokenManager>(data) }
    }
}

/// Decodes the operator and token address from the given data.
///
/// The counterpart on EVM is implemented [here](https://github.com/axelarnetwork/interchain-token-service/blob/main/contracts/token-manager/TokenManager.sol#L191).
///
/// # Errors
///
/// If the data cannot be decoded.
pub fn decode_params(
    data: &[u8],
) -> Result<(Option<Pubkey>, Option<Pubkey>, Pubkey), ProgramError> {
    let (operator_bytes, mint_authority_bytes, token_address) =
        <(Bytes, Bytes, FixedBytes<32>)>::abi_decode(data, true)
            .map_err(|_err| ProgramError::InvalidInstructionData)?;

    let token_address = Pubkey::new_from_array(token_address.0);

    let operator = if operator_bytes.is_empty() {
        None
    } else {
        let operator_byte_array: [u8; 32] = operator_bytes
            .as_ref()
            .try_into()
            .map_err(|_err| ProgramError::InvalidInstructionData)?;

        Some(Pubkey::new_from_array(operator_byte_array))
    };

    let mint_authority = if mint_authority_bytes.is_empty() {
        None
    } else {
        let mint_authority_byte_array: [u8; 32] = mint_authority_bytes
            .as_ref()
            .try_into()
            .map_err(|_err| ProgramError::InvalidInstructionData)?;

        Some(Pubkey::new_from_array(mint_authority_byte_array))
    };

    Ok((operator, mint_authority, token_address))
}

/// Encodes the operator, mint authority, and token address into a byte array.
///
/// This encoding scheme is aimed at Solana ITS. If you're sending a
/// `DeployTokenManager` message to a different chain, please make sure to
/// encode the data as required by the destination chain.
#[must_use]
pub fn encode_params(
    maybe_operator: Option<Pubkey>,
    maybe_mint_authority: Option<Pubkey>,
    token_address: Pubkey,
) -> Vec<u8> {
    let operator_bytes = maybe_operator
        .map(|operator| Bytes::from(operator.to_bytes()))
        .unwrap_or_default();
    let mint_authority_bytes = maybe_mint_authority
        .map(|mint_authority| Bytes::from(mint_authority.to_bytes()))
        .unwrap_or_default();
    let token_address_bytes = FixedBytes::<32>::from(token_address.to_bytes());
    (operator_bytes, mint_authority_bytes, token_address_bytes).abi_encode()
}

#[cfg(test)]
mod tests {
    use solana_program::pubkey::Pubkey;

    #[test]
    fn test_encode_decode_params_roundtrip() {
        let operator = Pubkey::new_unique();
        let mint_authority = Pubkey::new_unique();
        let token_address = Pubkey::new_unique();
        let encoded = super::encode_params(Some(operator), Some(mint_authority), token_address);
        let (decoded_operator, decoded_mint_authority, decoded_token_address) =
            super::decode_params(&encoded).unwrap();

        assert_eq!(Some(operator), decoded_operator);
        assert_eq!(Some(mint_authority), decoded_mint_authority);
        assert_eq!(token_address, decoded_token_address);
    }
}
