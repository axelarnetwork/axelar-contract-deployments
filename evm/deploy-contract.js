'use strict';

/**
 * @fileoverview EVM Contract Deployment Script
 *
 * This script provides functionality to deploy various Axelar contracts on EVM-compatible chains.
 * It supports multiple deployment methods (create, create2, create3) and handles contract
 * verification, configuration management, and deployment validation.
 *
 * Supported contract types:
 * - AxelarServiceGovernance: Governance contract for Axelar services
 * - InterchainProposalSender: Contract for sending interchain proposals
 * - InterchainGovernance: Interchain governance contract
 * - Multisig: Multi-signature wallet contract
 * - Operators: Operator management contract
 * - ConstAddressDeployer: Constant address deployer contract
 * - Create3Deployer: Create3 deployment contract
 * - TokenDeployer: Token deployment contract
 * - AxelarTransceiver: Axelar transceiver contract
 * - TransceiverStructs: Transceiver structures library
 *
 * @requires hardhat
 * @requires ethers
 * @requires commander
 * @requires ./utils
 * @requires ./cli-utils
 * @requires ./deploy-transceiver
 */

const chalk = require('chalk');
const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    utils: { isAddress, keccak256, toUtf8Bytes },
} = ethers;
const { Command, Option } = require('commander');
const {
    printInfo,
    printWarn,
    printError,
    getGasOptions,
    isNonEmptyString,
    isNumber,
    isAddressArray,
    getBytecodeHash,
    printWalletInfo,
    getDeployedAddress,
    deployContract,
    saveConfig,
    prompt,
    mainProcessor,
    isContract,
    getContractJSON,
    getDeployOptions,
} = require('./utils');
const { addEvmOptions } = require('./cli-utils');

// Import transceiver-specific functions from deploy-transceiver.js
const { linkLibraryToTransceiver } = require('./deploy-transceiver');

/**
 * Generates constructor arguments for a given contract based on its configuration and options.
 *
 * @param {string} contractName - The name of the contract to deploy
 * @param {Object} config - The chain configuration object containing contract configurations
 * @param {Object} wallet - The wallet instance used for deployment
 * @param {Object} options - Deployment options including custom args
 * @returns {Array} Array of constructor arguments for the contract
 * @throws {Error} When required configuration is missing or invalid
 */
