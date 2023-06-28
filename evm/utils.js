'use strict';

const {
    ContractFactory,
    utils: { isAddress },
} = require('ethers');
const http = require('http');
const { outputJsonSync, readJsonSync } = require('fs-extra');
const { exec } = require('child_process');
const { writeFile } = require('fs');
const { promisify } = require('util');
const chalk = require('chalk');

const execAsync = promisify(exec);
const writeFileAsync = promisify(writeFile);

const deployContract = async (wallet, contractJson, args = [], options = {}) => {
    const factory = new ContractFactory(contractJson.abi, contractJson.bytecode, wallet);

    const contract = await factory.deploy(...args, { ...options });
    await contract.deployed();
    return contract;
};

const printObj = (obj) => {
    console.log(JSON.stringify(obj, null, 2));
};

const printInfo = (msg, info) => {
    console.log(`${msg}: ${chalk.green(info)}`);
};

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
        http.get(url, (res) => {
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

    if (chains.chains) {
        // Use new info format
        chains = Object.values(chains.chains);
    }

    // Add custom networks
    chains.forEach((chain) => {
        const name = chain.name.toLowerCase();
        networks[name] = {
            chainId: chain.chainId,
            id: chain.id,
            url: chain.rpc,
            blockGasLimit: chain.gasOptions?.gasLimit,
            confirmations: chain.confirmations || 1,
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
const verifyContract = async (env, chain, contract, args) => {
    const stringArgs = args.map((arg) => JSON.stringify(arg));
    const content = `module.exports = [\n    ${stringArgs.join(',\n    ')}\n];`;
    const file = 'temp-arguments.js';
    const cmd = `ENV=${env} npx hardhat verify --network ${chain.toLowerCase()} --no-compile --constructor-args ${file} ${contract} --show-stack-traces`;

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
    return typeof arg === 'string';
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

module.exports = {
    deployContract,
    readJSON,
    writeJSON,
    httpGet,
    importNetworks,
    verifyContract,
    printObj,
    printInfo,
    isString,
    isNumber,
    isAddressArray,
};
