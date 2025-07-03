'use strict';

/**
 * @fileoverview EVM Upgradable Contract Deployment Script
 *
 * This script provides functionality to deploy upgradable contracts using the proxy pattern
 * on EVM-compatible chains. It supports multiple deployment methods (create, create2, create3)
 * and handles contract verification, configuration management, and deployment validation.
 *
 * Supported upgradable contract types:
 * - AxelarGasService: Gas service contract with upgrade capability
 * - AxelarDepositService: Deposit service contract with upgrade capability
 * - TransceiverStructs: Library for transceiver structures
 * - AxelarTransceiver: Transceiver contract with upgrade capability
 *
 * @requires hardhat
 * @requires ethers
 * @requires commander
 * @requires ./upgradable
 * @requires ./utils
 * @requires ./cli-utils
 */

const chalk = require('chalk');
const { ethers } = require('hardhat');
const {
    Contract,
    Wallet,
    getDefaultProvider,
    utils: { isAddress },
} = ethers;
const IUpgradable = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IUpgradable.json');
const { Command, Option } = require('commander');

const { deployUpgradable, deployCreate2Upgradable, deployCreate3Upgradable, upgradeUpgradable } = require('./upgradable');
const {
    printInfo,
    printError,
    printWalletInfo,
    getDeployedAddress,
    prompt,
    getGasOptions,
    getDeployOptions,
    mainProcessor,
} = require('./utils');
const { addEvmOptions } = require('./cli-utils');

/**
 * Creates a proxy contract instance for interacting with an upgradable contract.
 *
 * @param {Object} wallet - The wallet instance
 * @param {string} proxyAddress - The proxy contract address
 * @returns {Object} The proxy contract instance
 */
function getProxy(wallet, proxyAddress) {
    return new Contract(proxyAddress, IUpgradable.abi, wallet);
}

/**
 * Links the TransceiverStructs library to the AxelarTransceiver bytecode.
 * Uses the correct Solidity library placeholder format with keccak256 hash.
 *
 * @param {Object} transceiverJson - The contract JSON object
 * @param {string} libraryAddress - The library address to link
 * @returns {Object} A new contract JSON object with linked library
 */
function linkLibraryToTransceiver(transceiverJson, libraryAddress) {
    // Create a copy to avoid modifying the cached object
    const linkedJson = JSON.parse(JSON.stringify(transceiverJson));

    // Solidity generates library placeholders as: __$<keccak256(libraryName).slice(0, 34)>$__
    const libraryName = 'TransceiverStructs';
    const libraryNameHash = ethers.utils.keccak256(ethers.utils.toUtf8Bytes(libraryName));
    const libraryPlaceholder = `__$${libraryNameHash.slice(2, 36)}__`; // Remove '0x' and take 34 chars

    // Ensure the library address is properly formatted (40 hex chars without 0x)
    const libraryAddressPadded = libraryAddress.replace('0x', '').padStart(40, '0');

    // Replace the placeholder in the bytecode
    if (linkedJson.bytecode.includes(libraryPlaceholder)) {
        linkedJson.bytecode = linkedJson.bytecode.replace(libraryPlaceholder, libraryAddressPadded);
    } else {
        throw new Error(`Library placeholder '${libraryPlaceholder}' not found in bytecode. Library linking failed.`);
    }

    return linkedJson;
}

/**
 * Generates implementation constructor arguments for a given contract based on its configuration and options.
 *
 * @param {string} contractName - The name of the contract to deploy
 * @param {Object} config - The chain configuration object containing contract configurations
 * @param {Object} options - Deployment options including custom args
 * @returns {Array} Array of constructor arguments for the implementation contract
 * @throws {Error} When required configuration is missing or invalid
 */
async function getImplementationArgs(contractName, config, options) {
    let args;

    try {
        args = options.args ? JSON.parse(options.args) : {};
    } catch (error) {
        console.error('Error parsing args:\n', error.message);
    }

    const contractConfig = config[contractName];
    Object.assign(contractConfig, args);

    switch (contractName) {
        case 'AxelarGasService': {
            const collector = contractConfig.collector;

            if (!isAddress(collector)) {
                throw new Error(`Missing AxelarGasService.collector ${collector}.`);
            }

            return [collector];
        }

        case 'AxelarDepositService': {
            const symbol = contractConfig.wrappedSymbol;

            if (symbol === undefined) {
                throw new Error(`Missing AxelarDepositService.wrappedSymbol in the chain info.`);
            } else if (symbol === '') {
                console.log(`${config.name} | AxelarDepositService.wrappedSymbol: wrapped token is disabled`);
            }

            const refundIssuer = contractConfig.refundIssuer;

            if (!isAddress(refundIssuer)) {
                throw new Error(`${config.name} | Missing AxelarDepositService.refundIssuer in the chain info.`);
            }

            const gateway = config.AxelarGateway?.address;

            if (!isAddress(gateway)) {
                throw new Error(`Missing AxelarGateway address in the chain info.`);
            }

            return [gateway, symbol, refundIssuer];
        }

        case 'TransceiverStructs': {
            return [];
        }

        case 'AxelarTransceiver': {
            const gateway = config.AxelarGateway?.address;
            const gasService = config.AxelarGasService?.address;
            const nttManager = options.nttManager;

            if (!isAddress(gateway)) {
                throw new Error(`Missing AxelarGateway address in the chain info.`);
            }

            if (!isAddress(gasService)) {
                throw new Error(`Missing AxelarGasService address in the chain info.`);
            }

            if (!isAddress(nttManager)) {
                throw new Error(`Missing NTT Manager address. Please provide --nttManager parameter.`);
            }

            return [gateway, gasService, nttManager];
        }
    }

    throw new Error(`${contractName} is not supported.`);
}

