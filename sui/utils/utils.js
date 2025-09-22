'use strict';

const { ethers } = require('hardhat');
const toml = require('toml');
const { printInfo, printError, printWarn, validateParameters, writeJSON, getCurrentVerifierSet } = require('../../common/utils');
const {
    BigNumber,
    utils: { arrayify, hexlify, toUtf8Bytes, keccak256 },
    constants: { HashZero },
} = ethers;
const fs = require('fs');
const { fromB64, toB64 } = require('@mysten/bcs');
const {
    updateMoveToml,
    copyMovePackage,
    TxBuilder,
    bcsStructs,
    getDefinedSuiVersion,
    getInstalledSuiVersion,
    STD_PACKAGE_ID,
    SUI_PACKAGE_ID,
} = require('@axelar-network/axelar-cgp-sui');
const { Transaction } = require('@mysten/sui/transactions');
const { broadcast, broadcastFromTxBuilder } = require('./sign-utils');

const suiPackageAddress = '0x2';
const suiClockAddress = '0x6';
const suiCoinId = '0x2::sui::SUI';
const moveDir = `${__dirname}/../move`;

const getAmplifierVerifiers = async (config, chain) => {
    const { verifierSetId, verifierSet, signers } = await getCurrentVerifierSet(config.axelar, chain);

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
    copyMovePackage(packageName, null, moveDir);

    const builder = new TxBuilder(client);
    await builder.publishPackageAndTransferCap(packageName, options.owner || keypair.toSuiAddress(), moveDir);
    const publishTxn = await broadcastFromTxBuilder(builder, keypair, `Deployed ${packageName} Package`, options);

    const packageId = (findPublishedObject(publishTxn) ?? []).packageId;

    updateMoveToml(packageName, packageId, moveDir);
    return { packageId, publishTxn };
};

const findPublishedObject = (publishTxn) => {
    return publishTxn.objectChanges.find((change) => change.type === 'published');
};

const checkSuiVersionMatch = () => {
    const installedVersion = getInstalledSuiVersion();
    const definedVersion = getDefinedSuiVersion();

    if (installedVersion !== definedVersion) {
        printWarn('Version mismatch detected:');
        printWarn(`- Installed SUI version: ${installedVersion}`);
        printWarn(`- Version used for tests: ${definedVersion}`);
        printWarn(`Please download the correct version (${definedVersion}) from https://github.com/MystenLabs/sui/releases`);
    }
};

const readMoveToml = (moveDir) => {
    return fs.readFileSync(`${__dirname}/../../node_modules/@axelar-network/axelar-cgp-sui/move/${moveDir}/Move.toml`, 'utf8');
};

const readMovePackageName = (moveDir) => {
    try {
        const { package: movePackage } = toml.parse(readMoveToml(moveDir));

        return movePackage.name;
    } catch (err) {
        printError('Error reading TOML file');
        throw err;
    }
};

