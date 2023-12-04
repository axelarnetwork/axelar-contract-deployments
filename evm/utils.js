'use strict';

const { ethers } = require('hardhat');
const {
    ContractFactory,
    Contract,
    utils: { computeAddress, getContractAddress, keccak256, isAddress, getCreate2Address, defaultAbiCoder, isHexString },
    constants: { AddressZero },
    getDefaultProvider,
} = ethers;
const https = require('https');
const http = require('http');
const fs = require('fs');
const path = require('path');
const { outputJsonSync } = require('fs-extra');
const zkevm = require('@0xpolygonhermez/zkevm-commonjs');
const readlineSync = require('readline-sync');
const chalk = require('chalk');
const {
    create3DeployContract,
    deployContractConstant,
    predictContractConstant,
    getCreate3Address,
    printObj,
} = require('@axelar-network/axelar-gmp-sdk-solidity');
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const CreateDeploy = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/deploy/CreateDeploy.sol/CreateDeploy.json');
const IDeployer = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IDeployer.json');
const { verifyContract } = require(`${__dirname}/../axelar-chains-config`);

const getSaltFromKey = (key) => {
    return keccak256(defaultAbiCoder.encode(['string'], [key.toString()]));
};

const deployCreate = async (wallet, contractJson, args = [], options = {}, verifyOptions = null, chain = {}) => {
    const factory = new ContractFactory(contractJson.abi, contractJson.bytecode, wallet);

    const contract = await factory.deploy(...args, options);
    await contract.deployTransaction.wait(chain.confirmations);

    if (verifyOptions?.env) {
        sleep(10000);

        try {
            await verifyContract(verifyOptions.env, verifyOptions.chain, contract.address, args, verifyOptions);
        } catch (e) {
            console.log('FAILED VERIFICATION!!');
        }
    }

    return contract;
};

const deployCreate2 = async (
    constAddressDeployerAddress,
    wallet,
    contractJson,
    args = [],
    salt = Date.now(),
    gasOptions = null,
    verifyOptions = null,
    chain = {},
) => {
    let contract;

    if (!verifyOptions?.only) {
        contract = await deployContractConstant(
            constAddressDeployerAddress,
            wallet,
            contractJson,
            salt,
            args,
            gasOptions,
            chain.confirmations,
        );
    } else {
        contract = { address: await predictContractConstant(constAddressDeployerAddress, wallet, contractJson, salt, args) };
    }

    if (verifyOptions?.env) {
        sleep(2000);

        try {
            await verifyContract(verifyOptions.env, verifyOptions.chain, contract.address, args, verifyOptions);
        } catch (e) {
            console.log(`FAILED VERIFICATION!! ${e}`);
        }
    }

    return contract;
};

const deployCreate3 = async (
    create3DeployerAddress,
    wallet,
    contractJson,
    args = [],
    key = Date.now(),
    gasOptions = null,
    verifyOptions = null,
    chain = {},
) => {
    let contract;

    if (!verifyOptions?.only) {
        contract = await create3DeployContract(create3DeployerAddress, wallet, contractJson, key, args, gasOptions, chain.confirmations);
    } else {
        contract = { address: await getCreate3Address(create3DeployerAddress, wallet, key) };
    }

    if (verifyOptions?.env) {
        sleep(2000);

        try {
            await verifyContract(verifyOptions.env, verifyOptions.chain, contract.address, args, verifyOptions);
        } catch (e) {
            console.log(`FAILED VERIFICATION!! ${e}`);
        }
    }

    return contract;
};

const printInfo = (msg, info = '', colour = chalk.green) => {
    if (info) {
        console.log(`${msg}: ${colour(info)}\n`);
    } else {
        console.log(`${msg}\n`);
    }
};

const printWarn = (msg, info = '') => {
    if (info) {
        msg = `${msg}: ${info}`;
    }

    console.log(`${chalk.italic.yellow(msg)}\n`);
};

const printError = (msg, info = '') => {
    if (info) {
        msg = `${msg}: ${info}`;
    }

    console.log(`${chalk.bold.red(msg)}\n`);
};

