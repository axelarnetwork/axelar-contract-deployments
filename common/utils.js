'use strict';

const { existsSync, mkdirSync, writeFileSync, readFileSync } = require('fs');
const path = require('path');
const { outputJsonSync } = require('fs-extra');
const chalk = require('chalk');
const https = require('https');
const http = require('http');
const readlineSync = require('readline-sync');
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const { ethers } = require('hardhat');
const {
    utils: { keccak256, hexlify, defaultAbiCoder },
} = ethers;
const { normalizeBech32 } = require('@cosmjs/encoding');
const fetch = require('node-fetch');
const StellarSdk = require('@stellar/stellar-sdk');
const bs58 = require('bs58');
const { AsyncLocalStorage } = require('async_hooks');
const { cvToHex, principalCV } = require('@stacks/transactions');

const pascalToSnake = (str) => str.replace(/([A-Z])/g, (group) => `_${group.toLowerCase()}`).replace(/^_/, '');

const pascalToKebab = (str) => str.replace(/([A-Z])/g, (group) => `-${group.toLowerCase()}`).replace(/^-/, '');

const kebabToPascal = (str) => str.replace(/-./g, (match) => match.charAt(1).toUpperCase()).replace(/^./, (match) => match.toUpperCase());

const VERSION_REGEX = /^\d+\.\d+\.\d+$/;
const SHORT_COMMIT_HASH_REGEX = /^[a-f0-9]{7,}$/;
const SVM_BASE58_ADDRESS_REGEX = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;

function loadConfig(env) {
    return require(`${__dirname}/../axelar-chains-config/info/${env}.json`);
}

function saveConfig(config, env) {
    writeJSON(config, `${__dirname}/../axelar-chains-config/info/${env}.json`);
}

const writeJSON = (data, name) => {
    outputJsonSync(name, data, {
        spaces: 2,
        EOL: '\n',
    });
};

let asyncLocalLoggerStorage = new AsyncLocalStorage();

const printMsg = (msg) => {
    const streams = asyncLocalLoggerStorage?.getStore();
    if (streams?.stdStream) {
        streams.stdStream.write(`${msg}\n\n`);
    } else {
        console.log(`${msg}\n`);
    }
};

const printErrorMsg = (msg) => {
    const streams = asyncLocalLoggerStorage?.getStore();
    if (streams?.errorStream && streams?.stdStream) {
        streams.errorStream.write(`${msg}\n\n`);
        streams.stdStream.write(`${msg}\n\n`);
    } else {
        console.log(`${msg}\n`);
    }
};

const printInfo = (msg, info = '', colour = chalk.green) => {
    if (typeof info === 'boolean') {
        info = String(info);
    } else if (Array.isArray(info) || typeof info === 'object') {
        info = JSON.stringify(info, null, 2);
    }

    if (info) {
        printMsg(`${msg}: ${colour(info)}`);
    } else {
        printMsg(`${msg}`);
    }
};

const printWarn = (msg, info = '') => {
    if (info) {
        msg = `${msg}: ${info}`;
    }

    printMsg(`${chalk.italic.yellow(msg)}`);
};

const printError = (msg, info = '') => {
    if (info) {
        msg = `${msg}: ${info}`;
    }

    printErrorMsg(`${chalk.bold.red(msg)}`);
};

const printHighlight = (msg, info = '', colour = chalk.bgBlue) => {
    if (info) {
        msg = `${msg}: ${info}`;
    }

    printMsg(`${colour(msg)}`);
};

const printDivider = (char = '-', width = process.stdout.columns, colour = chalk.bold.white) => {
    printMsg(colour(char.repeat(width)));
};

function printLog(log) {
    printMsg(JSON.stringify({ log }, null, 2));
}

const isString = (arg) => {
    return typeof arg === 'string';
};

const isNonEmptyString = (arg) => {
    return isString(arg) && arg !== '';
};

const isStringLowercase = (arg) => {
    return isNonEmptyString(arg) && arg === arg.toLowerCase();
};

const isStringArray = (arr) => Array.isArray(arr) && arr.every(isString);

const isNumber = (arg) => {
    return Number.isInteger(arg);
};

const isValidNumber = (arg) => {
    return !isNaN(parseInt(arg)) && isFinite(arg);
};

const isValidDecimal = (arg) => {
    return !isNaN(parseFloat(arg)) && isFinite(arg);
};

const isNumberArray = (arr) => {
    if (!Array.isArray(arr)) {
        return false;
    }

    for (const item of arr) {
        if (!isNumber(item)) {
            return false;
        }
    }

    return true;
};

