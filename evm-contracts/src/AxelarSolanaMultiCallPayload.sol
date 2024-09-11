// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.19;

import {AbiSolanaGatewayPayload, SolanaGatewayPayload, SolanaAccountRepr} from "./SolanaGatewayPayload.sol";

/// @notice Payload for a program call
struct ProgramPayload {
    /// @notice The data to pass as instruction data to the program.
    bytes instructionData;
    /// @notice The index of the program account in the top-level accounts slice.
    uint64 programAccountIndex;
    /// @notice The start index within the top-level accounts slice where the accounts for this program call are
    /// located.
    uint64 accountsStartIndex;
    /// @notice The end index within the top-level accounts slice where the accounts for this program call are
    /// located.
    uint64 accountsEndIndex;
}

/// @notice Payload for a multi-call
struct MultiCallPayload {
    ProgramPayload[] payloads;
}

/// @notice A call to a program
struct AxelarSolanaCall {
    /// @notice The destination program to call on Solana
    bytes32 destinationProgram;
    /// @notice The payload (instruction data) to execute
    SolanaGatewayPayload payload;
}

library AxelarSolanaMultiCallPayloadEncoder {
    function encode(
        AxelarSolanaCall[] calldata calls
    ) internal pure returns (bytes memory) {
        uint64 currentIndex = 0;
        uint totalAccounts = calls.length;
        ProgramPayload[] memory payloads = new ProgramPayload[](calls.length);

        for (uint i = 0; i < calls.length; i++) {
            totalAccounts += calls[i].payload.accounts.length;
        }

        SolanaAccountRepr[] memory topLevelAccounts = new SolanaAccountRepr[](
            totalAccounts
        );

        for (uint i = 0; i < calls.length; i++) {
            payloads[i] = ProgramPayload({
                instructionData: calls[i].payload.executePayload,
                programAccountIndex: currentIndex,
                accountsStartIndex: currentIndex + 1,
                accountsEndIndex: currentIndex +
                    1 +
                    uint64(calls[i].payload.accounts.length)
            });

            topLevelAccounts[currentIndex++] = SolanaAccountRepr({
                pubkey: calls[i].destinationProgram,
                isSigner: false,
                isWritable: false
            });

            for (uint j = 0; j < calls[i].payload.accounts.length; j++) {
                topLevelAccounts[currentIndex++] = calls[i].payload.accounts[j];
            }
        }

        SolanaGatewayPayload memory outputPayload = SolanaGatewayPayload({
            executePayload: abi.encode(MultiCallPayload({payloads: payloads})),
            accounts: topLevelAccounts
        });

        return AbiSolanaGatewayPayload.encode(outputPayload);
    }
}
