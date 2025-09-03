'use strict';

const { ethers } = require('ethers');
const { MessageType } = require('@commonprefix/axelar-cgp-ton');

function encodeInterchainTransferHubMessage(originalSourceChain, params) {
    const abiCoder = new ethers.utils.AbiCoder();

    // First encode the inner payload (interchain transfer message)
    const innerPayload = abiCoder.encode(
        ['uint256', 'bytes32', 'bytes', 'bytes', 'uint256', 'bytes'],
        [MessageType.INTERCHAIN_TRANSFER, params.tokenId, params.sourceAddress, params.destinationAddress, params.amount, params.data],
    );

    // Then wrap it in the hub message format
    const hubMessage = abiCoder.encode(['uint256', 'string', 'bytes'], [MessageType.SEND_TO_HUB, originalSourceChain, innerPayload]);

    return hubMessage.slice(2); // remove the "0x" prefix
}

function encodeDeployInterchainTokenHubMessage(originalSourceChain, params) {
    const abiCoder = new ethers.utils.AbiCoder();

    // First encode the inner payload (deploy interchain token message)
    const innerPayload = abiCoder.encode(
        ['uint256', 'bytes32', 'string', 'string', 'uint8', 'bytes'],
        [MessageType.DEPLOY_INTERCHAIN_TOKEN, params.tokenId, params.name, params.symbol, params.decimals, params.minter],
    );

    // Then wrap it in the hub message format
    const hubMessage = abiCoder.encode(['uint256', 'string', 'bytes'], [MessageType.SEND_TO_HUB, originalSourceChain, innerPayload]);

    return hubMessage.slice(2); // remove the "0x" prefix
}

function encodeLinkTokenHubMessage(originalSourceChain, params) {
    const abiCoder = new ethers.utils.AbiCoder();

    // First encode the inner payload (link token message)
    const innerPayload = abiCoder.encode(
        ['uint256', 'bytes32', 'uint256', 'bytes', 'bytes', 'bytes'],
        [
            MessageType.LINK_TOKEN,
            params.tokenId,
            params.tokenManagerType,
            params.sourceAddress,
            params.destinationAddress,
            params.linkParams,
        ],
    );

    // Then wrap it in the hub message format
    const hubMessage = abiCoder.encode(['uint256', 'string', 'bytes'], [MessageType.SEND_TO_HUB, originalSourceChain, innerPayload]);

    return hubMessage.slice(2); // remove the "0x" prefix
}

function encodeRegisterTokenMetadataAbi(message) {
    const abiCoder = new ethers.utils.AbiCoder();

    // Encode inner payload: uint256, bytes, uint256
    const encoded = abiCoder.encode(
        ['uint256', 'bytes', 'uint256'],
        [
            MessageType.REGISTER_TOKEN_METADATA, // uint256 - MessageType.REGISTER_TOKEN_METADATA
            message.tokenAddress, // bytes - token address
            message.decimals, // uint256 - decimals
        ],
    );

    return encoded;
}

module.exports = {
    MessageType,
    encodeInterchainTransferHubMessage,
    encodeDeployInterchainTokenHubMessage,
    encodeLinkTokenHubMessage,
    encodeRegisterTokenMetadataAbi,
};