const isNonEmptyStringArray = (arr) => {
    if (!Array.isArray(arr)) {
        return false;
    }

    for (const item of arr) {
        if (typeof item !== 'string') {
            return false;
        }
    }

    return true;
};

function copyObject(obj) {
    return JSON.parse(JSON.stringify(obj));
}

const httpGet = (url) => {
    return new Promise((resolve, reject) => {
        (url.startsWith('https://') ? https : http).get(url, (res) => {
            const { statusCode } = res;
            const contentType = res.headers['content-type'];
            let error;

            if (statusCode !== 200 && statusCode !== 301) {
                error = new Error('Request Failed.\n' + `Request: ${url}\nStatus Code: ${statusCode}`);
            } else if (!/^application\/json/.test(contentType)) {
                error = new Error('Invalid content-type.\n' + `Expected application/json but received ${contentType}`);
            }

            if (error) {
                res.resume();
                reject(error);
                return;
            }

            res.setEncoding('utf8');
            let rawData = '';
            res.on('data', (chunk) => {
                rawData += chunk;
            });
            res.on('end', () => {
                try {
                    const parsedData = JSON.parse(rawData);
                    resolve(parsedData);
                } catch (e) {
                    reject(e);
                }
            });
        });
    });
};

const httpPost = async (url, data) => {
    const response = await fetch(url, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(data),
    });
    return response.json();
};

const callAxelarscanApi = async (config, method, data, time = 10000) => {
    return timeout(
        httpPost(`${config.axelar.axelarscanApi}/${method}`, data),
        time,
        new Error(`Timeout calling Axelarscan API: ${method}`),
    );
};

const itsHubContractAddress = (axelar) => {
    return axelar?.contracts?.InterchainTokenService?.address;
};

/**
 * Parses the input string into an array of arguments, recognizing and converting
 * to the following types: boolean, number, array, and string.
 *
 * @param {string} args - The string of arguments to parse.
 *
 * @returns {Array} - An array containing parsed arguments.
 *
 * @example
 * const input = "hello true 123 [1,2,3]";
 * const output = parseArgs(input);
 * console.log(output); // Outputs: [ 'hello', true, 123, [ 1, 2, 3] ]
 */
const parseArgs = (args) => {
    return args
        .split(/\s+/)
        .filter((item) => item !== '')
        .map((arg) => {
            if (arg.startsWith('[') && arg.endsWith(']')) {
                return JSON.parse(arg);
            } else if (arg === 'true') {
                return true;
            } else if (arg === 'false') {
                return false;
            } else if (!isNaN(arg) && !arg.startsWith('0x')) {
                return Number(arg);
            }

            return arg;
        });
};

function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

function timeout(prom, time, exception) {
    let timer;

    // Racing the promise with a timer
    // If the timer resolves first, the promise is rejected with the exception
    return Promise.race([prom, new Promise((resolve, reject) => (timer = setTimeout(reject, time, exception)))]).finally(() =>
        clearTimeout(timer),
    );
}

/**
 * Determines if a given input is a valid keccak256 hash.
 *
 * @param {string} input - The string to validate.
 * @returns {boolean} - Returns true if the input is a valid keccak256 hash, false otherwise.
 */
function isKeccak256Hash(input) {
    // Ensure it's a string of 66 characters length and starts with '0x'
    if (typeof input !== 'string' || input.length !== 66 || input.slice(0, 2) !== '0x') {
        return false;
    }

    // Ensure all characters after the '0x' prefix are hexadecimal (0-9, a-f, A-F)
    const hexPattern = /^[a-fA-F0-9]{64}$/;

    return hexPattern.test(input.slice(2));
}

/**
 * Validate if the input string matches the time format YYYY-MM-DDTHH:mm:ss
 *
 * @param {string} timeString - The input time string.
 * @return {boolean} - Returns true if the format matches, false otherwise.
 */
function isValidTimeFormat(timeString) {
    const regex = /^\d{4}-(?:0[1-9]|1[0-2])-(?:0[1-9]|1\d|2\d|3[01])T(?:[01]\d|2[0-3]):[0-5]\d:[0-5]\d$/;

    if (timeString === '0') {
        return true;
    }

    return regex.test(timeString);
}

/**
 * Validate if the given address or array of addresses are valid Stellar addresses.
 *
 * A valid Stellar address is either:
 * - a valid Stellar account address (starts with 'G')
 * - a valid Stellar contract address (starts with 'C')
 *
 * @param {string|string[]} addresses - A single Stellar address or an array of Stellar addresses.
 * @returns {boolean} - True if the address or all addresses are valid, otherwise false.
 */
