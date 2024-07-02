'use strict';

const { ethers } = require('hardhat');
const { loadConfig } = require('../evm/utils');
const {
    BigNumber,
    utils: { arrayify, hexlify },
} = ethers;
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

const SUI_COIN_ID = '0x2::sui::SUI';

const getAmplifierSigners = async (config, chain) => {
    const client = await CosmWasmClient.connect(config.axelar.rpc);
    const workerSet = await client.queryContractSmart(config.axelar.contracts.MultisigProver[chain].address, 'current_verifier_set');
    const signers = Object.values(workerSet.signers);

    const weightedSigners = signers
        .map((signer) => ({
            pubkey: arrayify(`0x${signer.pub_key.ecdsa}`),
            weight: Number(signer.weight),
        }))
        .sort((a, b) => hexlify(a.pubkey).localeCompare(hexlify(b.pubkey)));

    return {
        signers: weightedSigners,
        threshold: Number(workerSet.threshold),
        nonce: ethers.utils.hexZeroPad(BigNumber.from(workerSet.created_at).toHexString(), 32),
    };
};

const loadSuiConfig = (env) => {
    const config = loadConfig(env);
    const suiEnv = env === 'local' ? 'localnet' : env;

    if (!config.sui) {
        config.sui = {
            networkType: suiEnv,
            name: 'Sui',
            contracts: {
                axelar_gateway: {},
            },
        };
    }

    return config;
};

const isGasToken = (coinType) => {
    return coinType === SUI_COIN_ID;
};

const paginateAll = async (client, paginatedFn, params, pageLimit = 100) => {
    let cursor;
    let response = await client[paginatedFn]({
        ...params,
        cursor,
        limit: pageLimit,
    });
    const items = response.data;

    while (response.hasNextPage) {
        response = await client[paginatedFn]({
            ...params,
            cursor: response.nextCursor,
            limit: pageLimit,
        });
        items.push(...response.data);
    }

    return items;
};

module.exports = {
    SUI_COIN_ID,
    getAmplifierSigners,
    isGasToken,
    paginateAll,
    loadSuiConfig,
};
