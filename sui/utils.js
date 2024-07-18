'use strict';

const { ethers } = require('hardhat');
const { loadConfig } = require('../evm/utils');
const {
    BigNumber,
    utils: { arrayify, hexlify },
} = ethers;
const { fromB64 } = require('@mysten/bcs');
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

const getAmplifierSigners = async (config, chain) => {
    const client = await CosmWasmClient.connect(config.axelar.rpc);
    const { id: verifierSetId, verifier_set: verifierSet } = await client.queryContractSmart(
        config.axelar.contracts.MultisigProver[chain].address,
        'current_verifier_set',
    );
    const signers = Object.values(verifierSet.signers);

    const weightedSigners = signers
        .map((signer) => ({
            pubkey: arrayify(`0x${signer.pub_key.ecdsa}`),
            weight: Number(signer.weight),
        }))
        .sort((a, b) => hexlify(a.pubkey).localeCompare(hexlify(b.pubkey)));

    return {
        signers: weightedSigners,
        threshold: Number(verifierSet.threshold),
        nonce: ethers.utils.hexZeroPad(BigNumber.from(verifierSet.created_at).toHexString(), 32),
        verifierSetId,
    };
};

// Given sui client and object id, return the base64-decoded object bcs bytes
const getBcsBytesByObjectId = async (client, objectId) => {
    const response = await client.getObject({
        id: objectId,
        options: {
            showBcs: true,
        },
    });

    return fromB64(response.data.bcs.bcsBytes);
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

const findPublishedObject = (published, packageName, contractName) => {
    const packageId = published.packageId;
    return published.publishTxn.objectChanges.find((change) => change.objectType === `${packageId}::${packageName}::${contractName}`);
};

module.exports = {
    getAmplifierSigners,
    getBcsBytesByObjectId,
    loadSuiConfig,
    findPublishedObject,
};