function isValidStellarAddress(addresses) {
    if (typeof addresses === 'string') {
        return isValidStellarAccount(addresses) || isValidStellarContract(addresses);
    }

    if (Array.isArray(addresses)) {
        return addresses.every((address) => isValidStellarAccount(address) || isValidStellarContract(address));
    }

    return false;
}

/**
 * Validate if the given address is a Stellar account address.
 *
 * A valid Stellar account address:
 * - Is a 56-character Base32-encoded string starting with 'G'.
 *
 * @param {string} address - The input Stellar account address.
 * @returns {boolean} - True if the address is a valid Stellar account, otherwise false.
 */
function isValidStellarAccount(address) {
    return StellarSdk.StrKey.isValidEd25519PublicKey(address);
}

/**
 * Validate if the given address is a Stellar contract address.
 *
 * A valid Stellar contract address can be:
 * - A 56-character Base32-encoded string starting with 'C'.
 *
 * @param {string} address - The input Stellar contract address.
 * @returns {boolean} - True if the address is a valid Stellar contract, otherwise false.
 */
function isValidStellarContract(address) {
    return StellarSdk.StrKey.isValidContract(address);
}

/**
 * Basic validatation to check if the provided string *might* be a valid SVM
 * address. One needs to ensure that it's 32 bytes long after decoding.
 *
 * See https://solana.com/developers/guides/advanced/exchange#basic-verification.
 *
 * @param {string} address - The base58 encoded Solana address to validate
 * @returns {boolean} - True if the address is valid, false otherwise
 */
function isValidSvmAddressFormat(address) {
    return SVM_BASE58_ADDRESS_REGEX.test(address);
}

const validationFunctions = {
    isNonEmptyString,
    isNumber,
    isValidNumber,
    isValidDecimal,
    isNumberArray,
    isKeccak256Hash,
    isString,
    isNonEmptyStringArray,
    isValidTimeFormat,
    isValidStellarAddress,
    isValidStellarAccount,
    isValidStellarContract,
    isValidSvmAddressFormat,
};

function validateParameters(parameters) {
    for (const [validatorFunctionString, paramsObj] of Object.entries(parameters)) {
        const validatorFunction = validationFunctions[validatorFunctionString];

        if (typeof validatorFunction !== 'function') {
            throw new Error(`Validator function ${validatorFunction} is not defined`);
        }

        for (const paramKey of Object.keys(paramsObj)) {
            const paramValue = paramsObj[paramKey];
            const isValid = validatorFunction(paramValue);

            if (!isValid) {
                throw new Error(`Input validation failed for ${validatorFunctionString} with parameter ${paramKey}: ${paramValue}`);
            }
        }
    }
}

const dateToEta = (utcTimeString) => {
    if (utcTimeString === '0') {
        return 0;
    }

    const date = new Date(utcTimeString + 'Z');

    if (isNaN(date.getTime())) {
        throw new Error(`Invalid date format provided: ${utcTimeString}`);
    }

    return Math.floor(date.getTime() / 1000);
};

const etaToDate = (timestamp) => {
    const date = new Date(timestamp * 1000);

    if (isNaN(date.getTime())) {
        throw new Error(`Invalid timestamp provided: ${timestamp}`);
    }

    return date.toISOString().slice(0, 19);
};

const getCurrentTimeInSeconds = () => {
    const now = new Date();
    const currentTimeInSecs = Math.floor(now.getTime() / 1000);
    return currentTimeInSecs;
};

/**
 * Prompt the user for confirmation
 * @param {string} question Prompt question
 * @param {boolean} yes If true, skip the prompt
 * @returns {boolean} Returns true if the prompt was skipped, false otherwise
 */
const prompt = (question, yes = false) => {
    // skip the prompt if yes was passed
    if (yes) {
        return false;
    }

    const answer = readlineSync.question(`${question} ${chalk.green('(y/n)')} `);
    console.log();

    return answer !== 'y';
};

function findProjectRoot(startDir) {
    let currentDir = startDir;

    while (currentDir !== path.parse(currentDir).root) {
        const potentialPackageJson = path.join(currentDir, 'package.json');

        if (existsSync(potentialPackageJson)) {
            return currentDir;
        }

        currentDir = path.resolve(currentDir, '..');
    }

    throw new Error('Unable to find project root');
}

function toBigNumberString(number) {
    return Math.ceil(number).toLocaleString('en', { useGrouping: false });
}

