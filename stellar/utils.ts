'use strict';

import {
    Keypair,
    rpc,
    Horizon,
    TransactionBuilder,
    Networks,
    BASE_FEE,
    Address,
    xdr,
    nativeToScVal,
} from '@stellar/stellar-sdk';
import { downloadContractCode, VERSION_REGEX, SHORT_COMMIT_HASH_REGEX } from '../common/utils';
import { printInfo, sleep, addEnvOption, getCurrentVerifierSet } from '../common';
import { Command, Option } from 'commander';
import { ethers } from 'ethers';
import { itsCustomMigrationDataToScValV112 } from './type-utils';
const {
    utils: { arrayify, hexZeroPad, id, isHexString, keccak256 },
    BigNumber,
} = ethers;
const stellarCmd = 'stellar';
const ASSET_TYPE_NATIVE = 'native';

const AXELAR_R2_BASE_URL = 'https://static.axelar.network';

const TRANSACTION_TIMEOUT = 30;
const RETRY_WAIT = 1000; // 1 sec
const MAX_RETRIES = 30;

// TODO: Need to be migrated to Pascal Case
const SUPPORTED_CONTRACTS = new Set([
    'AxelarExample',
    'AxelarGateway',
    'AxelarOperators',
    'AxelarGasService',
    'InterchainToken',
    'TokenManager',
    'InterchainTokenService',
    'Upgrader',
    'Multicall',
    'TokenUtils'
]);

type NetworkType = 'local' | 'futurenet' | 'testnet' | 'mainnet';

interface Options {
    timeout?: number;
    verbose?: boolean;
    nativePayment?: boolean;
    estimateCost?: boolean;
    simulateTransaction?: boolean;
    ignorePrivateKey?: boolean;
    address?: string;
}

const CustomMigrationDataTypeToScValV112 = {
    InterchainTokenService: (migrationData) => itsCustomMigrationDataToScValV112(migrationData),
};

const VERSIONED_CUSTOM_MIGRATION_DATA_TYPES = {
    '1.1.2': CustomMigrationDataTypeToScValV112,
};

function getNetworkPassphrase(networkType: NetworkType) {
    switch (networkType) {
        case 'local':
            return Networks.STANDALONE;
        case 'futurenet':
            return Networks.FUTURENET;
        case 'testnet':
            return Networks.TESTNET;
        case 'mainnet':
            return Networks.PUBLIC;
        default:
            throw new Error(`Unknown network type: ${networkType}`);
    }
}

