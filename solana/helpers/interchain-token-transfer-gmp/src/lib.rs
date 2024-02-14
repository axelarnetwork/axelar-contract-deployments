// TODO investigate if we can use the "new and shiny" [alloy-rs](https://github.com/alloy-rs/core/blob/main/crates/dyn-abi/src/value.rs)

pub use ethers_core;
use ethers_core::abi::{self, AbiDecode, AbiEncode, Detokenize, ParamType, Tokenizable};
use ethers_core::types::U256;
use token_manager::TokenManagerType;

/// The Remote one / TODO: better comment
pub struct DeployInterchainTokenB {
    pub salt: [u8; 32],
    pub destination_chain: Vec<u8>,
    pub token_manager: TokenManagerType,
    pub params: Vec<u8>,
    pub gas_value: U256, // TODO: check if this one is correct / there is like 4 of them now
}

/// The messages going through the Axelar Network between
/// InterchainTokenServices need to have a consistent format to be
/// understood properly. We chose to use abi encoding because it is easy to
/// use in EVM chains, which are at the front and center of programmable
/// blockchains, and because it is easy to implement in other ecosystems
/// which tend to be more gas efficient.
#[derive(Clone, Debug, PartialEq)]
pub enum GMPPayload {
    InterchainTransfer(InterchainTransfer),
    DeployInterchainToken(DeployInterchainToken),
    DeployTokenManager(DeployTokenManager),
}

pub trait Selector {
    fn selector() -> U256;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Bytes32(pub [u8; 32]);

/// This message has the following data encoded and should only be sent
/// after the proper tokens have been procured by the service. It should
/// result in the proper funds being transferred to the user at the
/// destionation chain.
#[derive(Clone, Debug, PartialEq)]
pub struct InterchainTransfer {
    /// The interchainTokenId of the token being transferred
    pub token_id: Bytes32,
    /// The address of the sender, encoded as bytes to account for different
    /// chain architectures
    pub source_address: Vec<u8>,
    /// The address of the recipient, encoded as bytes as well
    pub destination_address: Vec<u8>,
    /// The amount of token being send, not accounting for decimals (1 ETH
    /// would be 10^18)
    pub amount: U256,
    /// Either empty, for just a transfer, or any data to be passed to the
    /// destination address as a contract call
    pub data: Vec<u8>,
}

/// This message has the following data encoded and should only be sent
/// after the interchainTokenId has been properly generated (a user should
/// not be able to claim just any interchainTokenId)
#[derive(Clone, Debug, PartialEq)]
pub struct DeployInterchainToken {
    /// The interchainTokenId of the token being deployed
    pub token_id: Bytes32,
    /// The name for the token
    pub name: String,
    /// The symbol for the token
    pub symbol: String,
    /// The decimals for the token
    pub decimals: u8,
    /// An address on the destination chain that can mint/burn the deployed
    /// token on the destination chain, empty for no minter
    pub minter: Vec<u8>,
}

/// This message has the following data encoded and should only be sent
/// after the proper tokens have been procured by the service. It should
/// result in the proper funds being transferred to the user at the
/// destination chain.
#[derive(Clone, Debug, PartialEq)]
pub struct DeployTokenManager {
    /// The interchainTokenId of the token being deployed
    pub token_id: Bytes32,
    /// The type of the token manager, look at the [code](https://github.com/axelarnetwork/interchain-token-service/blob/main/contracts/interfaces/ITokenManagerType.sol) for details on EVM, but it could be different for different architectures
    pub token_manager_type: U256,
    /// An address on the destination chain that can mint/burn the deployed
    /// token on the destination chain, empty for no minter
    pub params: Vec<u8>,
}

impl Selector for InterchainTransfer {
    fn selector() -> U256 {
        U256::from(0)
    }
}

impl Selector for DeployInterchainToken {
    fn selector() -> U256 {
        U256::from(1)
    }
}

impl Selector for DeployTokenManager {
    fn selector() -> U256 {
        U256::from(2)
    }
}

impl Tokenizable for Bytes32 {
    fn from_token(
        token: ethers_core::abi::Token,
    ) -> Result<Self, ethers_core::abi::InvalidOutputType>
    where
        Self: Sized,
    {
        let bytes = <[u8; 32]>::from_token(token)?;
        Ok(Self(bytes))
    }

