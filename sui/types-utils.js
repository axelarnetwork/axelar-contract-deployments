'use strict';

const { bcs } = require('@mysten/sui/bcs');
const { fromHEX, toHEX } = require('@mysten/bcs');
const { ethers } = require('hardhat');
const {
    utils: { arrayify, hexlify },
} = ethers;

const addressStruct = bcs.bytes(32).transform({
    input: (val) => arrayify(val),
    output: (val) => hexlify(val),
});

const signerStruct = bcs.struct('WeightedSigner', {
    pub_key: bcs.vector(bcs.u8()),
    weight: bcs.u128(),
});

const bytes32Struct = bcs.fixedArray(32, bcs.u8()).transform({
    input: (id) => arrayify(id),
    output: (id) => hexlify(id),
});

const UID = bcs.fixedArray(32, bcs.u8()).transform({
    input: (id) => fromHEX(id),
    output: (id) => toHEX(Uint8Array.from(id)),
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

const approvedMessageStruct = bcs.struct('ApprovedMessage', {
    source_chain: bcs.string(),
    message_id: bcs.string(),
    source_address: bcs.string(),
    destination_id: addressStruct,
    payload: bcs.vector(bcs.u8()),
});

const proofStruct = bcs.struct('Proof', {
    signers: signersStruct,
    signatures: bcs.vector(bcs.vector(bcs.u8())),
});

const gasServiceStruct = bcs.struct('GasService', {
    id: UID,
    balance: bcs.u64(),
});

const channelStruct = bcs.struct('Channel', {
    id: UID,
});

const singletonStruct = bcs.struct('Singleton', {
    id: UID,
    channel: channelStruct,
});

const discoveryTable = bcs.struct('DiscoveryTable', {
    id: UID,
});

const discoveryStruct = bcs.struct('Discovery', {
    id: UID,
    fields: discoveryTable,
});

const bagStruct = bcs.struct('Bag', {
    id: UID,
    size: bcs.U64,
});

const operatorsStruct = bcs.struct('Operators', {
    id: UID,
    operators: bcs.vector(addressStruct),
    caps: bagStruct,
});

const tableStruct = bcs.struct('Table', {
    id: UID,
    size: bcs.U64,
});

const interchainAddressTrackerStruct = bcs.struct('InterchainAddressTracker', {
    trusted_addresses: tableStruct,
});

const itsStruct = bcs.struct('ITS', {
    id: UID,
    channel: channelStruct,
    address_tracker: interchainAddressTrackerStruct,
    unregistered_coin_types: tableStruct,
    unregistered_coin_info: bagStruct,
    registered_coin_types: tableStruct,
    registered_coins: bagStruct,
    relayer_discovery_id: UID,
});

const coinBagStrcut = bcs.struct('CoinBag', {
    bag: bagStruct,
});

const squidStruct = bcs.struct('Squid', {
    id: UID,
    channel: channelStruct,
    coin_bag: coinBagStrcut,
});

module.exports = {
    addressStruct,
    signerStruct,
    bytes32Struct,
    signersStruct,
    operatorsStruct,
    messageToSignStruct,
    messageStruct,
    approvedMessageStruct,
    proofStruct,
    gasServiceStruct,
    channelStruct,
    singletonStruct,
    discoveryStruct,
    itsStruct,
    squidStruct,
};