const addBaseOptions = (command: Command, options: Options = {}) => {
    addEnvOption(command);
    command.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
    command.addOption(new Option('--chain-name <chainName>', 'chain name for stellar in amplifier').default('stellar').env('CHAIN'));
    command.addOption(new Option('-v, --verbose', 'verbose output').default(false));
    command.addOption(new Option('--estimate-cost', 'estimate on-chain resources').default(false));

    if (options && !options.ignorePrivateKey) {
        command.addOption(new Option('-p, --private-key <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
    }

    if (options && options.address) {
        command.addOption(new Option('--address <address>', 'override contract address'));
    }

    return command;
};

async function buildTransaction(operation, server, wallet, networkType, options: Options = {}) {
    const account = await server.getAccount(wallet.publicKey());
    const networkPassphrase = getNetworkPassphrase(networkType);
    const builtTransaction = new TransactionBuilder(account, {
        fee: BASE_FEE,
        networkPassphrase,
    })
        .addOperation(operation)
        .setTimeout(options.timeout || TRANSACTION_TIMEOUT)
        .build();

    if (options && options.verbose) {
        printInfo('Tx', builtTransaction.toXDR());
    }

    return builtTransaction;
}

const prepareTransaction = async (operation, server, wallet, networkType, options: Options = {}) => {
    const builtTransaction = await buildTransaction(operation, server, wallet, networkType, options);

    // We use the RPC server to "prepare" the transaction. This simulating the
    // transaction, discovering the storage footprint, and updating the
    // transaction to include that footprint. If you know the footprint ahead of
    // time, you could manually use `addFootprint` and skip this step.
    const preparedTransaction = await server.prepareTransaction(builtTransaction);

    preparedTransaction.sign(wallet);

    if (options && options.verbose) {
        printInfo('Signed tx', preparedTransaction.toEnvelope().toXDR('base64'));
    }

    return preparedTransaction;
};

async function sendTransaction(tx, server, action, options: Options = {}) {
    // Submit the transaction to the Soroban-RPC server. The RPC server will
    // then submit the transaction into the network for us. Then we will have to
    // wait, polling `getTransaction` until the transaction completes.
    try {
        let sendResponse, getResponse;
        let retries = MAX_RETRIES;

        while (retries > 0) {
            sendResponse = await server.sendTransaction(tx);

            if (sendResponse.status === 'PENDING') break;

            await sleep(RETRY_WAIT);
            retries--;
        }

        printInfo(`${action} tx`, sendResponse.hash);

        if (options && options.verbose) {
            printInfo('Transaction broadcast response', JSON.stringify(sendResponse));
        }

        if (sendResponse.status !== 'PENDING') {
            throw Error(`Response: ${JSON.stringify(sendResponse, null, 2)}`);
        }

        retries = MAX_RETRIES;

        while (retries > 0) {
            getResponse = await server.getTransaction(sendResponse.hash);

            if (getResponse.status === 'SUCCESS') break;

            await sleep(RETRY_WAIT);
            retries--;
        }

        if (options && options.verbose) {
            printInfo('Transaction response', JSON.stringify(getResponse));
        }

        if (getResponse.status !== 'SUCCESS') {
            throw Error(`Transaction failed: ${getResponse.txHash}`);
        }

        // Native payment — sorobanMeta is not present, so skip parsing.
        if (options && options.nativePayment) return;

        // Make sure the transaction's resultMetaXDR is not empty
        // TODO: might be empty if the operation doesn't have a return value
        if (!getResponse.resultMetaXdr) {
            throw Error('Empty resultMetaXDR in getTransaction response');
        }

        const transactionMeta = getResponse.resultMetaXdr;
        const returnValue = transactionMeta.v3().sorobanMeta().returnValue();

        if (options && options.verbose) {
            printInfo('Transaction result', returnValue.value());
        }

        return returnValue;
    } catch (err) {
        console.log('Sending transaction failed');
        throw err;
    }
}

async function broadcast(operation, wallet, chain, action, options: Options, simulateTransaction = false) {
    const server = new rpc.Server(chain.rpc, { allowHttp: chain.networkType === 'local' });

    if (options && options.nativePayment) {
        const tx = await buildTransaction(operation, server, wallet, chain.networkType, options);
        tx.sign(wallet);
        return sendTransaction(tx, server, action, options);
    }
    if (options && options.estimateCost) {
        const tx = await buildTransaction(operation, server, wallet, chain.networkType, options);
        const resourceCost = await estimateCost(tx, server);
        printInfo('Gas cost', JSON.stringify(resourceCost, null, 2));
        return;
    }

    if (simulateTransaction) {
        const tx = await buildTransaction(operation, server, wallet, chain.networkType, options);
        try {
            const response = await server.simulateTransaction(tx);
            printInfo('successfully simulated tx', `action: ${action}, networkType: ${chain.networkType}, chainName: ${chain.name}`);
            return response;
        } catch (error) {
            throw new Error(error);
        }
    }

    const tx = await prepareTransaction(operation, server, wallet, chain.networkType, options);
    return sendTransaction(tx, server, action, options);
}

function getAssetCode(balance, chain) {
    return balance.asset_type === 'native' ? chain.tokenSymbol : balance.asset_code;
}

/*
 * To enable connecting to the local network, allowHttp needs to be set to true.
 * This is necessary because the local network does not accept HTTPS requests.
 */
function getRpcOptions(chain) {
    return {
        allowHttp: chain.networkType === 'local',
    };
}

async function getWallet(chain, options) {
    const keypair = Keypair.fromSecret(options.privateKey);
    const address = keypair.publicKey();
    const provider = new rpc.Server(chain.rpc, getRpcOptions(chain));
    const horizonServer = new Horizon.Server(chain.horizonRpc, getRpcOptions(chain));
    const balances = await getBalances(horizonServer, address);

    printInfo('Wallet address', address);
    printInfo('Wallet balances', balances.map((balance) => `${balance.balance} ${getAssetCode(balance, chain)}`).join('  '));
    printInfo('Wallet sequence', await provider.getAccount(address).then((account) => account.sequenceNumber()));

    return keypair;
}

async function getBalances(horizonServer, address) {
    const response = await horizonServer
        .accounts()
        .accountId(address)
        .call()
        .catch((error) => {
            if (error?.response?.status === 404) {
                return { balances: [] };
            }

            throw error;
        });
    return response.balances;
}

async function getNativeBalance(chain, address) {
    const horizonServer = new Horizon.Server(chain.horizonRpc, getRpcOptions(chain));
    const balances = await getBalances(horizonServer, address);
    const native = balances.find((balance) => balance.asset_type === ASSET_TYPE_NATIVE);
    return native ? parseFloat(native.balance) : 0;
}

async function estimateCost(tx, server) {
    await server.simulateTransaction(tx);

    const response = await server._simulateTransaction(tx);

    if (response.error) {
        throw new Error(response.error);
    }

    const events = response.events.map((event) => {
        const e = xdr.DiagnosticEvent.fromXDR(event, 'base64');

        if (e.event().type().name === 'diagnostic') return 0;

        return e.toXDR().length;
    });

    const eventsAndReturnValueSize =
        events.reduce((accumulator, currentValue) => accumulator + currentValue, 0) + // events
        Buffer.from(response.results[0].xdr, 'base64').length; // return value size

    const sorobanTransactionData = xdr.SorobanTransactionData.fromXDR(response.transactionData, 'base64');

    return {
        // the first two lines are incorrect. use sorobanTransactionData instead of `cost`
        cpu_instructions: Number(response.cost.cpuInsns),
        ram: Number(response.cost.memBytes),

        min_resource_fee: response.minResourceFee,
        ledger_read_bytes: sorobanTransactionData.resources().readBytes(),
        ledger_write_bytes: sorobanTransactionData.resources().writeBytes(),
        ledger_entry_reads: sorobanTransactionData.resources().footprint().readOnly().length,
        ledger_entry_writes: sorobanTransactionData.resources().footprint().readWrite().length,
        events_and_return_value_size: eventsAndReturnValueSize,
        transaction_size: Buffer.from(response.transactionData, 'base64').length,
    };
}

const getAmplifierVerifiers = async (config, chain) => {
    const { verifierSetId, verifierSet, signers } = await getCurrentVerifierSet(config, chain);

    // Include pubKey for sorting, sort based on pubKey, then remove pubKey after sorting.
    const weightedSigners = signers
        .map((signer) => ({
            signer: Address.account(Buffer.from(arrayify(`0x${signer.pub_key.ed25519}`))).toString(),
            weight: Number(signer.weight),
            pubKey: signer.pub_key.ed25519,
        }))
        .sort((a, b) => a.pubKey.localeCompare(b.pubKey))
        .map(({ signer, weight }) => ({ signer, weight }));

    return {
        signers: weightedSigners,
        threshold: Number(verifierSet.threshold),
        nonce: arrayify(ethers.utils.hexZeroPad(BigNumber.from(verifierSet.created_at).toHexString(), 32)),
        verifierSetId,
    };
};

const getNewSigners = async (wallet, config, chain, options) => {
    if (options && options.signers === 'wallet') {
        return {
            nonce: options.newNonce ? arrayify(id(options.newNonce)) : Array(32).fill(0),
            signers: [
                {
                    signer: wallet.publicKey(),
                    weight: 1,
                },
            ],
            threshold: 1,
        };
    }

    return getAmplifierVerifiers(config, chain.axelarId);
};

function serializeValue(value) {
    if (value instanceof xdr.ScAddress) {
        return Address.fromScAddress(value).toString();
    }

    if (value instanceof Uint8Array) {
        return Buffer.from(value).toString('hex');
    }

    if (Array.isArray(value)) {
        return value.map(serializeValue);
    }

    if (typeof value === 'bigint') {
        return value.toString();
    }

    if (typeof value === 'object') {
        return Object.entries(value).reduce((acc, [key, val]) => {
            acc[key] = serializeValue(val);
            return acc;
        }, {});
    }

    return value;
}

const createAuthorizedFunc = (contractAddress, functionName, args) =>
    xdr.SorobanAuthorizedFunction.sorobanAuthorizedFunctionTypeContractFn(
        new xdr.InvokeContractArgs({
            contractAddress: contractAddress.toScAddress(),
            functionName,
            args,
        }),
    );

function addressToScVal(addressString) {
    return nativeToScVal(Address.fromString(addressString), { type: 'address' });
}

function hexToScVal(hexString) {
    return nativeToScVal(Buffer.from(arrayify(hexString)), { type: 'bytes' });
}

function tokenToScVal(tokenAddress, tokenAmount) {
    return tokenAmount === 0
        ? nativeToScVal(null, { type: 'null' })
        : nativeToScVal(
              {
                  address: Address.fromString(tokenAddress),
                  amount: tokenAmount,
              },
              {
                  type: {
                      address: ['symbol', 'address'],
                      amount: ['symbol', 'i128'],
                  },
              },
          );
}

function tokenMetadataToScVal(decimal, name, symbol) {
    return nativeToScVal(
        {
            decimal,
            name,
            symbol,
        },
        {
            type: {
                decimal: ['symbol', 'u32'],
                name: ['symbol', 'string'],
                symbol: ['symbol', 'string'],
            },
        },
    );
}

function saltToBytes32(salt) {
    return isHexString(salt) ? hexZeroPad(salt, 32) : keccak256(salt);
}

const getContractR2Url = (contractName, version) => {
    if (!SUPPORTED_CONTRACTS.has(contractName)) {
        throw new Error(`Unsupported contract ${contractName} for versioned deployment`);
    }

    const dirPath = `stellar-${pascalToKebab(contractName)}`;
    const fileName = dirPath.replace(/-/g, '_');

    if (VERSION_REGEX.test(version)) {
        // Extra v for versioned releases in R2
        return `${AXELAR_R2_BASE_URL}/releases/stellar/${dirPath}/v${version}/wasm/${fileName}.wasm`;
    }

    if (SHORT_COMMIT_HASH_REGEX.test(version)) {
        return `${AXELAR_R2_BASE_URL}/releases/stellar/${dirPath}/${version}/wasm/${fileName}.wasm`;
    }

    throw new Error(`Invalid version format: ${version}. Must be a semantic version (ommit prefix v) or a commit hash`);
};

function getContractArtifactPath(artifactPath, contractName) {
    const basePath = artifactPath.slice(0, artifactPath.lastIndexOf('/') + 1);
    const fileName = `stellar_${pascalToKebab(contractName).replace(/-/g, '_')}.optimized.wasm`;
    return basePath + fileName;
}

const getContractCodePath = async (options, contractName) => {
    if (options && options.artifactPath) {
        if (contractName === 'InterchainToken' || contractName === 'TokenManager') {
            return getContractArtifactPath(options.artifactPath, contractName);
        }

        return options.artifactPath;
    }

    if (options && options.version) {
        const url = getContractR2Url(contractName, options.version);
        return downloadContractCode(url, contractName, options.version);
    }

    throw new Error('Either --artifact-path or --version must be provided');
};

const getUploadContractCodePath = async (options, contractName) => {
    if (options && options.artifactPath) return options.artifactPath;

    if (options && options.version) {
        const url = getContractR2Url(contractName, options.version);
        return downloadContractCode(url, contractName, options.version);
    }

    throw new Error('Either --artifact-path or --version must be provided');
};

function isValidAddress(address) {
    try {
        // try conversion
        Address.fromString(address);
        return true;
    } catch {
        return false;
    }
}

function BytesToScVal(wasmHash) {
    return nativeToScVal(Buffer.from(wasmHash, 'hex'), {
        type: 'bytes',
    });
}

/**
 * Converts a PascalCase or camelCase string to kebab-case.
 *
 * - Inserts a hyphen (`-`) before each uppercase letter (except the first letter).
 * - Converts all letters to lowercase.
 * - Works for PascalCase, camelCase, and mixed-case strings.
 *
 * @param {string} str - The input string in PascalCase or camelCase.
 * @returns {string} - The converted string in kebab-case.
 *
 * @example
 * pascalToKebab("PascalCase");        // "pascal-case"
 * pascalToKebab("camelCase");         // "camel-case"
 * pascalToKebab("XMLHttpRequest");    // "xml-http-request"
 * pascalToKebab("exampleString");     // "example-string"
 * pascalToKebab("already-kebab");     // "already-kebab" (unchanged)
 * pascalToKebab("noChange");          // "no-change"
 * pascalToKebab("single");            // "single" (unchanged)
 * pascalToKebab("");                  // "" (empty string case)
 */
function pascalToKebab(str) {
    return str.replace(/([A-Z])/g, (match, _, offset) => (offset > 0 ? `-${match.toLowerCase()}` : match.toLowerCase()));
}

function sanitizeMigrationData(migrationData, version, contractName) {
    if (migrationData === null || migrationData === '()') return null;

    try {
        return Address.fromString(migrationData);
    } catch (_) {
        // not an address, continue to next parsing attempt
    }

    let parsed;

    try {
        parsed = JSON.parse(migrationData);
    } catch (_) {
        // not json, keep as string
        return migrationData;
    }

    if (Array.isArray(parsed)) {
        return parsed.map((value) => sanitizeMigrationData(value, version, contractName));
    }

    const custom = customMigrationData(parsed, version, contractName);

    if (custom) {
        return custom;
    }

    if (parsed !== null && typeof parsed === 'object') {
        return Object.fromEntries(Object.entries(parsed).map(([key, value]) => [key, sanitizeMigrationData(value, version, contractName)]));
    }

    printInfo('Sanitized migration data', parsed);

    return parsed;
}

function customMigrationData(migrationDataObj, version, contractName) {
    if (!version || !VERSIONED_CUSTOM_MIGRATION_DATA_TYPES[version] || !VERSIONED_CUSTOM_MIGRATION_DATA_TYPES[version][contractName]) {
        return null;
    }

    const customMigrationDataTypeToScVal = VERSIONED_CUSTOM_MIGRATION_DATA_TYPES[version][contractName];

    try {
        printInfo(`Retrieving custom migration data for ${contractName}`);
        return customMigrationDataTypeToScVal(migrationDataObj);
    } catch (error) {
        throw new Error(`Failed to convert custom migration data for ${contractName}: ${error}`);
    }
}

async function generateKeypair(options) {
    switch (options.signatureScheme) {
        case 'ed25519':
            return Keypair.random();

        default: {
            throw new Error(`Unsupported scheme: ${options.signatureScheme}`);
        }
    }
}

function isFriendbotSupported(networkType) {
    switch (networkType) {
        case 'local':
        case 'futurenet':
        case 'testnet':
            return true;
        case 'mainnet':
            return false;
        default:
            throw new Error(`Unknown network type: ${networkType}`);
    }
}

function assetToScVal(asset) {
    return nativeToScVal(
        Buffer.from(asset.toXDRObject().toXDR('base64'), 'base64'),
        { type: 'bytes' }
    );
}

module.exports = {
    stellarCmd,
    ASSET_TYPE_NATIVE,
    buildTransaction,
    prepareTransaction,
    sendTransaction,
    broadcast,
    getWallet,
    estimateCost,
    getNetworkPassphrase,
    addBaseOptions,
    getNewSigners,
    serializeValue,
    getBalances,
    getNativeBalance,
    createAuthorizedFunc,
    addressToScVal,
    hexToScVal,
    tokenToScVal,
    tokenMetadataToScVal,
    saltToBytes32,
    getContractCodePath,
    getUploadContractCodePath,
    isValidAddress,
    SUPPORTED_CONTRACTS,
    BytesToScVal,
    pascalToKebab,
    sanitizeMigrationData,
    generateKeypair,
    isFriendbotSupported,
    assetToScVal,
};