const isValidCosmosAddress = (str) => {
    try {
        normalizeBech32(str);

        return true;
    } catch (error) {
        return false;
    }
};

const getSaltFromKey = (key) => {
    return keccak256(defaultAbiCoder.encode(['string'], [key.toString()]));
};

const getAmplifierContractOnchainConfig = async (axelar, chain, contract = 'MultisigProver') => {
    const key = Buffer.from('config');
    const client = await CosmWasmClient.connect(axelar.rpc);
    const value = await client.queryContractRaw(axelar.contracts[contract][chain].address, key);
    return JSON.parse(Buffer.from(value).toString('ascii'));
};

async function getDomainSeparator(axelar, chain, options, contract = 'MultisigProver') {
    // Allow any domain separator for local deployments or `0x` if not provided
    if (options.env === 'local') {
        if (options.domainSeparator && options.domainSeparator !== 'offline') {
            return options.domainSeparator;
        }

        return ethers.constants.HashZero;
    }

    if (isKeccak256Hash(options.domainSeparator)) {
        // return the domainSeparator for debug deployments
        return options.domainSeparator;
    }

    const { contracts, chainId } = axelar;
    const {
        Router: { address: routerAddress },
    } = contracts;

    if (!isString(chain.axelarId)) {
        throw new Error(`missing or invalid axelar ID for chain ${chain.name}`);
    }

    if (!isString(routerAddress) || !isValidCosmosAddress(routerAddress)) {
        throw new Error(`missing or invalid router address`);
    }

    if (!isString(chainId)) {
        throw new Error(`missing or invalid chain ID`);
    }

    const expectedDomainSeparator = calculateDomainSeparator(chain.axelarId, routerAddress, chainId);

    if (options.domainSeparator === 'offline') {
        printInfo('Computed domain separator offline');
        return expectedDomainSeparator;
    }

    printInfo(`Retrieving domain separator for ${chain.name} from Axelar network`);
    const domainSeparator = hexlify((await getAmplifierContractOnchainConfig(axelar, chain.axelarId, contract)).domain_separator);

    if (domainSeparator !== expectedDomainSeparator) {
        throw new Error(`unexpected domain separator (want ${expectedDomainSeparator}, got ${domainSeparator})`);
    }

    return expectedDomainSeparator;
}

const getChainConfig = (chains, chainName, options = {}) => {
    if (!chainName) {
        return undefined;
    }

    const chainConfig = chains[chainName];

    if (!options.skipCheck && !chainConfig) {
        throw new Error(`Chain ${chainName} not found in config`);
    }

    return chainConfig;
};

const getChainConfigByAxelarId = (config, chainAxelarId) => {
    if (chainAxelarId === 'axelar') {
        return config.axelar;
    }

    for (const chain of Object.values(config.chains)) {
        if (chain.axelarId === chainAxelarId) {
            return chain;
        }
    }

    throw new Error(`Chain with axelarId ${chainAxelarId} not found in config`);
};

const getMultisigProof = async (axelar, chain, multisigSessionId) => {
    const query = { proof: { multisig_session_id: `${multisigSessionId}` } };
    const client = await CosmWasmClient.connect(axelar.rpc);
    const value = await client.queryContractSmart(axelar.contracts.MultisigProver[chain].address, query);
    return value;
};

const getCurrentVerifierSet = async (axelar, chain, contract = 'MultisigProver') => {
    const client = await CosmWasmClient.connect(axelar.rpc);
    const { id: verifierSetId, verifier_set: verifierSet } = await client.queryContractSmart(
        axelar.contracts[contract][chain].address,
        'current_verifier_set',
    );

    return {
        verifierSetId,
        verifierSet,
        signers: Object.values(verifierSet.signers),
    };
};

const calculateDomainSeparator = (chain, router, network) => keccak256(Buffer.from(`${chain}${router}${network}`));

const downloadContractCode = async (url, contractName, version) => {
    const tempDir = path.join(process.cwd(), 'artifacts');

    if (!existsSync(tempDir)) {
        mkdirSync(tempDir, { recursive: true });
    }

    const outputPath = path.join(tempDir, `${contractName}-${version}.wasm`);

    const response = await fetch(url);

    if (!response.ok) {
        throw new Error(`Failed to download WASM file: ${response.statusText}`);
    }

    const buffer = await response.buffer();
    writeFileSync(outputPath, buffer);

    return outputPath;
};

