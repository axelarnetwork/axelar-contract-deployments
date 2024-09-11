// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.19;

import {AbiSolanaGatewayPayload, SolanaGatewayPayload, SolanaAccountRepr} from "./SolanaGatewayPayload.sol";
import {IBaseAmplifierGateway} from "axelar-gmp-sdk-solidity/interfaces/IBaseAmplifierGateway.sol";
import {AxelarSolanaCall, AxelarSolanaMultiCallPayloadEncoder} from "./AxelarSolanaMultiCallPayload.sol";

contract AxelarSolanaMultiCall {
    /// @dev The amplifier gateway address
    IBaseAmplifierGateway public gateway;

    constructor(address gateway_) {
        gateway = IBaseAmplifierGateway(gateway_);
    }

    function multiCall(
        AxelarSolanaCall[] calldata calls,
        bytes calldata solanaChain,
        string calldata solanaMultiCallAddress
    ) external {
        bytes memory encodedPayload = AxelarSolanaMultiCallPayloadEncoder
            .encode(calls);
        gateway.callContract(
            string(solanaChain),
            solanaMultiCallAddress,
            encodedPayload
        );
    }
}