    fn into_token(self) -> ethers_core::abi::Token {
        self.0.into_token()
    }
}

impl AbiEncode for Bytes32 {
    fn encode(self) -> Vec<u8> {
        let token = self.0.into_token();
        abi::encode(&[token])
    }
}

impl AbiDecode for Bytes32 {
    fn decode(bytes: impl AsRef<[u8]>) -> Result<Self, ethers_core::abi::AbiError> {
        let tokens = abi::decode(&[ParamType::FixedBytes(32)], bytes.as_ref())?;
        Ok(<Self as Detokenize>::from_tokens(tokens)?)
    }
}

impl AbiEncode for InterchainTransfer {
    fn encode(self) -> Vec<u8> {
        let selector = Self::selector().into_token();
        let token_id = self.token_id.into_token();
        let source_address = ethers_core::types::Bytes::from_iter(self.source_address).into_token();
        let destination_address =
            ethers_core::types::Bytes::from_iter(self.destination_address).into_token();
        let amount = self.amount.into_token();
        let data = ethers_core::types::Bytes::from_iter(self.data).into_token();
        abi::encode(&[
            selector,
            token_id,
            source_address,
            destination_address,
            amount,
            data,
        ])
    }
}

impl AbiDecode for InterchainTransfer {
    fn decode(bytes: impl AsRef<[u8]>) -> Result<Self, ethers_core::abi::AbiError> {
        let mut tokens = abi::decode(
            &[
                ParamType::Uint(256),
                ParamType::FixedBytes(32),
                ParamType::Bytes,
                ParamType::Bytes,
                ParamType::Uint(256),
                ParamType::Bytes,
            ],
            bytes.as_ref(),
        )?
        .into_iter();

        let _selector = tokens.next().and_then(|x| x.into_uint()).expect("selector");
        Ok(Self {
            token_id: tokens
                .next()
                .and_then(|x| Bytes32::from_token(x).ok())
                .expect("token_id"),
            source_address: tokens
                .next()
                .and_then(|x| x.into_bytes())
                .expect("source_address"),
            destination_address: tokens
                .next()
                .and_then(|x| x.into_bytes())
                .expect("destination_address"),
            amount: tokens.next().and_then(|x| x.into_uint()).expect("amount"),
            data: tokens.next().and_then(|x| x.into_bytes()).expect("data"),
        })
    }
}

impl AbiEncode for DeployInterchainToken {
    fn encode(self) -> Vec<u8> {
        let selector = Self::selector().into_token();
        let token_id = self.token_id.into_token();
        let name = self.name.into_token();
        let symbol = self.symbol.into_token();
        let decimals = self.decimals.into_token();
        let minter = ethers_core::types::Bytes::from_iter(self.minter).into_token();
        abi::encode(&[selector, token_id, name, symbol, decimals, minter])
    }
}

impl AbiDecode for DeployInterchainToken {
    fn decode(bytes: impl AsRef<[u8]>) -> Result<Self, ethers_core::abi::AbiError> {
        let mut tokens = abi::decode(
            &[
                ParamType::Uint(256),
                ParamType::FixedBytes(32),
                ParamType::String,
                ParamType::String,
                ParamType::Uint(8),
                ParamType::Bytes,
            ],
            bytes.as_ref(),
        )?
        .into_iter();
        let _selector = tokens.next().and_then(|x| x.into_uint()).expect("selector");
        Ok(Self {
            token_id: tokens
                .next()
                .and_then(|x| Bytes32::from_token(x).ok())
                .expect("token_id"),
            name: tokens.next().and_then(|x| x.into_string()).expect("name"),
            symbol: tokens.next().and_then(|x| x.into_string()).expect("symbol"),
            decimals: tokens
                .next()
                .and_then(|x| x.into_uint())
                .map(|x| x.byte(0))
                .expect("decimals"),
            minter: tokens.next().and_then(|x| x.into_bytes()).expect("minter"),
        })
    }
}

impl AbiEncode for DeployTokenManager {
    fn encode(self) -> Vec<u8> {
        let selector = Self::selector().into_token();
        let token_id = self.token_id.into_token();
        let token_manager_type = self.token_manager_type.into_token();
        let params = ethers_core::types::Bytes::from_iter(self.params).into_token();
        abi::encode(&[selector, token_id, token_manager_type, params])
    }
}

impl AbiDecode for DeployTokenManager {
    fn decode(bytes: impl AsRef<[u8]>) -> Result<Self, ethers_core::abi::AbiError> {
        let mut tokens = abi::decode(
            &[
                ParamType::Uint(256),
                ParamType::FixedBytes(32),
                ParamType::Uint(256),
                ParamType::Bytes,
            ],
            bytes.as_ref(),
        )?
        .into_iter();

        let _selector = tokens.next().and_then(|x| x.into_uint()).expect("selector");
        Ok(Self {
            token_id: tokens
                .next()
                .and_then(|x| Bytes32::from_token(x).ok())
                .expect("token_id"),
            token_manager_type: tokens
                .next()
                .and_then(|x| x.into_uint())
                .expect("token_manager_type"),
            params: tokens.next().and_then(|x| x.into_bytes()).expect("params"),
        })
    }
}

impl AbiDecode for GMPPayload {
    fn decode(bytes: impl AsRef<[u8]>) -> Result<Self, ethers_core::abi::AbiError> {
        let variant = abi::decode(&[ParamType::Uint(256)], bytes.as_ref())?
            .into_iter()
            .next()
            .and_then(|variant| variant.into_uint())
            .ok_or(abi::ethabi::AbiError {
                name: "GMPPayload does not have a valid variant".to_string(),
                inputs: vec![],
            })
            .map_err(|_| ethers_core::abi::AbiError::WrongSelector)?;

        match variant.as_u32() {
            0 => Ok(GMPPayload::InterchainTransfer(InterchainTransfer::decode(
                bytes,
            )?)),
            1 => Ok(GMPPayload::DeployInterchainToken(
                DeployInterchainToken::decode(bytes)?,
            )),
            2 => Ok(GMPPayload::DeployTokenManager(DeployTokenManager::decode(
                bytes,
            )?)),
            _ => Err(ethers_core::abi::AbiError::WrongSelector),
        }
    }
}

impl AbiEncode for GMPPayload {
    fn encode(self) -> Vec<u8> {
        match self {
            GMPPayload::InterchainTransfer(data) => data.encode(),
            GMPPayload::DeployInterchainToken(data) => data.encode(),
            GMPPayload::DeployTokenManager(data) => data.encode(),
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn u256_into_u8() {
        let u256 = U256::from(42);
        let byte = u256.byte(0);
        assert_eq!(byte, 42);
    }

    /// fixture from https://github.com/axelarnetwork/interchain-token-service/blob/0977738a1d7df5551cb3bd2e18f13c0e09944ff2/test/InterchainTokenService.js
    /// [ 0,
    ///   '0xcccdb55f29bb017269049e59732c01ac41239e7b61e8a83be5c0ae1143ed8064',
    ///   '0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266',
    ///   '0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266',
    ///   1234,
    ///   '0x'
    /// ]
    const INTERCHAIN_TRANSFER: &str = "0x0000000000000000000000000000000000000000000000000000000000000000cccdb55f29bb017269049e59732c01ac41239e7b61e8a83be5c0ae1143ed806400000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000004d200000000000000000000000000000000000000000000000000000000000001400000000000000000000000000000000000000000000000000000000000000014f39fd6e51aad88f6f4ce6ab8827279cfffb922660000000000000000000000000000000000000000000000000000000000000000000000000000000000000014f39fd6e51aad88f6f4ce6ab8827279cfffb922660000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    fn interchain_transfer_decode() {
        // Setup
        let gmp = GMPPayload::decode_hex(INTERCHAIN_TRANSFER).unwrap();

        // Action
        let GMPPayload::InterchainTransfer(data) = gmp else {
            panic!("wrong variant");
        };

        // Assert
        assert_eq!(
            data.token_id.encode_hex(),
            "0xcccdb55f29bb017269049e59732c01ac41239e7b61e8a83be5c0ae1143ed8064"
        );
        assert_eq!(
            data.source_address,
            hex::decode("f39fd6e51aad88f6f4ce6ab8827279cfffb92266").unwrap()
        );
        assert_eq!(
            data.destination_address,
            hex::decode("f39fd6e51aad88f6f4ce6ab8827279cfffb92266").unwrap()
        );
        assert_eq!(data.amount, U256::from(1234));
        assert_eq!(data.data, Vec::<u8>::new());
    }

    #[test]
    fn interchain_transfer_encode() {
        assert_eq!(
            GMPPayload::decode_hex(INTERCHAIN_TRANSFER)
                .unwrap()
                .encode_hex(),
            INTERCHAIN_TRANSFER,
            "encode-decode should be idempotent"
        );
    }

    /// fixture from https://github.com/axelarnetwork/interchain-token-service/blob/0977738a1d7df5551cb3bd2e18f13c0e09944ff2/test/InterchainTokenService.js
    /// [
    ///   2,
    ///   '0xc5d28da02863aba312624a3c2c0b163be2292d59121ccfb7f37e666e50f75863',
    ///   2,
    ///   '0x00000000000000000000000000000000000000000000000000000000000000400000000000000000000000009a676e781a523b5d0c0e43731313a708cb6075080000000000000000000000000000000000000000000000000000000000000014f39fd6e51aad88f6f4ce6ab8827279cfffb92266000000000000000000000000'
    /// ]
    const DEPLOY_TOKEN_MANAGER: &str = "0x0000000000000000000000000000000000000000000000000000000000000002c5d28da02863aba312624a3c2c0b163be2292d59121ccfb7f37e666e50f7586300000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000400000000000000000000000009a676e781a523b5d0c0e43731313a708cb6075080000000000000000000000000000000000000000000000000000000000000014f39fd6e51aad88f6f4ce6ab8827279cfffb92266000000000000000000000000";

    #[test]
    fn deploy_token_manager_decode() {
        // Setup
        let gmp = GMPPayload::decode_hex(DEPLOY_TOKEN_MANAGER).unwrap();

        // Action
        let GMPPayload::DeployTokenManager(data) = gmp else {
            panic!("wrong variant");
        };

        // Assert
        assert_eq!(
            data.token_id.encode_hex(),
            "0xc5d28da02863aba312624a3c2c0b163be2292d59121ccfb7f37e666e50f75863"
        );
        assert_eq!(data.token_manager_type, U256::from(2));
        assert_eq!(
            data.params,
            hex::decode("00000000000000000000000000000000000000000000000000000000000000400000000000000000000000009a676e781a523b5d0c0e43731313a708cb6075080000000000000000000000000000000000000000000000000000000000000014f39fd6e51aad88f6f4ce6ab8827279cfffb92266000000000000000000000000").unwrap()
        );
    }

    #[test]
    fn deploy_token_manager_encode() {
        assert_eq!(
            GMPPayload::decode_hex(DEPLOY_TOKEN_MANAGER)
                .unwrap()
                .encode_hex(),
            DEPLOY_TOKEN_MANAGER,
            "encode-decode should be idempotent"
        );
    }

    /// fixture from https://github.com/axelarnetwork/interchain-token-service/blob/0977738a1d7df5551cb3bd2e18f13c0e09944ff2/test/InterchainTokenService.js
    /// [
    ///   1,
    ///   '0xd8a4ae903349d12f4f96391cb47ea769a5535e57963562a5ae0ef932b18137e2',
    ///   'Token Name',
    ///   'TN',
    ///   13,
    ///   '0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266'
    /// ]
    const DEPLOY_INTERCHAIN_TOKEN: &str = "0x0000000000000000000000000000000000000000000000000000000000000001d8a4ae903349d12f4f96391cb47ea769a5535e57963562a5ae0ef932b18137e200000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000d0000000000000000000000000000000000000000000000000000000000000140000000000000000000000000000000000000000000000000000000000000000a546f6b656e204e616d65000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002544e0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000014f39fd6e51aad88f6f4ce6ab8827279cfffb92266000000000000000000000000";

    #[test]
    fn deploy_token_interchain_token_decode() {
        // Setup
        let gmp = GMPPayload::decode_hex(DEPLOY_INTERCHAIN_TOKEN).unwrap();

        // Action
        let GMPPayload::DeployInterchainToken(data) = gmp else {
            panic!("wrong variant");
        };

        // Assert
        assert_eq!(
            data.token_id.encode_hex(),
            "0xd8a4ae903349d12f4f96391cb47ea769a5535e57963562a5ae0ef932b18137e2"
        );
        assert_eq!(data.name, "Token Name".to_string());
        assert_eq!(data.symbol, "TN".to_string(),);
        assert_eq!(data.decimals, 13,);
        assert_eq!(
            data.minter,
            hex::decode("f39fd6e51aad88f6f4ce6ab8827279cfffb92266").unwrap()
        );
    }

    #[test]
    fn deploy_token_interchain_token_encode() {
        assert_eq!(
            GMPPayload::decode_hex(DEPLOY_INTERCHAIN_TOKEN)
                .unwrap()
                .encode_hex(),
            DEPLOY_INTERCHAIN_TOKEN,
            "encode-decode should be idempotent"
        );
    }
}
