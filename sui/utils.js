'use strict';

const { ethers } = require('hardhat');
const { printInfo, loadConfig, printError } = require('../common/utils');
const {
    BigNumber,
    utils: { arrayify, hexlify, toUtf8Bytes, keccak256 },
    constants: { HashZero },
} = ethers;
const fs = require('fs');
const { fromB64 } = require('@mysten/bcs');
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { updateMoveToml, copyMovePackage, TxBuilder } = require('@axelar-network/axelar-cgp-sui');
const { singletonStruct } = require('./types-utils');

const suiPackageAddress = '0x2';
const suiClockAddress = '0x6';

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

const findPublishedObject = (published, packageDir, contractName) => {
    const packageId = published.packageId;
    return published.publishTxn.objectChanges.find((change) => change.objectType === `${packageId}::${packageDir}::${contractName}`);
};

const readMovePackageName = (moveDir) => {
    try {
        const moveToml = fs.readFileSync(`${__dirname}/../node_modules/@axelar-network/axelar-cgp-sui/move/${moveDir}/Move.toml`, 'utf8');

        const nameMatch = moveToml.match(/^\s*name\s*=\s*"([^"]+)"/m);

        if (nameMatch && nameMatch[1]) {
            return nameMatch[1];
        }

        throw new Error('Package name not found in TOML file');
    } catch (err) {
        printError('Error reading TOML file');
        throw err;
    }
};

const getObjectIdsByObjectTypes = (txn, objectTypes) =>
    objectTypes.map((objectType) => {
        const objectId = txn.objectChanges.find((change) => change.objectType === objectType)?.objectId;

        if (!objectId) {
            throw new Error(`No object found for type: ${objectType}`);
        }

        return objectId;
    });

// Parse bcs bytes from singleton object which is created when the Test contract is deployed
const getChannelId = async (client, singletonObjectId) => {
    const bcsBytes = await getBcsBytesByObjectId(client, singletonObjectId);
    const data = singletonStruct.parse(bcsBytes);
    return '0x' + data.channel.id;
};

const getSigners = async (keypair, config, chain, options) => {
    if (options.signers === 'wallet') {
        const pubKey = keypair.getPublicKey().toRawBytes();
        printInfo('Using wallet pubkey as the signer for the gateway', hexlify(pubKey));

        if (keypair.getKeyScheme() !== 'Secp256k1') {
            throw new Error('Only Secp256k1 pubkeys are supported by the gateway');
        }

        return {
            signers: [{ pub_key: pubKey, weight: 1 }],
            threshold: 1,
            nonce: options.nonce ? keccak256(toUtf8Bytes(options.nonce)) : HashZero,
        };
    } else if (options.signers) {
        printInfo('Using provided signers', options.signers);

        const signers = JSON.parse(options.signers);
        return {
            signers: signers.signers.map(({ pub_key: pubKey, weight }) => {
                return { pub_key: arrayify(pubKey), weight };
            }),
            threshold: signers.threshold,
            nonce: arrayify(signers.nonce) || HashZero,
        };
    }

    return getAmplifierSigners(config, chain);
};

module.exports = {
    suiPackageAddress,
    suiClockAddress,
    getAmplifierSigners,
    getBcsBytesByObjectId,
    loadSuiConfig,
    deployPackage,
    findPublishedObject,
    readMovePackageName,
    getObjectIdsByObjectTypes,
    getChannelId,
    getSigners,
};
