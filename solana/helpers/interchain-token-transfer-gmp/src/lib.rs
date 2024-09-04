pub use alloy_primitives;
use alloy_sol_types::{sol, SolValue};

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

sol! {
    /// This message has the following data encoded and should only be sent
    /// after the proper tokens have been procured by the service. It should
    /// result in the proper funds being transferred to the user at the
    /// destination chain.
    #[derive(Debug, PartialEq)]
    #[repr(C)]
    struct InterchainTransfer {
        /// Will always have a value of 0
        uint256 selector;
        /// The interchainTokenId of the token being transferred
        bytes32 token_id;
        /// The address of the sender, encoded as bytes to account for different chain
        /// architectures
        bytes source_address;
        /// The address of the recipient, encoded as bytes as well
        bytes destination_address;
        /// The amount of token being send, not accounting for decimals (1 ETH would be 1018)
        uint256 amount;
        /// Either empty, for just a transfer, or any data to be passed to the destination address
        /// as a contract call
        bytes data;
    }

    /// This message has the following data encoded and should only be sent
    /// after the interchainTokenId has been properly generated (a user should
    /// not be able to claim just any interchainTokenId)
    #[derive(Debug, PartialEq)]
    #[repr(C)]
    struct DeployInterchainToken {
        uint256 selector;
        /// The interchainTokenId of the token being deployed
        bytes32 token_id;
        /// The name for the token
        string name;
        /// The symbol for the token
        string symbol;
        /// The decimals for the token
        uint8 decimals;
        /// An address on the destination chain that can mint/burn the deployed
        /// token on the destination chain, empty for no minter
        bytes minter;
    }

    /// This message has the following data encoded and should only be sent
    /// after the proper tokens have been procured by the service. It should
    /// result in the proper funds being transferred to the user at the
    /// destination chain.
    #[derive(Debug, PartialEq)]
    #[repr(C)]
    struct DeployTokenManager {
        uint256 selector;
        /// The interchainTokenId of the token being deployed
        bytes32 token_id;
        /// The type of the token manager, look at the [code](https://github.com/axelarnetwork/interchain-token-service/blob/main/contracts/interfaces/ITokenManagerType.sol) for details on EVM, but it could be different for different architectures
        uint256 token_manager_type;
        /// An address on the destination chain that can mint/burn the deployed
        /// token on the destination chain, empty for no minter
        bytes params;
    }
}

impl GMPPayload {
    pub fn decode(bytes: &[u8]) -> Result<Self, alloy_sol_types::Error> {
        let variant = alloy_primitives::U256::abi_decode(&bytes[0..32], true)?;

        match variant.byte(0) {
            0 => Ok(GMPPayload::InterchainTransfer(
                InterchainTransfer::abi_decode_params(bytes, true)?,
            )),
            1 => Ok(GMPPayload::DeployInterchainToken(
                DeployInterchainToken::abi_decode_params(bytes, true)?,
            )),
            2 => Ok(GMPPayload::DeployTokenManager(
                DeployTokenManager::abi_decode_params(bytes, true)?,
            )),
            _ => Err(alloy_sol_types::Error::custom(
                "Invalid selector for InterchainTokenService message",
            )),
        }
    }

    pub fn encode(self) -> Vec<u8> {
        match self {
            GMPPayload::InterchainTransfer(data) => data.abi_encode_params(),
            GMPPayload::DeployInterchainToken(data) => data.abi_encode_params(),
            GMPPayload::DeployTokenManager(data) => data.abi_encode_params(),
        }
    }
}

#[cfg(test)]
mod tests {

    use alloy_primitives::U256;

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
    const INTERCHAIN_TRANSFER: &str = "0000000000000000000000000000000000000000000000000000000000000000cccdb55f29bb017269049e59732c01ac41239e7b61e8a83be5c0ae1143ed806400000000000000000000000000000000000000000000000000000000000000c0000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000004d200000000000000000000000000000000000000000000000000000000000001400000000000000000000000000000000000000000000000000000000000000014f39fd6e51aad88f6f4ce6ab8827279cfffb922660000000000000000000000000000000000000000000000000000000000000000000000000000000000000014f39fd6e51aad88f6f4ce6ab8827279cfffb922660000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

    #[test]
    fn interchain_transfer_decode() {
        // Setup
        let gmp = GMPPayload::decode(&hex::decode(INTERCHAIN_TRANSFER).unwrap()).unwrap();

        // Action
        let GMPPayload::InterchainTransfer(data) = gmp else {
            panic!("wrong variant");
        };

        // Assert
        assert_eq!(
            hex::encode(data.token_id.abi_encode()),
            "cccdb55f29bb017269049e59732c01ac41239e7b61e8a83be5c0ae1143ed8064"
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
            hex::encode(
                GMPPayload::decode(&hex::decode(INTERCHAIN_TRANSFER).unwrap())
                    .unwrap()
                    .encode()
            ),
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
    const DEPLOY_TOKEN_MANAGER: &str = "0000000000000000000000000000000000000000000000000000000000000002c5d28da02863aba312624a3c2c0b163be2292d59121ccfb7f37e666e50f7586300000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000000400000000000000000000000009a676e781a523b5d0c0e43731313a708cb6075080000000000000000000000000000000000000000000000000000000000000014f39fd6e51aad88f6f4ce6ab8827279cfffb92266000000000000000000000000";

    #[test]
    fn deploy_token_manager_decode() {
        // Setup
        let gmp = GMPPayload::decode(&hex::decode(DEPLOY_TOKEN_MANAGER).unwrap()).unwrap();

        // Action
        let GMPPayload::DeployTokenManager(data) = gmp else {
            panic!("wrong variant");
        };

        // Assert
        assert_eq!(
            hex::encode(data.token_id.abi_encode()),
            "c5d28da02863aba312624a3c2c0b163be2292d59121ccfb7f37e666e50f75863"
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
            hex::encode(
                GMPPayload::decode(&hex::decode(DEPLOY_TOKEN_MANAGER).unwrap())
                    .unwrap()
                    .encode()
            ),
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
    const DEPLOY_INTERCHAIN_TOKEN: &str = "0000000000000000000000000000000000000000000000000000000000000001d8a4ae903349d12f4f96391cb47ea769a5535e57963562a5ae0ef932b18137e200000000000000000000000000000000000000000000000000000000000000c00000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000000d0000000000000000000000000000000000000000000000000000000000000140000000000000000000000000000000000000000000000000000000000000000a546f6b656e204e616d65000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000002544e0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000014f39fd6e51aad88f6f4ce6ab8827279cfffb92266000000000000000000000000";

    #[test]
    fn deploy_token_interchain_token_decode() {
        // Setup
        let gmp = GMPPayload::decode(&hex::decode(DEPLOY_INTERCHAIN_TOKEN).unwrap()).unwrap();

        // Action
        let GMPPayload::DeployInterchainToken(data) = gmp else {
            panic!("wrong variant");
        };

        // Assert
        assert_eq!(
            hex::encode(data.token_id.abi_encode()),
            "d8a4ae903349d12f4f96391cb47ea769a5535e57963562a5ae0ef932b18137e2"
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
            hex::encode(
                GMPPayload::decode(&hex::decode(DEPLOY_INTERCHAIN_TOKEN).unwrap())
                    .unwrap()
                    .encode()
            ),
            DEPLOY_INTERCHAIN_TOKEN,
            "encode-decode should be idempotent"
        );
    }
}