/**
 * Generates initialization arguments for proxy setup.
 *
 * @param {string} contractName - The name of the contract
 * @returns {string} The initialization arguments as a hex string
 * @throws {Error} When contract is not supported
 */
function getInitArgs(contractName) {
    switch (contractName) {
        case 'AxelarGasService': {
            return '0x';
        }

        case 'AxelarDepositService': {
            return '0x';
        }

        case 'TransceiverStructs': {
            return '0x';
        }

        case 'AxelarTransceiver': {
            return '0x';
        }
    }

    throw new Error(`${contractName} is not supported.`);
}

/**
 * Generates upgrade arguments for contract upgrades.
 *
 * @param {string} contractName - The name of the contract
 * @returns {string} The upgrade arguments as a hex string
 * @throws {Error} When contract is not supported
 */
function getUpgradeArgs(contractName) {
    switch (contractName) {
        case 'AxelarGasService': {
            return '0x';
        }

        case 'AxelarDepositService': {
            return '0x';
        }

        case 'TransceiverStructs': {
            return '0x';
        }

        case 'AxelarTransceiver': {
            return '0x';
        }
    }

    throw new Error(`${contractName} is not supported.`);
}

/**
 * Deploy or upgrade an upgradable contract that's based on the init proxy pattern.
 * This function handles both initial deployment and upgrades of upgradable contracts.
 *
 * @param {Object} config - The global configuration object (unused, kept for compatibility)
 * @param {Object} chain - The chain configuration object
 * @param {Object} options - Deployment options
 * @param {string} options.contractName - The name of the contract to deploy
 * @param {string} options.deployMethod - The deployment method (create, create2, create3)
 * @param {string} options.privateKey - The private key for deployment
 * @param {boolean} options.upgrade - Whether to perform an upgrade
 * @param {string} options.verifyEnv - Environment for contract verification
 * @param {boolean} options.yes - Skip confirmation prompts
 * @param {boolean} options.predictOnly - Only predict address without deploying
 * @returns {Promise<Object|null>} The deployed contract or null if cancelled
 */
