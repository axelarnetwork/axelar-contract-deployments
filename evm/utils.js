'use strict';

const {
    ContractFactory,
    Contract,
    utils: { computeAddress, getContractAddress, keccak256, isAddress, getCreate2Address, defaultAbiCoder },
} = require('ethers');
const https = require('https');
const http = require('http');
const { outputJsonSync, readJsonSync } = require('fs-extra');
const { exec } = require('child_process');
const { writeFile } = require('fs');
const { promisify } = require('util');
const zkevm = require('@0xpolygonhermez/zkevm-commonjs');
const chalk = require('chalk');
const {
    create3DeployContract,
    deployContractConstant,
    predictContractConstant,
    getCreate3Address,
} = require('@axelar-network/axelar-gmp-sdk-solidity');
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');
const CreateDeploy = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/deploy/CreateDeploy.sol/CreateDeploy.json');
const IDeployer = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IDeployer.json');

const execAsync = promisify(exec);
const writeFileAsync = promisify(writeFile);

const getSaltFromKey = (key) => {
    return keccak256(defaultAbiCoder.encode(['string'], [key.toString()]));
};

const deployCreate = async (wallet, contractJson, args = [], options = {}, verifyOptions = null) => {
    const factory = new ContractFactory(contractJson.abi, contractJson.bytecode, wallet);

    const contract = await factory.deploy(...args, { ...options });
    await contract.deployed();

    if (verifyOptions?.env) {
        sleep(10000);

        try {
            await verifyContract(verifyOptions.env, verifyOptions.chain, contract.address, args, verifyOptions.contractPath);
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
) => {
    let contract;

    if (!verifyOptions?.only) {
        contract = await deployContractConstant(constAddressDeployerAddress, wallet, contractJson, salt, args, gasOptions?.gasLimit);
    } else {
        contract = { address: await predictContractConstant(constAddressDeployerAddress, wallet, contractJson, salt, args) };
    }

    if (verifyOptions?.env) {
        sleep(2000);

        try {
            await verifyContract(verifyOptions.env, verifyOptions.chain, contract.address, args, verifyOptions.contractPath);
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
) => {
    let contract;

    if (!verifyOptions?.only) {
        contract = await create3DeployContract(create3DeployerAddress, wallet, contractJson, key, args, gasOptions.gasLimit);
    } else {
        contract = { address: await getCreate3Address(create3DeployerAddress, wallet, key) };
    }

    if (verifyOptions?.env) {
        sleep(2000);

        try {
            await verifyContract(verifyOptions.env, verifyOptions.chain, contract.address, args, verifyOptions.contractPath);
        } catch (e) {
            console.log(`FAILED VERIFICATION!! ${e}`);
        }
    }

    return contract;
};

const printObj = (obj) => {
    console.log(JSON.stringify(obj, null, 2));
};

const printInfo = (msg, info = '') => {
    if (info) {
        console.log(`${msg}: ${chalk.green(info)}\n`);
    } else {
        console.log(`${msg}\n`);
    }
};

const printWarn = (msg, info = '') => {
    if (info) {
        msg = msg + ': ' + info;
    }

    console.log(`${chalk.yellow(msg)}\n`);
};

const printError = (msg, info = '') => {
    if (info) {
        msg = msg + ': ' + info;
    }

    console.log(`${chalk.red(msg)}\n`);
};

function printLog(log) {
    console.log(JSON.stringify({ log }, null, 2));
}

const readJSON = (filePath, require = false) => {
    let data;

    try {
        data = readJsonSync(filePath, 'utf8');
    } catch (err) {
        if (err.code === 'ENOENT' && !require) {
            return undefined;
        }

        throw err;
    }

    return data;
};

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

            if (statusCode !== 200) {
                error = new Error('Request Failed.\n' + `Status Code: ${statusCode}`);
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

/**
 * Imports custom networks into hardhat config format.
 * Check out the example hardhat config for usage `.example.hardhat.config.js`.
 *
 * @param {Object[]} chains - Array of chain objects following the format in info/mainnet.json
 * @param {Object} keys - Object containing keys for contract verification and accounts
 * @returns {Object} - Object containing networks and etherscan config
 */
const importNetworks = (chains, keys) => {
    const networks = {
        hardhat: {
            chainId: 31337, // default hardhat network chain id
            id: 'hardhat',
            confirmations: 1,
        },
    };

    const etherscan = {
        apiKey: {},
        customChains: [],
    };

    if (!chains.chains) {
        // Use new format
        delete chains.chains;
        chains = {
            chains,
        };
    }

    // Add custom networks
    Object.entries(chains.chains).forEach(([chainName, chain]) => {
        const name = chainName.toLowerCase();
        networks[name] = {
            chainId: chain.chainId,
            id: chain.id,
            url: chain.rpc,
            blockGasLimit: chain.gasOptions?.gasLimit,
            confirmations: chain.confirmations || 1,
            contracts: chain.contracts,
        };

        if (keys) {
            networks[name].accounts = keys.accounts || keys.chains[name]?.accounts;
        }

        // Add contract verification keys
        if (chain.explorer) {
            if (keys) {
                etherscan.apiKey[name] = keys.chains[name]?.api;
            }

            etherscan.customChains.push({
                network: name,
                chainId: chain.chainId,
                urls: {
                    apiURL: chain.explorer.api,
                    browserURL: chain.explorer.url,
                },
            });
        }
    });

    return { networks, etherscan };
};

/**
 * Verifies a contract on etherscan-like explorer of the provided chain using hardhat.
 * This assumes that the chain has been loaded as a custom network in hardhat.
 *
 * @async
 * @param {string} env
 * @param {string} chain
 * @param {string} contract
 * @param {any[]} args
 * @returns {Promise<void>}
 */
const verifyContract = async (env, chain, contract, args, contractPath = null) => {
    const stringArgs = args.map((arg) => JSON.stringify(arg));
    const content = `module.exports = [\n    ${stringArgs.join(',\n    ')}\n];`;
    const file = 'temp-arguments.js';
    const contractArg = contractPath ? `--contract ${contractPath}` : '';
    const cmd = `ENV=${env} npx hardhat verify --network ${chain.toLowerCase()} ${contractArg} --no-compile --constructor-args ${file} ${contract} --show-stack-traces`;

    return writeFileAsync(file, content, 'utf-8')
        .then(() => {
            console.log(`Verifying contract ${contract} with args '${stringArgs.join(',')}'`);
            console.log(cmd);

            return execAsync(cmd, { stdio: 'inherit' });
        })
        .then(() => {
            console.log('Verified!');
        });
};

const isString = (arg) => {
    return typeof arg === 'string' && arg !== '';
};

const isNumber = (arg) => {
    return Number.isInteger(arg);
};

const isAddressArray = (arg) => {
    if (!Array.isArray(arg)) return false;

    for (const ele of arg) {
        if (!isAddress(ele)) {
            return false;
        }
    }

    return true;
};

/**
 * Compute bytecode hash for a deployed contract or contract factory as it would appear on-chain.
 * Some chains don't use keccak256 for their state representation, which is taken into account by this function.
 * @param {Object} contractObject - An instance of the contract or a contract factory (ethers.js Contract or ContractFactory object)
 * @returns {Promise<string>} - The keccak256 hash of the contract bytecode
 */
async function getBytecodeHash(contractObject, chain = '', provider = null) {
    let bytecode;

    if (isString(contractObject)) {
        if (provider === null) {
            throw new Error('Provider must be provided for chain');
        }

        bytecode = await provider.getCode(contractObject);
    } else if (contractObject.address) {
        // Contract instance
        provider = contractObject.provider;
        bytecode = await provider.getCode(contractObject.address);
    } else if (contractObject.bytecode) {
        // Contract factory
        bytecode = contractObject.bytecode;
    } else {
        throw new Error('Invalid contract object. Expected ethers.js Contract or ContractFactory.');
    }

    if (chain.toLowerCase() === 'polygon-zkevm') {
        const codehash = await zkevm.smtUtils.hashContractBytecode(bytecode);
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

            if (!isString(deployerContract)) {
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

            if (!isString(deployerContract)) {
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

const getEVMAddresses = async (config, chainName, options) => {
    const keyID = options.keyId || '';

    const evmAddresses = options.amplifier
        ? await getAmplifierKeyAddresses(config, chainName, keyID)
        : await httpGet(`${config.axelar.lcd}/axelar/evm/v1beta1/key_address/${config.chains[chainName].id}?key_id=${keyID}`);

    const sortedAddresses = evmAddresses.addresses.sort((a, b) => a.address.toLowerCase().localeCompare(b.address.toLowerCase()));

    const addresses = sortedAddresses.map((weightedAddress) => weightedAddress.address);
    const weights = sortedAddresses.map((weightedAddress) => Number(weightedAddress.weight));
    const threshold = Number(evmAddresses.threshold);

    return { addresses, weights, threshold };
};

const getAmplifierKeyAddresses = async (config, chainName, keyID = '') => {
    const client = await CosmWasmClient.connect(config.axelar.rpc);
    const key = await client.queryContractSmart(config.axelar.contracts.Multisig.address, {
        get_key: { key_id: { owner: config.axelar.contracts.MultisigProver[chainName].address, subkey: keyID } },
    });
    const pubkeys = new Map(Object.entries(key.pub_keys));

    const weightedAddresses = Object.values(key.snapshot.participants).map((participant) => ({
        address: computeAddress(`0x${pubkeys.get(participant.address)}`),
        weight: participant.weight,
    }));

    return { addresses: weightedAddresses, threshold: key.snapshot.quorum };
};

function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}

function loadConfig(env) {
    return require(`${__dirname}/../info/${env}.json`);
}

function saveConfig(config, env) {
    writeJSON(config, `${__dirname}/../info/${env}.json`);
}

async function printWalletInfo(wallet) {
    printInfo('Wallet address', wallet.address);
    const balance = await wallet.provider.getBalance(wallet.address);
    printInfo('Wallet balance', `${balance / 1e18}`);
    printInfo('Wallet nonce', (await wallet.provider.getTransactionCount(wallet.address)).toString());

    if (balance.isZero()) {
        printError('Wallet balance is 0');
    }
}

const deployContract = async (
    deployMethod,
    wallet,
    contractJson,
    constructorArgs,
    deployOptions = {},
    gasOptions = {},
    verifyOptions = {},
) => {
    switch (deployMethod) {
        case 'create': {
            const contract = await deployCreate(wallet, contractJson, constructorArgs, gasOptions, verifyOptions);
            return contract;
        }

        case 'create2': {
            if (!isString(deployOptions.deployerContract)) {
                throw new Error('Deployer contract address was not provided');
            }

            if (!isString(deployOptions.salt)) {
                throw new Error('Salt was not provided');
            }

            const contract = await deployCreate2(
                deployOptions.deployerContract,
                wallet,
                contractJson,
                constructorArgs,
                deployOptions.salt,
                gasOptions.gasLimit,
                verifyOptions,
            );

            return contract;
        }

        case 'create3': {
            if (!isString(deployOptions.deployerContract)) {
                throw new Error('Deployer contract address was not provided');
            }

            if (!isString(deployOptions.salt)) {
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
            );

            return contract;
        }

        default: {
            throw new Error(`Invalid deployment method: ${deployMethod}`);
        }
    }
};

module.exports = {
    deployCreate,
    deployCreate2,
    deployCreate3,
    deployContract,
    readJSON,
    writeJSON,
    httpGet,
    importNetworks,
    verifyContract,
    printObj,
    printLog,
    printInfo,
    printWarn,
    printError,
    getBytecodeHash,
    predictAddressCreate,
    getDeployedAddress,
    isString,
    isNumber,
    isAddressArray,
    getProxy,
    getEVMAddresses,
    sleep,
    loadConfig,
    saveConfig,
    printWalletInfo,
};