const tryItsEdgeContract = (chainConfig) => {
    const itsEdgeContract =
        chainConfig.contracts.InterchainTokenService?.objects?.ChannelId || // sui
        chainConfig.contracts.InterchainTokenService?.address;

    return itsEdgeContract;
};

const itsEdgeContract = (chainConfig) => {
    const itsEdgeContract = tryItsEdgeContract(chainConfig);

    if (!itsEdgeContract) {
        throw new Error(`Missing InterchainTokenService edge contract for chain: ${chainConfig.name}`);
    }

    return itsEdgeContract;
};

const itsEdgeChains = (chains) =>
    Object.values(chains)
        .filter(tryItsEdgeContract)
        .map((chain) => chain.axelarId);

const parseTrustedChains = (chains, trustedChains) => {
    return trustedChains.length === 1 && trustedChains[0] === 'all' ? itsEdgeChains(chains) : trustedChains;
};

const readContractCode = (options) => {
    return readFileSync(options.contractCodePath);
};

function asciiToBytes(string) {
    return hexlify(Buffer.from(string, 'ascii'));
}

function solanaAddressBytesFromBase58(string) {
    const decoded = bs58.default.decode(string);
    if (decoded.length !== 32) {
        throw new Error(`Invalid Solana address: ${string}`);
    }
    return hexlify(decoded);
}

/**
 * Encodes the destination address for Interchain Token Service (ITS) transfers.
 * This function ensures proper encoding of the destination address based on the destination chain type.
 * Note: - Stellar and XRPL addresses are converted to ASCII byte arrays.
 *       - Solana (svm) addresses are decoded from base58 and hexlified.
 *       - EVM and Sui addresses are returned as-is (default behavior).
 *       - Additional encoding logic can be added for new chain types.
 */
function encodeITSDestination(chains, destinationChain, destinationAddress) {
    const chainType = getChainConfig(chains, destinationChain, { skipCheck: true })?.chainType;

    switch (chainType) {
        case undefined:
            printWarn(`destinationChain ${destinationChain} not found in config`);
            return destinationAddress;

        case 'stellar':
            validateParameters({ isValidStellarAddress: { destinationAddress } });
            return asciiToBytes(destinationAddress);

        case 'svm':
            validateParameters({ isValidSvmAddressFormat: { destinationAddress } });
            return solanaAddressBytesFromBase58(destinationAddress);

        case 'xrpl':
            // TODO: validate XRPL address format
            return asciiToBytes(destinationAddress);

        case 'stacks':
            return cvToHex(principalCV(destinationAddress));

        case 'evm':
        case 'sui':
        default: // EVM, Sui, and other chains (return as-is)
            return destinationAddress;
    }
}

const getProposalConfig = (config, env, key) => {
    try {
        const value = config.axelar?.[key];
        if (value === undefined) throw new Error(`Key "${key}" not found in config for ${env}`);
        return value;
    } catch (error) {
        throw new Error(`Failed to load config value "${key}" for ${env}: ${error.message}`);
    }
};

module.exports = {
    loadConfig,
    saveConfig,
    writeJSON,
    printInfo,
    printWarn,
    printError,
    printHighlight,
    printDivider,
    printLog,
    isKeccak256Hash,
    isNonEmptyString,
    isString,
    isStringArray,
    isStringLowercase,
    isNumber,
    isValidNumber,
    isValidDecimal,
    isNumberArray,
    isNonEmptyStringArray,
    isValidTimeFormat,
    copyObject,
    httpGet,
    httpPost,
    callAxelarscanApi,
    parseArgs,
    sleep,
    dateToEta,
    etaToDate,
    getCurrentTimeInSeconds,
    prompt,
    findProjectRoot,
    toBigNumberString,
    timeout,
    validateParameters,
    getDomainSeparator,
    getChainConfig,
    getChainConfigByAxelarId,
    getMultisigProof,
    getAmplifierContractOnchainConfig,
    getSaltFromKey,
    calculateDomainSeparator,
    downloadContractCode,
    pascalToKebab,
    pascalToSnake,
    kebabToPascal,
    readContractCode,
    VERSION_REGEX,
    SHORT_COMMIT_HASH_REGEX,
    itsEdgeContract,
    tryItsEdgeContract,
    parseTrustedChains,
    isValidStellarAddress,
    isValidStellarAccount,
    isValidStellarContract,
    isValidSvmAddressFormat,
    getCurrentVerifierSet,
    asciiToBytes,
    encodeITSDestination,
    getProposalConfig,
    itsHubContractAddress,
    asyncLocalLoggerStorage,
    printMsg,
};