async function deployEvmUpgradableContract(_, chain, options) {
    const { contractName, deployMethod, privateKey, upgrade, verifyEnv, yes, predictOnly } = options;
    const verifyOptions = verifyEnv ? { env: verifyEnv, chain: chain.axelarId } : null;

    if (deployMethod === 'create3' && (contractName === 'AxelarGasService' || contractName === 'AxelarDepositService')) {
        printError(`${deployMethod} not supported for ${contractName}`);
        return;
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);
    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    const artifactPath =
        options.artifactPath ||
        '@axelar-network/axelar-cgp-solidity/artifacts/contracts/' +
            (() => {
                switch (contractName) {
                    case 'AxelarGasService':
                        return 'gas-service/';
                    case 'AxelarDepositService':
                        return 'deposit-service/';
                    case 'TransceiverStructs':
                        return 'transceiver-structs/';
                    case 'AxelarTransceiver':
                        return 'transceiver/';
                    default:
                        return '';
                }
            })();

    const implementationPath = artifactPath + contractName + '.sol/' + contractName + '.json';
    const proxyPath = artifactPath + contractName + 'Proxy.sol/' + contractName + 'Proxy.json';
    const implementationJson = require(implementationPath);
    const proxyJson = require(proxyPath);

    const contracts = chain.contracts;

    if (!contracts[contractName]) {
        contracts[contractName] = {};
    }

    const contractConfig = contracts[contractName];
    const implArgs = await getImplementationArgs(contractName, contracts, options);
    const gasOptions = await getGasOptions(chain, options, contractName);
    printInfo(`Implementation args for chain ${chain.name}`, implArgs);
    const { deployerContract, salt } = getDeployOptions(deployMethod, options.salt || contractName, chain);

    // Special handling for AxelarTransceiver - link the library
    if (contractName === 'AxelarTransceiver') {
        const libraryAddress = contracts.TransceiverStructs?.address;
        if (!libraryAddress) {
            throw new Error('TransceiverStructs library address not found. Deploy it first.');
        }

        implementationJson = linkLibraryToTransceiver(implementationJson, libraryAddress);
    }

    if (upgrade) {
        if (!contractConfig.address) {
            throw new Error(`${chain.name} | Contract ${contractName} is not deployed.`);
        }

        const contract = getProxy(wallet.connect(provider), contractConfig.address);
        const owner = await contract.owner();
        printInfo(`Upgrading proxy on ${chain.name}`, contract.address);
        printInfo('Existing implementation', await contract.implementation());
        printInfo('Existing owner', owner);

        if (wallet.address !== owner) {
            throw new Error(
                `${chain.name} | Signer ${wallet.address} does not match contract owner ${owner} for chain ${chain.name} in info.`,
            );
        }

        if (predictOnly || prompt(`Perform an upgrade for ${chain.name}?`, yes)) {
            return;
        }

        await upgradeUpgradable(
            deployMethod,
            contractConfig.address,
            wallet.connect(provider),
            implementationJson,
            implArgs,
            getUpgradeArgs(contractName, chain),
            {
                deployerContract,
                salt: `${salt} Implementation`,
            },
            gasOptions,
            verifyOptions,
            chain.axelarId,
            options,
        );

        contractConfig.implementation = await contract.implementation();

        console.log(`${chain.name} | New Implementation for ${contractName} is at ${contractConfig.implementation}`);
        console.log(`${chain.name} | Upgraded.`);
    } else {
        const setupArgs = getInitArgs(contractName, contracts);
        printInfo('Proxy setup args', setupArgs);

        const predictedAddress = await getDeployedAddress(wallet.address, deployMethod, {
            salt,
            deployerContract,
            contractJson: proxyJson,
            constructorArgs: [],
            provider: wallet.provider,
            nonce: (await wallet.provider.getTransactionCount(wallet.address)) + 1,
        });

        if (deployMethod !== 'create') {
            printInfo(`${contractName} deployment salt`, salt);
        }

        printInfo('Deployment method', deployMethod);
        printInfo('Deployer contract', deployerContract);
        printInfo(`${contractName} will be deployed to`, predictedAddress, chalk.cyan);

        if (predictOnly || prompt(`Does derived address match existing deployments? Proceed with deployment on ${chain.name}?`, yes)) {
            return;
        }

        let contract;

        switch (deployMethod) {
            case 'create': {
                contract = await deployUpgradable(
                    wallet,
                    implementationJson,
                    proxyJson,
                    implArgs,
                    [],
                    setupArgs,
                    gasOptions,
                    verifyOptions,
                );
                break;
            }

            case 'create2': {
                contract = await deployCreate2Upgradable(
                    deployerContract,
                    wallet,
                    implementationJson,
                    proxyJson,
                    implArgs,
                    [],
                    setupArgs,
                    salt,
                    gasOptions,
                    verifyOptions,
                );

                contractConfig.salt = salt;
                printInfo(`${chain.name} | ConstAddressDeployer`, deployerContract);
                break;
            }

            case 'create3': {
                contract = await deployCreate3Upgradable(
                    deployerContract,
                    wallet,
                    implementationJson,
                    proxyJson,
                    implArgs,
                    [],
                    setupArgs,
                    salt,
                    gasOptions,
                    verifyOptions,
                );

                contractConfig.salt = salt;
                printInfo(`${chain.name} | Create3Deployer`, deployerContract);
                break;
            }

            default: {
                throw new Error(`Unknown deployment method ${deployMethod}`);
            }
        }

        contractConfig.address = contract.address;
        contractConfig.implementation = await contract.implementation();
        contractConfig.deployer = wallet.address;

        printInfo(`${chain.name} | Implementation for ${contractName}`, contractConfig.implementation);
        printInfo(`${chain.name} | Proxy for ${contractName}`, contractConfig.address);

        const owner = await contract.owner();

        if (owner !== wallet.address) {
            printError(`${chain.name} | Signer ${wallet.address} does not match contract owner ${owner} for chain ${chain.name} in info.`);
        }

        return contract;
    }
}

/**
 * Main entry point for the deploy-upgradable script.
 * Processes deployment options and executes the deployment across specified chains.
 *
 * @param {Object} options - Command line options and configuration
 * @returns {Promise<void>}
 */
async function main(options) {
    await mainProcessor(options, deployEvmUpgradableContract);
}

// CLI setup and execution
if (require.main === module) {
    const program = new Command();

    program.name('deploy-upgradable').description('Deploy upgradable contracts');

    addEvmOptions(program, { artifactPath: true, contractName: true, salt: true, skipChains: true, upgrade: true, predictOnly: true });

    program.addOption(
        new Option('-m, --deployMethod <deployMethod>', 'deployment method').choices(['create', 'create2', 'create3']).default('create2'),
    );
    program.addOption(new Option('--args <args>', 'customize deployment args'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}

module.exports = { deployEvmUpgradableContract };
