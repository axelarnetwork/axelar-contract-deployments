'use strict';

const { ethers } = require('hardhat');
const toml = require('toml');
const { printInfo, printError, printWarn } = require('../../common/utils');
const {
    BigNumber,
    utils: { arrayify, hexlify, toUtf8Bytes, keccak256 },
    constants: { HashZero },
} = ethers;
const fs = require('fs');
const { fromB64 } = require('@mysten/bcs');
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const {
    updateMoveToml,
    copyMovePackage,
    TxBuilder,
    bcsStructs,
    getDefinedSuiVersion,
    getInstalledSuiVersion,
} = require('@axelar-network/axelar-cgp-sui');

const suiPackageAddress = '0x2';
const suiClockAddress = '0x6';
const suiCoinId = '0x2::sui::SUI';

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

const deployPackage = async (packageName, client, keypair, options = {}) => {
    const compileDir = `${__dirname}/../move`;

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

const checkSuiVersionMatch = () => {
    const installedVersion = getInstalledSuiVersion();
    const definedVersion = getDefinedSuiVersion();

    if (installedVersion !== definedVersion) {
        printWarn('Version mismatch detected:');
        printWarn(`- Installed SUI version: ${installedVersion}`);
        printWarn(`- Version used for tests: ${definedVersion}`);
    }
};

const readMovePackageName = (moveDir) => {
    try {
        const moveToml = fs.readFileSync(
            `${__dirname}/../../node_modules/@axelar-network/axelar-cgp-sui/move/${moveDir}/Move.toml`,
            'utf8',
        );

        const { package: movePackage } = toml.parse(moveToml);

        return movePackage.name;
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
const getSingletonChannelId = async (client, singletonObjectId) => {
    const bcsBytes = await getBcsBytesByObjectId(client, singletonObjectId);
    const data = bcsStructs.gmp.Singleton.parse(bcsBytes);
    return '0x' + data.channel.id;
};

const getItsChannelId = async (client, itsObjectId) => {
    const bcsBytes = await getBcsBytesByObjectId(client, itsObjectId);
    const data = bcsStructs.its.ITS.parse(bcsBytes);
    return '0x' + data.channel.id;
};

const getSquidChannelId = async (client, squidObjectId) => {
    const bcsBytes = await getBcsBytesByObjectId(client, squidObjectId);
    const data = bcsStructs.squid.Squid.parse(bcsBytes);
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

const isGasToken = (coinType) => {
    return coinType === suiCoinId;
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

const findOwnedObjectId = async (client, ownerAddress, objectType) => {
    const ownedObjects = await client.getOwnedObjects({
        owner: ownerAddress,
        options: {
            showContent: true,
        },
    });

    const targetObject = ownedObjects.data.find(({ data }) => data.content.type === objectType);

    if (!targetObject) {
        throw new Error(`No object found for type: ${objectType}`);
    }

    return targetObject.data.content.fields.id.id;
};

const getBagContentId = async (client, objectType, bagId, bagName) => {
    const result = await client.getDynamicFields({
        parentId: bagId,
        name: bagName,
    });

    const objectId = result.data.find((cap) => cap.objectType === objectType)?.objectId;

    if (!objectId) {
        throw new Error(`${objectType} not found in the capabilities bag`);
    }

    const objectDetails = await client.getObject({
        id: objectId,
        options: {
            showContent: true,
        },
    });

    return objectDetails.data.content.fields.value.fields.id.id;
};

module.exports = {
    suiCoinId,
    getAmplifierSigners,
    isGasToken,
    paginateAll,
    suiPackageAddress,
    suiClockAddress,
    checkSuiVersionMatch,
    findOwnedObjectId,
    getBcsBytesByObjectId,
    deployPackage,
    findPublishedObject,
    readMovePackageName,
    getObjectIdsByObjectTypes,
    getSingletonChannelId,
    getItsChannelId,
    getSquidChannelId,
    getSigners,
    getBagContentId,
};
