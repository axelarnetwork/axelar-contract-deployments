'use strict';

const { bcs } = require('@mysten/sui.js/bcs');
const { ethers } = require('hardhat');
const {
    utils: { arrayify, hexlify },
} = ethers;

const addressStruct = bcs.bytes(32).transform({
    input: (val) => arrayify(val),
    output: (val) => hexlify(val),
});

const signerStruct = bcs.struct('WeightedSigner', {
    pubkey: bcs.vector(bcs.u8()),
    weight: bcs.u128(),
});

const bytes32Struct = bcs.fixedArray(32, bcs.u8()).transform({
    input: (id) => arrayify(id),
    output: (id) => hexlify(id),
});

const signersStruct = bcs.struct('WeightedSigners', {
    signers: bcs.vector(signerStruct),
    threshold: bcs.u128(),
    nonce: bytes32Struct,
});

const messageToSignStruct = bcs.struct('MessageToSign', {
    domain_separator: bytes32Struct,
    signers_hash: bytes32Struct,
    data_hash: bytes32Struct,
});

const messageStruct = bcs.struct('Message', {
    source_chain: bcs.string(),
    message_id: bcs.string(),
    source_address: bcs.string(),
    destination_id: addressStruct,
    payload_hash: bytes32Struct,
});

const proofStruct = bcs.struct('Proof', {
    signers: signersStruct,
    signatures: bcs.vector(bcs.vector(bcs.u8())),
});

module.exports = {
    addressStruct,
    signerStruct,
    bytes32Struct,
    signersStruct,
    messageToSignStruct,
    messageStruct,
    proofStruct,
};
