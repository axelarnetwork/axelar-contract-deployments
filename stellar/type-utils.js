'use strict';

const { Address, nativeToScVal } = require('@stellar/stellar-sdk');
const { ethers } = require('hardhat');
const {
    utils: { isHexString },
} = ethers;

function weightedSignersToScVal(signers) {
    return nativeToScVal(
        {
            nonce: Buffer.from(signers.nonce),
            signers: signers.signers.map(({ signer, weight }) => ({
                signer: Address.fromString(signer).toBuffer(),
                weight,
            })),
            threshold: signers.threshold,
        },
        {
            type: {
                signers: [
                    'symbol',
                    {
                        signer: ['symbol', 'bytes'],
                        weight: ['symbol', 'u128'],
                    },
                ],
                nonce: ['symbol', 'bytes'],
                threshold: ['symbol', 'u128'],
            },
        },
    );
}

function commandTypeToScVal(commandType) {
    return nativeToScVal([nativeToScVal(commandType, { type: 'symbol' })]);
}

function messagesToScVal(messages) {
    return nativeToScVal(
        messages.map((message) =>
            nativeToScVal(
                {
                    message_id: message.messageId,
                    source_chain: message.sourceChain,
                    source_address: message.sourceAddress,
                    contract_address: Address.fromString(message.contractAddress),
                    payload_hash: Buffer.from(message.payloadHash),
                },
                {
                    type: {
                        message_id: ['symbol'],
                        source_chain: ['symbol'],
                        source_address: ['symbol'],
                        contract_address: ['symbol'],
                        payload_hash: ['symbol'],
                    },
                },
            ),
        ),
    );
}

function proofToScVal(proof) {
    return nativeToScVal(
        {
            signers: proof.signers.map(({ signer: { signer, weight }, signature }) => {
                return {
                    signer: {
                        signer: Address.fromString(signer).toBuffer(),
                        weight,
                    },
                    signature: signature
                        ? nativeToScVal([nativeToScVal('Signed', { type: 'symbol' }), Buffer.from(signature)])
                        : nativeToScVal([nativeToScVal('Unsigned', { type: 'symbol' })]),
                };
            }),
            threshold: proof.threshold,
            nonce: Buffer.from(proof.nonce),
        },
        {
            type: {
                signers: [
                    'symbol',
                    {
                        signer: [
                            'symbol',
                            {
                                signer: ['symbol', 'bytes'],
                                weight: ['symbol', 'u128'],
                            },
                        ],
                        signature: ['symbol'],
                    },
                ],
                threshold: ['symbol', 'u128'],
                nonce: ['symbol', 'bytes'],
            },
        },
    );
}

function itsCustomMigrationDataToScValV112(migrationData) {
    return nativeToScVal(
        {
            new_token_manager_wasm_hash: Buffer.from(migrationData.newTokenManagerWasmHash, 'hex'),
            new_interchain_token_wasm_hash: Buffer.from(migrationData.newInterchainTokenWasmHash, 'hex'),
        },
        {
            type: {
                new_token_manager_wasm_hash: ['symbol', 'bytes'],
                new_interchain_token_wasm_hash: ['symbol', 'bytes'],
            },
        },
    );
}

function functionCallsToScVal(functionCalls) {
    if (!functionCalls.length) return nativeToScVal([]);

    return nativeToScVal(functionCalls.map(({ contract, approver, function: fn, args = [] }) =>
        nativeToScVal(
            {
                contract: Address.fromString(contract),
                approver: Address.fromString(approver),
                function: nativeToScVal(fn, { type: 'symbol' }),
                args: args.map(arg =>
                    isHexString(arg)
                        ? Buffer.from(arg.slice(2), 'hex')
                        : arg
                )
            },
            {
                type: {
                    contract: ['symbol'],
                    approver: ['symbol'],
                    function: ['symbol'],
                    args: ['symbol'],
                },
            }
        )
    ));
}

module.exports = {
    commandTypeToScVal,
    messagesToScVal,
    weightedSignersToScVal,
    proofToScVal,
    itsCustomMigrationDataToScValV112,
    functionCallsToScVal,
};