function printLog(log) {
    console.log(JSON.stringify({ log }, null, 2));
}

const writeJSON = (data, name) => {
    outputJsonSync(name, data, {
        spaces: 2,
        EOL: '\n',
    });
};

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

const isNonEmptyString = (arg) => {
    return typeof arg === 'string' && arg !== '';
};

const isString = (arg) => {
    return typeof arg === 'string';
};

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

const isAddressArray = (arr) => {
    if (!Array.isArray(arr)) return false;

    for (const item of arr) {
        if (!isAddress(item)) {
            return false;
        }
    }

    return true;
};

const getCurrentTimeInSeconds = () => {
    const now = new Date();
    const currentTimeInSecs = Math.floor(now.getTime() / 1000);
    return currentTimeInSecs;
};

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
 * Determines if a given input is a valid calldata for Solidity.
 *
 * @param {string} input - The string to validate.
 * @returns {boolean} - Returns true if the input is a valid calldata, false otherwise.
 */
function isValidCalldata(input) {
    if (input === '0x') {
        return true;
    }

    // Ensure it's a string, starts with '0x' and has an even number of characters after '0x'
    if (typeof input !== 'string' || input.slice(0, 2) !== '0x' || input.length % 2 !== 0) {
        return false;
    }

    // Ensure all characters after the '0x' prefix are hexadecimal (0-9, a-f, A-F)
    const hexPattern = /^[a-fA-F0-9]+$/;

    return hexPattern.test(input.slice(2));
}

function isValidBytesAddress(input) {
    const addressRegex = /^0x[a-fA-F0-9]{40}$/;
    return addressRegex.test(input);
}

const isContract = async (address, provider) => {
    const code = await provider.getCode(address);
    return code && code !== '0x';
};