const getObjectIdsByObjectTypes = (txn, objectTypes) =>
    objectTypes.map((objectType) => {
        const objectId = txn.objectChanges.find((change) => change.objectType?.includes(objectType))?.objectId;

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
    const data = bcsStructs.its.InterchainTokenService.parse(bcsBytes);
    const channelId = data.value.channel.id;
    return '0x' + channelId;
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

    return getAmplifierVerifiers(config, chain);
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

const findOwnedObjectIdByType = async (client, ownerAddress, objectType) => {
    const ownedObjects = await client.getOwnedObjects({
        owner: ownerAddress,
        filter: {
            StructType: objectType,
        },
        options: {
            showContent: true,
        },
    });

    if (ownedObjects.data.length !== 1) {
        throw new Error(`Expecting exactly one object of type ${objectType} owned by ${ownerAddress}`);
    }

    const targetObject = ownedObjects.data[0];

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

const getTransactionList = async (client, discoveryObjectId) => {
    const tableBcsBytes = await getBcsBytesByObjectId(client, discoveryObjectId);
    const data = bcsStructs.relayerDiscovery.RelayerDiscovery.parse(tableBcsBytes);
    const tableId = data.value.configurations.id;

    const tableResult = await client.getDynamicFields({
        parentId: tableId,
    });

    return tableResult.data;
};

const parseDiscoveryInfo = (chain) => {
    return {
        discovery: chain.RelayerDiscovery.objects.RelayerDiscovery,
        packageId: chain.RelayerDiscovery.address,
    };
};

const parseGatewayInfo = (chain) => {
    return {
        gateway: chain.AxelarGateway.objects.Gateway,
        packageId: chain.AxelarGateway.address,
    };
};

const checkTrustedAddresses = async (destinationChain) => {
    // TODO: another PR adds functionality that will enable this
};

const getStructs = async (client, packageId) => {
    const packageData = await client.getObject({ id: packageId, options: { showBcs: true } });
    const structs = {};

    for (const type of packageData.data.bcs.typeOriginTable) {
        structs[type.datatype_name] = `${type.package}::${type.module_name}::${type.datatype_name}`;
    }

    return structs;
};

const saveGeneratedTx = async (tx, message, client, options) => {
    const { txFilePath } = options;
    validateParameters({ isNonEmptyString: { txFilePath } });

    const txBytes = await tx.build({ client });
    const txB64Bytes = toB64(txBytes);

    writeJSON({ message, status: 'PENDING', unsignedTx: txB64Bytes }, txFilePath);
    printInfo(`Unsigned transaction`, txFilePath);
};

const isAllowed = async (client, keypair, chain, exec, options) => {
    const addError = (tx) => {
        tx.moveCall({
            target: `${STD_PACKAGE_ID}::ascii::char`,
            arguments: [tx.pure.u8(128)],
        });
    };

    const tx = new Transaction();
    exec(tx);
    addError(tx);

    try {
        await broadcast(client, keypair, tx, undefined, options);
    } catch (e) {
        const errorMessage = e.cause.effects.status.error;
        let regexp = /address: (.*?),/;
        const packageId = `0x${regexp.exec(errorMessage)[1]}`;

        regexp = /Identifier\("(.*?)"\)/;
        const module = regexp.exec(errorMessage)[1];

        regexp = /Some\("(.*?)"\)/;
        const functionName = regexp.exec(errorMessage)[1];

        if (packageId === chain.contracts.VersionControl.address && module === 'version_control' && functionName === 'check') {
            regexp = /Some\(".*?"\) \}, (.*?)\)/;

            if (parseInt(regexp.exec(errorMessage)[1]) === 9223372539365950000) {
                return false;
            }
        }

        let suiPackageAddress = SUI_PACKAGE_ID;

        while (suiPackageAddress.length < 66) {
            suiPackageAddress = suiPackageAddress.substring(0, suiPackageAddress.length - 1) + '02';
        }

        if (
            packageId === suiPackageAddress &&
            module === 'dynamic_field' &&
            (functionName === 'borrow_child_object_mut' || functionName === 'borrow_child_object')
        ) {
            regexp = /Some\(".*?"\) \}, (.*?)\)/;

            if (parseInt(regexp.exec(errorMessage)[1]) === 2) {
                return false;
            }
        }
    }

    return true;
};

const getAllowedFunctions = async (client, versionedObjectId) => {
    const response = await client.getObject({
        id: versionedObjectId,
        options: {
            showContent: true,
        },
    });
    const allowedFunctionsArray = response.data.content.fields.value.fields.version_control.fields.allowed_functions;
    return allowedFunctionsArray.map((allowedFunctions) => allowedFunctions.fields.contents);
};

module.exports = {
    suiCoinId,
    isGasToken,
    paginateAll,
    suiPackageAddress,
    suiClockAddress,
    checkSuiVersionMatch,
    findOwnedObjectIdByType,
    getBcsBytesByObjectId,
    deployPackage,
    findPublishedObject,
    readMovePackageName,
    getObjectIdsByObjectTypes,
    getSingletonChannelId,
    getItsChannelId,
    getSigners,
    getBagContentId,
    moveDir,
    getTransactionList,
    parseDiscoveryInfo,
    parseGatewayInfo,
    checkTrustedAddresses,
    getStructs,
    saveGeneratedTx,
    isAllowed,
    getAllowedFunctions,
};
