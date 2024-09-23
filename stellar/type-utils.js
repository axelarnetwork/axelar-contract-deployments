'use strict';

const { Address, nativeToScVal } = require('@stellar/stellar-sdk');

function weightedSignersToScVal(signers) {
    return nativeToScVal(
        {
            nonce: Buffer.from(signers.nonce),
            signers: signers.signers.map(({ signer, weight }) => ({
                signer: Address.fromString(signer).toBuffer(),
                weight,
            })),
            threshold: 1,
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
                    payload_hash: Buffer.from(message.payloadHash, 'hex'),
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
                        ? nativeToScVal([nativeToScVal('Signed', { type: 'symbol' }), Buffer.from(signature, 'hex')])
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

module.exports = {
    commandTypeToScVal,
    messagesToScVal,
    weightedSignersToScVal,
    proofToScVal,
};
