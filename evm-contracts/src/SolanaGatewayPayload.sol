// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.13;

using AbiSolanaGatewayPayload for SolanaGatewayPayload global;

/// @notice Represents a payload that can be executed in the Solana network.
struct SolanaGatewayPayload {
    /// @notice The encoding scheme that's used for encoding the payload.
    uint8 scheme;
    /// @notice The specific instructions to be executed, encoded in bytes.
    bytes executePayload;
    /// @notice An array of Solana accounts involved in the transaction.
    SolanaAccountRepr[] accounts;
}

/// @notice Represents a Solana account in a transaction.
struct SolanaAccountRepr {
    /// @notice The public key of the Solana account.
    bytes32 pubkey;
    /// @notice Indicates if the account is a signer of the transaction.
    bool isSigner;
    /// @notice Indicates if the account should be writable during execution.
    bool isWritable;
}


// The ABI scheme used for encoding the SolanaGatewayPayload.
// The value is manually synced with the Rust implementation.
uint8 constant ABI_SCHEME = 1;

/// @notice Library for encoding and decoding SolanaGatewayPayload structs.
library AbiSolanaGatewayPayload {

    /// @dev Decodes a byte array into a `SolanaGatewayPayload` struct.
    /// Assumes the first byte indicates the ABI scheme used for encoding.
    /// @param data The byte array containing the encoded SolanaGatewayPayload.
    ///     The first byte is the ABI scheme, followed by the encoded payload.
    ///     We are using `calldata` because `memory` does not support slices.
    /// @return A `SolanaGatewayPayload` struct decoded from the input data.
    function decode(bytes calldata data) internal pure returns (SolanaGatewayPayload memory) {
        if (uint8(data[0]) != ABI_SCHEME) {
            revert("AbiSolanaGatewayPayload: invalid scheme");
        }

        (bytes memory executePayload, SolanaAccountRepr[] memory accounts) =
            abi.decode(data[1:], (bytes, SolanaAccountRepr[]));

        return SolanaGatewayPayload({scheme: ABI_SCHEME, executePayload: executePayload, accounts: accounts});
    }

    /// @dev Encodes a `SolanaGatewayPayload` struct into a byte array.
    /// Uses the ABI_SCHEME as the first byte to indicate the encoding scheme.
    /// @param payload The `SolanaGatewayPayload` struct to encode.
    /// @return A byte array containing the encoded SolanaGatewayPayload.
    function encode(SolanaGatewayPayload memory payload) internal pure returns (bytes memory) {
        bytes memory encodedPayload = abi.encode(payload.executePayload, payload.accounts);

        // we don't need need to pad the first byte as per the protocol design - hence using `abi.encodePacked`
        return abi.encodePacked(ABI_SCHEME, encodedPayload);
    }
}
