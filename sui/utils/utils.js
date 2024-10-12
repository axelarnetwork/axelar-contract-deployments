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
const moveDir = `${__dirname}/../move`;

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
    copyMovePackage(packageName, null, moveDir);

    const builder = new TxBuilder(client);
    await builder.publishPackageAndTransferCap(packageName, options.owner || keypair.toSuiAddress(), moveDir);
    const publishTxn = await builder.signAndExecute(keypair);

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
    const data = bcsStructs.its.ITS.parse(bcsBytes);
    const channelId = data.value.channel.id;
    return '0x' + channelId;
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

const parseExecuteDataFromTransaction = async (client, transaction, approvedMessage) => {
    // Get the transaction object from the object id
    const txObject = await client.getObject({
        id: transaction.objectId,
        options: {
            showContent: true,
        },
    });

    // Extract the fields from the transaction object
    const txFields = txObject.data.content.fields.value.fields.move_calls[0].fields;

    // Build the arguments for the move call
    // There're 5 types of arguments as mentioned in the following link https://github.com/axelarnetwork/axelar-cgp-sui/blob/72579e5c7735da61d215bd712627edad562cb82a/src/bcs.ts#L44-L49
    const txArgs = txFields.arguments.map(([argType, ...arg]) => {
        if (argType === 0) {
            return '0x' + Buffer.from(arg).toString('hex');
        } else if (argType === 1) {
            // TODO: handle pures followed by the bcs encoded form of the pure
            // throw new Error('Not implemented yet');
        } else if (argType === 2) {
            return approvedMessage;
        } else if (argType === 3) {
            // TODO: handle the payload of the contract call (to be passed into the intermediate function)
            throw new Error('Not implemented yet');
        } else if (argType === 4) {
            // TODO: handle an argument returned from a previous move call, followed by a u8 specified which call to get the return of (0 for the first transaction AFTER the one that gets ApprovedMessage out), and then another u8 specifying which argument to input.
            throw new Error('Not implemented yet');
        }

        throw new Error(`Unknown argument type: ${argType}`);
    });

    const { module_name: moduleName, name, package_id: packageId } = txFields.function.fields;

    return {
        moduleName,
        name,
        packageId,
        txArgs,
    };
};

const checkTrustedAddresses = (trustedAddresses, destinationChain, destinationAddress) => {
    if (!trustedAddresses[destinationChain] || !trustedAddresses[destinationChain].includes(destinationAddress)) {
        throw new Error(
            `Destination address ${destinationAddress} is not trusted on ${destinationChain}. Check if the given adress is trusted on ${destinationChain} or set trusted address with 'node sui/its-example.js setup-trusted-address <destination-chain> <destination-address>'`,
        );
    }
};

module.exports = {
    suiCoinId,
    getAmplifierSigners,
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
    getSquidChannelId,
    getSigners,
    getBagContentId,
    moveDir,
    getTransactionList,
    parseExecuteDataFromTransaction,
    checkTrustedAddresses,
};