function isValidAddress(address, allowZeroAddress) {
    if (!allowZeroAddress && address === AddressZero) {
        return false;
    }

    return isAddress(address);
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

// Validate if the input privateKey is correct
function isValidPrivateKey(privateKey) {
    // Check if it's a valid hexadecimal string
    if (!privateKey?.startsWith('0x')) {
        privateKey = '0x' + privateKey;
    }

    if (!isHexString(privateKey) || privateKey.length !== 66) {
        return false;
    }

    return true;
}

function isValidTokenId(input) {
    if (!input?.startsWith('0x')) {
        return false;
    }

    const hexPattern = /^[0-9a-fA-F]+$/;

    if (!hexPattern.test(input.slice(2))) {
        return false;
    }

    const minValue = BigInt('0x00');
    const maxValue = BigInt('0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF');
    const numericValue = BigInt(input);

    return numericValue >= minValue && numericValue <= maxValue;
}

const validationFunctions = {
    isNonEmptyString,
    isNumber,
    isValidNumber,
    isValidDecimal,
    isNumberArray,
    isString,
    isNonEmptyStringArray,
    isAddressArray,
    isKeccak256Hash,
    isValidCalldata,
    isValidBytesAddress,
    isValidTimeFormat,
    isContract,
    isValidAddress,
    isValidPrivateKey,
    isValidTokenId,
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

/**
 * Parses the input string into an array of arguments, recognizing and converting
 * to the following types: boolean, number, array, and string.
 *
 * @param {string} input - The string of arguments to parse.
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

/**
 * Compute bytecode hash for a deployed contract or contract factory as it would appear on-chain.
 * Some chains don't use keccak256 for their state representation, which is taken into account by this function.
 * @param {Object} contractObject - An instance of the contract or a contract factory (ethers.js Contract or ContractFactory object)
 * @returns {Promise<string>} - The keccak256 hash of the contract bytecode
 */
async function getBytecodeHash(contractObject, chain = '', provider = null) {
    let bytecode;

    if (isNonEmptyString(contractObject)) {
        if (provider === null) {
            throw new Error('Provider must be provided for chain');
        }

        bytecode = await provider.getCode(contractObject);
    } else if (contractObject.address) {
        // Contract instance
        provider = contractObject.provider;
        bytecode = await provider.getCode(contractObject.address);
    } else if (contractObject.deployedBytecode) {
        // Contract factory
        bytecode = contractObject.deployedBytecode;
    } else {
        throw new Error('Invalid contract object. Expected ethers.js Contract or ContractFactory.');
    }

    if (bytecode === '0x') {
        throw new Error('Contract bytecode is empty');
    }

    if (chain.toLowerCase() === 'polygon-zkevm') {
        const codehash = zkevm.smtUtils.hashContractBytecode(bytecode);
        return codehash;
    }

    return keccak256(bytecode);
}

const predictAddressCreate = async (from, nonce) => {
    const address = getContractAddress({
        from,
        nonce,
    });

    return address;
};

/**
 * Get the predicted address of a contract deployment using one of create/create2/create3 deployment method.
 * @param {string} deployer - Sender address that's triggering the contract deployment
 * @param {string} deployMethod - 'create', 'create2', 'create3'
 * @param {Object} options - Options for the deployment
 * @param {string} options.deployerContract - Address of the contract that will deploy the contract
 * @param {string} options.contractJson - Compiled contract to be deployed
 * @param {any[]} options.constructorArgs - Arguments for the contract constructor
 * @param {string} options.salt - Salt for the deployment
 * @param {number} options.nonce - Nonce for the deployment
 * @param {boolean} options.offline - Whether to compute address offline or use an online provider to get the nonce/deployed address
 * @param {Object} options.provider - Provider to use for online deployment
 * @returns {Promise<string>} - The predicted contract address
 */
const getDeployedAddress = async (deployer, deployMethod, options = {}) => {
    switch (deployMethod) {
        case 'create': {
            let nonce = options.nonce;

            if (!nonce && !options.offline) {
                nonce = await options.provider.getTransactionCount(deployer);
            } else if (!isNumber(nonce)) {
                throw new Error('Nonce must be provided for create deployment');
            }

            return getContractAddress({
                from: deployer,
                nonce,
            });
        }

        case 'create2': {
            let salt = getSaltFromKey(options.salt);

            const deployerContract = options.deployerContract;

            if (!isNonEmptyString(deployerContract)) {
                throw new Error('Deployer contract address was not provided');
            }

            const contractJson = options.contractJson;
            const constructorArgs = options.constructorArgs;
            const factory = new ContractFactory(contractJson.abi, contractJson.bytecode);
            const initCode = factory.getDeployTransaction(...constructorArgs).data;

            if (!options.offline) {
                const deployerInterface = new Contract(deployerContract, IDeployer.abi, options.provider);

                return await deployerInterface.deployedAddress(initCode, deployer, salt);
            }

            salt = keccak256(defaultAbiCoder.encode(['address', 'bytes32'], [deployer, salt]));

            return getCreate2Address(deployerContract, salt, keccak256(initCode));
        }

        case 'create3': {
            const deployerContract = options.deployerContract;

            if (!isNonEmptyString(deployerContract)) {
                throw new Error('Deployer contract address was not provided');
            }

            if (!options.offline) {
                const salt = getSaltFromKey(options.salt);

                const deployerInterface = new Contract(deployerContract, IDeployer.abi, options.provider);

                return await deployerInterface.deployedAddress('0x', deployer, salt);
            }

            const createDeployer = await getDeployedAddress(deployer, 'create2', {
                salt: options.salt,
                deployerContract,
                contractJson: CreateDeploy,
                constructorArgs: [],
            });

            const contractAddress = getContractAddress({
                from: createDeployer,
                nonce: 1,
            });

            return contractAddress;
        }

        default: {
            throw new Error(`Invalid deployment method: ${deployMethod}`);
        }
    }
};

const getProxy = async (config, chain) => {
    const address = (await httpGet(`${config.axelar.lcd}/axelar/evm/v1beta1/gateway_address/${chain}`)).address;
    return address;
};

const getEVMBatch = async (config, chain, batchID = '') => {
    const batch = await httpGet(`${config.axelar.lcd}/axelar/evm/v1beta1/batched_commands/${chain}/${batchID}`);
    return batch;
};

const getEVMAddresses = async (config, chain, options = {}) => {
    const keyID = options.keyID || '';

    if (isAddress(keyID)) {
        return { addresses: [keyID], weights: [Number(1)], threshold: 1, keyID: 'debug' };
    }

    const evmAddresses = options.amplifier
        ? await getAmplifierKeyAddresses(config, chain)
        : await httpGet(`${config.axelar.lcd}/axelar/evm/v1beta1/key_address/${chain}?key_id=${keyID}`);

    const sortedAddresses = evmAddresses.addresses.sort((a, b) => a.address.toLowerCase().localeCompare(b.address.toLowerCase()));

    const addresses = sortedAddresses.map((weightedAddress) => weightedAddress.address);
    const weights = sortedAddresses.map((weightedAddress) => Number(weightedAddress.weight));
    const threshold = Number(evmAddresses.threshold);

    return { addresses, weights, threshold, keyID: evmAddresses.key_id };
};

const getAmplifierKeyAddresses = async (config, chain) => {
    const client = await CosmWasmClient.connect(config.axelar.rpc);
    const workerSet = await client.queryContractSmart(config.axelar.contracts.MultisigProver[chain].address, 'get_worker_set');

    const weightedAddresses = workerSet.signers.map((signer) => ({
        address: computeAddress(`0x${signer.pub_key.ecdsa}`),
        weight: signer.weight,
    }));

    return { addresses: weightedAddresses, threshold: workerSet.threshold };
};

function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

function loadConfig(env) {
    return require(`${__dirname}/../axelar-chains-config/info/${env}.json`);
}

function saveConfig(config, env) {
    writeJSON(config, `${__dirname}/../axelar-chains-config/info/${env}.json`);
}

async function printWalletInfo(wallet, options = {}) {
    let balance = 0;
    const address = await wallet.getAddress();
    printInfo('Wallet address', address);

    if (!options.offline) {
        balance = await wallet.provider.getBalance(address);

        if (balance.isZero()) {
            printError('Wallet balance', '0');
        } else {
            printInfo('Wallet balance', `${balance / 1e18}`);
        }

        printInfo('Wallet nonce', (await wallet.provider.getTransactionCount(address)).toString());
    }

    return { address, balance };
}

const deployContract = async (
    deployMethod,
    wallet,
    contractJson,
    constructorArgs,
    deployOptions = {},
    gasOptions = {},
    verifyOptions = {},
    chain = {},
) => {
    const predictedAddress = await getDeployedAddress(wallet.address, deployMethod, {
        salt: deployOptions.salt,
        deployerContract: deployOptions.deployerContract,
        contractJson,
        constructorArgs,
        provider: wallet.provider,
    });

    if (await isContract(predictedAddress, wallet.provider)) {
        printError(`Contract is already deployed at ${predictedAddress}, skipping`);
        return new Contract(predictedAddress, contractJson.abi, wallet);
    }

    switch (deployMethod) {
        case 'create': {
            const contract = await deployCreate(wallet, contractJson, constructorArgs, gasOptions, verifyOptions, chain);
            return contract;
        }

        case 'create2': {
            if (!isNonEmptyString(deployOptions.deployerContract)) {
                throw new Error('Deployer contract address was not provided');
            }

            if (!isNonEmptyString(deployOptions.salt)) {
                throw new Error('Salt was not provided');
            }

            const contract = await deployCreate2(
                deployOptions.deployerContract,
                wallet,
                contractJson,
                constructorArgs,
                deployOptions.salt,
                gasOptions,
                verifyOptions,
                chain,
            );

            return contract;
        }

        case 'create3': {
            if (!isNonEmptyString(deployOptions.deployerContract)) {
                throw new Error('Deployer contract address was not provided');
            }

            if (!isNonEmptyString(deployOptions.salt)) {
                throw new Error('Salt was not provided');
            }

            const contract = await deployCreate3(
                deployOptions.deployerContract,
                wallet,
                contractJson,
                constructorArgs,
                deployOptions.salt,
                gasOptions,
                verifyOptions,
                chain,
            );

            return contract;
        }

        default: {
            throw new Error(`Invalid deployment method: ${deployMethod}`);
        }
    }
};

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

/**
 * Check if a specific event was emitted in a transaction receipt.
 *
 * @param {object} receipt - The transaction receipt object.
 * @param {object} contract - The ethers.js contract instance.
 * @param {string} eventName - The name of the event.
 * @return {boolean} - Returns true if the event was emitted, false otherwise.
 */
function wasEventEmitted(receipt, contract, eventName) {
    const event = contract.filters[eventName]();

    return receipt.logs.some((log) => log.topics[0] === event.topics[0]);
}

function copyObject(obj) {
    return JSON.parse(JSON.stringify(obj));
}

const mainProcessor = async (options, processCommand, save = true, catchErr = false) => {
    if (!options.env) {
        throw new Error('Environment was not provided');
    }

    if (!options.chainName && !options.chainNames) {
        throw new Error('Chain names were not provided');
    }

    printInfo('Environment', options.env);

    const config = loadConfig(options.env);
    let chains = options.chainName ? [options.chainName] : options.chainNames.split(',').map((str) => str.trim());
    const chainsToSkip = (options.skipChains || '').split(',').map((str) => str.trim());

    if (options.chainNames === 'all') {
        chains = Object.keys(config.chains);
    }

    for (const chainName of chains) {
        if (config.chains[chainName.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chainName} is not defined in the info file`);
        }
    }

    for (const chainName of chains) {
        const chain = config.chains[chainName.toLowerCase()];

        if (chainsToSkip.includes(chain.name.toLowerCase()) || chain.status === 'deactive' || chain.contracts[options.contractName]?.skip) {
            printWarn('Skipping chain', chain.name);
            continue;
        }

        printInfo('Chain', chain.name, chalk.cyan);

        try {
            await processCommand(config, chain, options);
        } catch (error) {
            printError(`Failed with error on ${chain.name}`, error.message);

            if (!catchErr && !options.ignoreError) {
                throw error;
            }
        }

        if (save) {
            saveConfig(config, options.env);
        }
    }
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

function getConfigByChainId(chainId, config) {
    for (const chain of Object.values(config.chains)) {
        if (chain.chainId === chainId) {
            return chain;
        }
    }

    throw new Error(`Chain with chainId ${chainId} not found in the config`);
}

function findProjectRoot(startDir) {
    let currentDir = startDir;

    while (currentDir !== path.parse(currentDir).root) {
        const potentialPackageJson = path.join(currentDir, 'package.json');

        if (fs.existsSync(potentialPackageJson)) {
            return currentDir;
        }

        currentDir = path.resolve(currentDir, '..');
    }

    throw new Error('Unable to find project root');
}

function findContractPath(dir, contractName) {
    const files = fs.readdirSync(dir);

    for (const file of files) {
        const filePath = path.join(dir, file);
        const stat = fs.statSync(filePath);

        if (stat && stat.isDirectory()) {
            const recursivePath = findContractPath(filePath, contractName);

            if (recursivePath) {
                return recursivePath;
            }
        } else if (file === `${contractName}.json`) {
            return filePath;
        }
    }
}

function getContractPath(contractName, projectRoot = '') {
    if (projectRoot === '') {
        projectRoot = path.join(findProjectRoot(__dirname), 'node_modules', '@axelar-network');
    }

    projectRoot = path.resolve(projectRoot);

    const searchDirs = [
        path.join(projectRoot, 'axelar-gmp-sdk-solidity', 'artifacts', 'contracts'),
        path.join(projectRoot, 'axelar-cgp-solidity', 'artifacts', 'contracts'),
        path.join(projectRoot, 'interchain-token-service', 'artifacts', 'contracts'),
    ];

    for (const dir of searchDirs) {
        if (fs.existsSync(dir)) {
            const contractPath = findContractPath(dir, contractName);

            if (contractPath) {
                return contractPath;
            }
        }
    }

    throw new Error(`Contract path for ${contractName} must be entered manually.`);
}

function getContractJSON(contractName, artifactPath) {
    let contractPath;

    if (artifactPath) {
        contractPath = artifactPath.endsWith('.json') ? artifactPath : artifactPath + contractName + '.sol/' + contractName + '.json';
    } else {
        contractPath = getContractPath(contractName);
    }

    try {
        const contractJson = require(contractPath);
        return contractJson;
    } catch (err) {
        throw new Error(`Failed to load contract JSON for ${contractName} at path ${contractPath} with error: ${err}`);
    }
}

/**
 * Retrieves gas options for contract interactions.
 *
 * This function determines the appropriate gas options for a given transaction.
 * It supports offline scenarios and applies gas price adjustments if specified.
 *
 * @param {Object} chain - The chain config object.
 * @param {Object} options - Script options, including the 'offline' flag.
 * @param {String} contractName - The name of the contract to deploy/interact with.
 * @param {Object} defaultGasOptions - Optional default gas options if none are provided in the chain or contract configs.
 *
 * @returns {Object} An object containing gas options for the transaction.
 *
 * @throws {Error} Throws an error if fetching the gas price fails.
 *
 * Note:
 * - If 'options.offline' is true, static gas options from the contract or chain config are used.
 * - If 'gasPriceAdjustment' is set in gas options and 'gasPrice' is not pre-defined, the gas price
 *   is fetched from the provider and adjusted according to 'gasPriceAdjustment'.
 */
async function getGasOptions(chain, options, contractName, defaultGasOptions = {}) {
    const { offline } = options;

    const contractConfig = contractName ? chain?.contracts[contractName] : null;

    if (offline) {
        return copyObject(contractConfig?.staticGasOptions || chain?.staticGasOptions || defaultGasOptions);
    }

    const gasOptions = copyObject(contractConfig?.gasOptions || chain?.gasOptions || defaultGasOptions);
    const gasPriceAdjustment = gasOptions.gasPriceAdjustment;

    if (gasPriceAdjustment && !gasOptions.gasPrice) {
        try {
            const provider = getDefaultProvider(chain.rpc);
            gasOptions.gasPrice = Math.floor((await provider.getGasPrice()) * gasPriceAdjustment);
        } catch (err) {
            throw new Error(`Provider failed to retrieve gas price on chain ${chain.name}: ${err}`);
        }
    }

    if (gasPriceAdjustment) {
        delete gasOptions.gasPriceAdjustment;
    }

    printInfo('Gas options', JSON.stringify(gasOptions, null, 2));

    return gasOptions;
}

module.exports = {
    deployCreate,
    deployCreate2,
    deployCreate3,
    deployContract,
    writeJSON,
    copyObject,
    httpGet,
    printObj,
    printLog,
    printInfo,
    printWarn,
    printError,
    getBytecodeHash,
    predictAddressCreate,
    getDeployedAddress,
    isString,
    isNonEmptyString,
    isNumber,
    isValidNumber,
    isValidDecimal,
    isNumberArray,
    isNonEmptyStringArray,
    isAddressArray,
    isKeccak256Hash,
    isValidCalldata,
    isValidBytesAddress,
    validateParameters,
    parseArgs,
    getProxy,
    getEVMBatch,
    getEVMAddresses,
    getConfigByChainId,
    sleep,
    loadConfig,
    saveConfig,
    printWalletInfo,
    isValidTimeFormat,
    dateToEta,
    etaToDate,
    getCurrentTimeInSeconds,
    wasEventEmitted,
    isContract,
    isValidAddress,
    isValidPrivateKey,
    isValidTokenId,
    verifyContract,
    prompt,
    mainProcessor,
    getContractPath,
    getContractJSON,
    getGasOptions,
};
