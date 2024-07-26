'use strict';

const { ethers } = require('hardhat');
const { loadConfig } = require('../evm/utils');
const {
    BigNumber,
    utils: { arrayify, hexlify },
} = ethers;
const { fromB64 } = require('@mysten/bcs');
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { updateMoveToml, copyMovePackage, TxBuilder } = require('@axelar-network/axelar-cgp-sui');

const getAmplifierSigners = async (config, chain) => {
    const client = await CosmWasmClient.connect(config.axelar.rpc);
    const { id: verifierSetId, verifier_set: verifierSet } = await client.queryContractSmart(
        config.axelar.contracts.MultisigProver[chain].address,
        'current_verifier_set',
    );
    const signers = Object.values(verifierSet.signers);

    const weightedSigners = signers
        .map((signer) => ({
            pub_key: arrayify(`0x${signer.pub_key.ecdsa}`),
            weight: Number(signer.weight),
        }))
        .sort((a, b) => hexlify(a.pub_key).localeCompare(hexlify(b.pub_key)));

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

const deployPackage = async (packageName, client, keypair, options = {}) => {
    const compileDir = `${__dirname}/move`;

    copyMovePackage(packageName, null, compileDir);

    const builder = new TxBuilder(client);
    await builder.publishPackageAndTransferCap(packageName, options.owner || keypair.toSuiAddress(), compileDir);
    const publishTxn = await builder.signAndExecute(keypair);

    const packageId = (publishTxn.objectChanges?.find((a) => a.type === 'published') ?? []).packageId;

    updateMoveToml(packageName, packageId, compileDir);
    return { packageId, publishTxn };
};

const getObjectIdsByObjectTypes = (txn, objectTypes) => {
    const objectIds = [];

    for (const objectType of objectTypes) {
        objectIds.push(txn.objectChanges.find((change) => change.objectType === objectType).objectId);
    }

    return objectIds;
};

module.exports = {
    getAmplifierSigners,
    getBcsBytesByObjectId,
    loadSuiConfig,
    deployPackage,
    getObjectIdsByObjectTypes,
};