async function getConstructorArgs(contractName, config, wallet, options) {
    const args = options.args ? JSON.parse(options.args) : {};
    const contractConfig = config[contractName];
    Object.assign(contractConfig, args);

    switch (contractName) {
        case 'AxelarServiceGovernance': {
            const gateway = config.AxelarGateway?.address;

            if (!isAddress(gateway)) {
                throw new Error(`Missing AxelarGateway address in the chain info.`);
            }

            const governanceChain = contractConfig.governanceChain || 'Axelarnet';
            contractConfig.governanceChain = governanceChain;

            if (!isNonEmptyString(governanceChain)) {
                throw new Error(`Missing AxelarServiceGovernance.governanceChain in the chain info.`);
            }

            const governanceAddress = contractConfig.governanceAddress || 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj';
            contractConfig.governanceAddress = governanceAddress;

            if (!isNonEmptyString(governanceAddress)) {
                throw new Error(`Missing AxelarServiceGovernance.governanceAddress in the chain info.`);
            }

            const minimumTimeDelay = contractConfig.minimumTimeDelay;

            if (!isNumber(minimumTimeDelay)) {
                throw new Error(`Missing AxelarServiceGovernance.minimumTimeDelay in the chain info.`);
            }

            const multisig = contractConfig.multisig;

            if (!isAddress(multisig)) {
                throw new Error(`Missing AxelarServiceGovernance.multisig address in the chain info.`);
            }

            return [gateway, governanceChain, governanceAddress, minimumTimeDelay, multisig];
        }

        case 'InterchainProposalSender': {
            const gateway = config.AxelarGateway?.address;
            const gasService = config.AxelarGasService?.address;

            if (!isAddress(gateway)) {
                throw new Error(`Missing AxelarGateway address in the chain info.`);
            }

            if (!isAddress(gasService)) {
                throw new Error(`Missing AxelarGasService address in the chain info.`);
            }

            return [gateway, gasService];
        }

        case 'InterchainGovernance': {
            const gateway = config.AxelarGateway?.address;

            if (!isAddress(gateway)) {
                throw new Error(`Missing AxelarGateway address in the chain info.`);
            }

            const governanceChain = contractConfig.governanceChain || 'Axelarnet';
            contractConfig.governanceChain = governanceChain;

            if (!isNonEmptyString(governanceChain)) {
                throw new Error(`Missing InterchainGovernance.governanceChain in the chain info.`);
            }

            const governanceAddress = contractConfig.governanceAddress || 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj';
            contractConfig.governanceAddress = governanceAddress;

            if (!isNonEmptyString(governanceAddress)) {
                throw new Error(`Missing InterchainGovernance.governanceAddress in the chain info.`);
            }

            const minimumTimeDelay = contractConfig.minimumTimeDelay;

            if (!isNumber(minimumTimeDelay)) {
                throw new Error(`Missing InterchainGovernance.minimumTimeDelay in the chain info.`);
            }

            return [gateway, governanceChain, governanceAddress, minimumTimeDelay];
        }

        case 'Multisig': {
            const signers = contractConfig.signers;

            if (!isAddressArray(signers)) {
                throw new Error(`Missing Multisig.signers in the chain info.`);
            }

            const threshold = contractConfig.threshold;

            if (!isNumber(threshold)) {
                throw new Error(`Missing Multisig.threshold in the chain info.`);
            }

            return [signers, threshold];
        }

        case 'Operators': {
            let owner = contractConfig.owner;

            if (!owner) {
                owner = wallet.address;
                contractConfig.owner = owner;
            } else if (!isAddress(owner)) {
                throw new Error(`Invalid Operators.owner in the chain info.`);
            }

            return [owner];
        }

        case 'ConstAddressDeployer': {
            return [];
        }

        case 'Create3Deployer': {
            return [];
        }

        case 'TokenDeployer': {
            return [];
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

        case 'ERC1967Proxy': {
            const args = options.args ? JSON.parse(options.args) : [];
            if (args.length < 2) {
                throw new Error(`ERC1967Proxy requires implementation address and init data.`);
            }
            return args;
        }
    }

    throw new Error(`${contractName} is not supported.`);
}

/**
 * Validates deployed contract configuration by checking contract state against expected values.
 *
 * @param {string} contractName - The name of the deployed contract
 * @param {Object} contract - The deployed contract instance
 * @param {Object} contractConfig - The expected contract configuration
 * @returns {Promise<void>}
 */
async function checkContract(contractName, contract, contractConfig) {
    switch (contractName) {
        case 'Operators': {
            const owner = await contract.owner();

            if (owner !== contractConfig.owner) {
                printError(`Expected owner ${contractConfig.owner} but got ${owner}.`);
            }

            break;
        }

        case 'InterchainGovernance': {
            const governanceChain = await contract.governanceChain();

            if (governanceChain !== contractConfig.governanceChain) {
                printError(`Expected governanceChain ${contractConfig.governanceChain} but got ${governanceChain}.`);
            }

            const governanceChainHash = await contract.governanceChainHash();
            const expectedChainHash = keccak256(toUtf8Bytes(contractConfig.governanceChain));

            if (governanceChainHash !== expectedChainHash) {
                printError(`Expected governanceChainHash ${expectedChainHash} but got ${governanceChainHash}.`);
            }

            const governanceAddress = await contract.governanceAddress();

            if (governanceAddress !== contractConfig.governanceAddress) {
                printError(`Expected governanceAddress ${contractConfig.governanceAddress} but got ${governanceAddress}.`);
            }

            const governanceAddressHash = await contract.governanceAddressHash();
            const expectedAddressHash = keccak256(toUtf8Bytes(contractConfig.governanceAddress));

            if (governanceAddressHash !== expectedAddressHash) {
                printError(`Expected governanceAddressHash ${expectedAddressHash} but got ${governanceAddressHash}.`);
            }

            const minimumTimeDelay = await contract.minimumTimeLockDelay();

            if (!minimumTimeDelay.eq(contractConfig.minimumTimeDelay)) {
                printError(`Expected minimumTimeDelay ${contractConfig.minimumTimeDelay} but got ${minimumTimeDelay}.`);
            }

            break;
        }

        case 'AxelarTransceiver': {
            const gateway = await contract.gateway();
            const gasService = await contract.gasService();
            const nttManager = await contract.nttManager();

            if (gateway !== contractConfig.gateway) {
                printError(`Expected gateway ${contractConfig.gateway} but got ${gateway}.`);
            }

            if (gasService !== contractConfig.gasService) {
                printError(`Expected gasService ${contractConfig.gasService} but got ${gasService}.`);
            }

            if (nttManager !== contractConfig.nttManager) {
                printError(`Expected nttManager ${contractConfig.nttManager} but got ${nttManager}.`);
            }

            printInfo('Transceiver contract verification passed');
            break;
        }
    }
}

/**
 * Processes the deployment command for a specific chain.
 * Handles contract deployment, verification, and configuration updates.
 *
 * @param {Object} config - The global configuration object
 * @param {Object} chain - The chain configuration object
 * @param {Object} options - Deployment options including:
 *   - {string} env - Environment name
 *   - {string} artifactPath - Path to contract artifacts
 *   - {string} contractName - Name of contract to deploy
 *   - {string} deployMethod - Deployment method (create/create2/create3)
 *   - {string} privateKey - Private key for deployment
 *   - {boolean|string} verify - Verification options
 *   - {boolean} yes - Skip confirmation prompts
 *   - {boolean} predictOnly - Only predict address without deploying
 * @returns {Promise<void>}
 */
async function deployEvmContract(config, chain, options) {
    const { env, artifactPath, contractName, deployMethod, privateKey, verify, yes, predictOnly } = options;
    const verifyOptions = verify ? { env, chain: chain.axelarId, only: verify === 'only' } : null;

    if (!chain.contracts) {
        chain.contracts = {};
    }

    const contracts = chain.contracts;

    if (!contracts[contractName]) {
        contracts[contractName] = {};
    }

    const contractConfig = contracts[contractName];

    if (contractConfig.address && options.skipExisting) {
        printWarn(`Skipping ${contractName} deployment on ${chain.name} because it is already deployed.`);
        return;
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    printInfo('Contract name', contractName);

    const contractJson = getContractJSON(contractName, artifactPath);

    // Special handling for AxelarTransceiver - link the library
    if (contractName === 'AxelarTransceiver') {
        const libraryAddress = contracts.TransceiverStructs?.address;
        if (!libraryAddress) {
            throw new Error('TransceiverStructs library address not found. Deploy it first.');
        }

        linkLibraryToTransceiver(contractJson, libraryAddress);
    }

    const predeployCodehash = await getBytecodeHash(contractJson, chain.axelarId);
    printInfo('Pre-deploy Contract bytecode hash', predeployCodehash);

    const constructorArgs = await getConstructorArgs(contractName, contracts, wallet, options);
    const gasOptions = await getGasOptions(chain, options, contractName);

    printInfo(`Constructor args for chain ${chain.name}`, constructorArgs);

    const { deployerContract, salt } = getDeployOptions(deployMethod, options.salt || contractName, chain);

    const predictedAddress = await getDeployedAddress(wallet.address, deployMethod, {
        salt,
        deployerContract,
        contractJson,
        constructorArgs,
        provider: wallet.provider,
    });

    if (await isContract(predictedAddress, provider)) {
        printWarn(`Contract ${contractName} is already deployed on ${chain.name} at ${predictedAddress}`);
        return;
    }

    if (deployMethod !== 'create') {
        printInfo(`${contractName} deployment salt`, salt);
    }

    printInfo('Deployment method', deployMethod);
    printInfo('Deployer contract', deployerContract);
    printInfo(`${contractName} will be deployed to`, predictedAddress, chalk.cyan);

    let existingAddress, existingCodeHash;

    for (const chainConfig of Object.values(config.chains)) {
        existingAddress = chainConfig.contracts?.[contractName]?.address;
        existingCodeHash = chainConfig.contracts?.[contractName]?.predeployCodehash;

        if (existingAddress !== undefined) {
            break;
        }
    }

    if (existingAddress !== undefined && predictedAddress !== existingAddress) {
        printWarn(`Predicted address ${predictedAddress} does not match existing deployment ${existingAddress} in chain configs.`);

        if (predeployCodehash !== existingCodeHash) {
            printWarn(
                `Pre-deploy bytecode hash ${predeployCodehash} does not match existing deployment's predeployCodehash ${existingCodeHash} in chain configs.`,
            );
        }

        printWarn('For official deployment, recheck the deployer, salt, args, or contract bytecode.');
        printWarn('This is NOT required if the deployments are done by different integrators');
    }

    if (predictOnly || prompt(`Proceed with deployment on ${chain.name}?`, yes)) {
        return;
    }

    const contract = await deployContract(
        deployMethod,
        wallet,
        contractJson,
        constructorArgs,
        { salt, deployerContract },
        gasOptions,
        verifyOptions,
        chain,
    );

    const codehash = await getBytecodeHash(contract, chain.axelarId);
    printInfo('Deployed Contract bytecode hash', codehash);

    contractConfig.address = contract.address;
    contractConfig.deployer = wallet.address;
    contractConfig.deploymentMethod = deployMethod;
    contractConfig.codehash = codehash;
    contractConfig.predeployCodehash = predeployCodehash;

    // Special handling for AxelarTransceiver - store additional config
    if (contractName === 'AxelarTransceiver') {
        const args = options.args ? JSON.parse(options.args) : {};
        contractConfig.gateway = args.gateway;
        contractConfig.gasService = args.gasService;
        contractConfig.nttManager = args.nttManager;
    }

    // Special handling for ERC1967Proxy - store proxy address in AxelarTransceiver config
    if (contractName === 'ERC1967Proxy' && contracts.AxelarTransceiver) {
        contracts.AxelarTransceiver.proxyAddress = contract.address;
        contracts.AxelarTransceiver.proxyDeployer = wallet.address;
        contracts.AxelarTransceiver.proxyDeploymentMethod = deployMethod;
        contracts.AxelarTransceiver.proxyCodehash = codehash;
        if (deployMethod !== 'create') {
            contracts.AxelarTransceiver.proxySalt = salt;
        }
    }

    if (deployMethod !== 'create') {
        contractConfig.salt = salt;
    }

    saveConfig(config, options.env);

    printInfo(`${chain.name} | ${contractName}`, contractConfig.address);

    await checkContract(contractName, contract, contractConfig);

    return contract;
}

/**
 * Main entry point for the deploy-contract script.
 * Processes deployment options and executes the deployment across specified chains.
 *
 * @param {Object} options - Command line options and configuration
 * @returns {Promise<void>}
 */
async function main(options) {
    await mainProcessor(options, deployEvmContract);
}

// CLI setup and execution
if (require.main === module) {
    const program = new Command();

    program.name('deploy-contract').description('Deploy contracts using create, create2, or create3');

    addEvmOptions(program, {
        artifactPath: true,
        contractName: true,
        salt: true,
        skipChains: true,
        skipExisting: true,
        predictOnly: true,
    });

    program.addOption(
        new Option('-m, --deployMethod <deployMethod>', 'deployment method').choices(['create', 'create2', 'create3']).default('create2'),
    );
    program.addOption(new Option('--ignoreError', 'ignore errors during deployment for a given chain'));
    program.addOption(new Option('--args <args>', 'custom deployment args'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}

// Export functions for use by other modules
module.exports = {
    deployEvmContract,
    getConstructorArgs,
    checkContract,
};
