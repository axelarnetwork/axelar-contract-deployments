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
 * - ERC1967Proxy: ERC1967 proxy contract
 * - AxelarTransceiver: Transceiver contract
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
    linkLibrariesInContractJson,
    validateParameters,
} = require('./utils');
const { addEvmOptions } = require('./cli-utils');

/**
 * Generates constructor arguments for a given contract based on its configuration and options.
 */
async function getConstructorArgs(contractName, config, contractConfig, wallet, options) {
    // Safety check for undefined contractConfig
    if (!contractConfig) {
        throw new Error(
            `Contract configuration is undefined for ${contractName}. This may indicate a missing contract in the chain configuration.`,
        );
    }

    const args = options.args ? JSON.parse(options.args) : {};
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

        case 'AxelarTransceiver': {
            const gateway = config.AxelarGateway?.address;
            const gasService = config.AxelarGasService?.address;
            const gmpManager = options.gmpManager;

            validateParameters({
                isAddress: { gateway, gasService, gmpManager },
            });

            return [gateway, gasService, gmpManager];
        }

        case 'ERC1967Proxy': {
            // Handle proxy-specific arguments
            const forContract = options.forContract;
            const proxyData = options.proxyData || '0x';

            // If forContract is specified, try to get implementation from config
            if (forContract && config[forContract]?.address) {
                const implementationAddress = config[forContract].address;
                printInfo(`Using implementation address from ${forContract}: ${implementationAddress}`);
                return [implementationAddress, proxyData];
            }

            // Fallback to explicit args if provided
            const args = options.args ? JSON.parse(options.args) : [];
            if (args.length >= 2) {
                return args;
            }

            // If forContract was specified but not found, throw error
            if (forContract) {
                throw new Error(`Proxy for ${forContract} requires implementation address to be present in the config.`);
            }

            // If no forContract and no explicit args, throw error
            throw new Error(`ERC1967Proxy requires implementation address and init data.`);
        }
    }

    throw new Error(`${contractName} is not supported.`);
}

/**
 * Validates deployed contract configuration by checking contract state against expected values.
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
            const gmpManager = await contract.gmpManager();

            if (gateway !== contractConfig.gateway) {
                printError(`Expected gateway ${contractConfig.gateway} but got ${gateway}.`);
            }

            if (gasService !== contractConfig.gasService) {
                printError(`Expected gasService ${contractConfig.gasService} but got ${gasService}.`);
            }

            if (gmpManager !== contractConfig.gmpManager) {
                printError(`Expected gmpManager ${contractConfig.gmpManager} but got ${gmpManager}.`);
            }

            printInfo('Transceiver contract verification passed');
            break;
        }
    }
}

async function updateConfig(
    contractName,
    contract,
    contractConfig,
    options,
    wallet,
    deployMethod,
    codehash,
    predeployCodehash,
    salt,
    constructorArgs,
) {
    switch (contractName) {
        case 'ERC1967Proxy': {
            const targetContract = options.forContract;
            if (targetContract && contractConfig) {
                // Get implementation address from constructor args
                const implementationAddress = constructorArgs[0];
                // Only store if this proxy points to the target contract's implementation
                if (implementationAddress === contractConfig.address) {
                    contractConfig.proxyAddress = contract.address;
                    contractConfig.proxyDeployer = wallet.address;
                    contractConfig.proxyDeploymentMethod = deployMethod;
                    contractConfig.proxyCodehash = codehash;
                    contractConfig.proxyData = constructorArgs[1] || '0x';
                    if (deployMethod !== 'create') {
                        contractConfig.proxySalt = salt;
                    }
                    printInfo(`Stored proxy address ${contract.address} for ${targetContract}`);
                }
            }
            break;
        }
        case 'AxelarTransceiver': {
            contractConfig.address = contract.address;
            contractConfig.deployer = wallet.address;
            contractConfig.deploymentMethod = deployMethod;
            contractConfig.codehash = codehash;
            contractConfig.predeployCodehash = predeployCodehash;
            contractConfig.gateway = await contract.gateway();
            contractConfig.gasService = await contract.gasService();
            contractConfig.gmpManager = await contract.gmpManager();
            if (deployMethod !== 'create') {
                contractConfig.salt = salt;
            }
            break;
        }
        default: {
            contractConfig.address = contract.address;
            contractConfig.deployer = wallet.address;
            contractConfig.deploymentMethod = deployMethod;
            contractConfig.codehash = codehash;
            contractConfig.predeployCodehash = predeployCodehash;
            if (deployMethod !== 'create') {
                contractConfig.salt = salt;
            }
        }
    }
}

/**
 * Processes the deployment command for a specific chain.
 * Handles contract deployment, verification, and configuration updates.
 */
async function processCommand(config, chain, options) {
    const { env, artifactPath, contractName, deployMethod, privateKey, verify, yes, predictOnly } = options;
    const verifyOptions = verify ? { env, chain: chain.axelarId, only: verify === 'only' } : null;

    if (!chain.contracts) {
        chain.contracts = {};
    }

    const contracts = chain.contracts;

    let contractConfig;
    if (contractName === 'ERC1967Proxy') {
        if (!options.forContract) {
            throw new Error('ERC1967Proxy requires --forContract option to specify which contract this proxy is for.');
        }
        contractConfig = contracts[options.forContract];
        if (!contractConfig) {
            throw new Error(
                `Contract ${options.forContract} not found in chain configuration. Available contracts: ${Object.keys(contracts).join(', ')}`,
            );
        }
        if (contractConfig.proxyAddress && options.skipExisting) {
            printWarn(`Skipping proxy deployment for ${options.forContract} deployment on ${chain.name} because it is already deployed.`);
            return;
        }
    } else {
        if (!contracts[contractName]) {
            contracts[contractName] = {};
        }
        contractConfig = contracts[contractName];
        if (contractConfig.address && options.skipExisting) {
            printWarn(`Skipping ${contractName} deployment on ${chain.name} because it is already deployed.`);
            return;
        }
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    printInfo('Contract name', contractName);

    const contractJson = getContractJSON(contractName, artifactPath);
    const constructorArgs = await getConstructorArgs(contractName, contracts, contractConfig, wallet, options);

    // Parse libraries option if provided
    let linkedContractJson = contractJson;
    if (options.libraries) {
        let libraries;
        try {
            libraries = JSON.parse(options.libraries);
            console.log('Parsed libraries:', libraries);
        } catch (error) {
            console.log('JSON parse error:', error.message);
            throw new Error(`Invalid libraries JSON format: ${options.libraries}`);
        }
        linkedContractJson = linkLibrariesInContractJson(contractJson, libraries);
    }

    const predeployCodehash = await getBytecodeHash(linkedContractJson, chain.axelarId);
    printInfo('Pre-deploy Contract bytecode hash', predeployCodehash);
    const gasOptions = await getGasOptions(chain, options, contractName);

    printInfo(`Constructor args for chain ${chain.name}`, constructorArgs);

    const { deployerContract, salt } = getDeployOptions(deployMethod, options.salt || contractName, chain);

    const predictedAddress = await getDeployedAddress(wallet.address, deployMethod, {
        salt,
        deployerContract,
        contractJson: linkedContractJson,
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
        linkedContractJson,
        constructorArgs,
        { salt, deployerContract },
        gasOptions,
        verifyOptions,
        chain,
    );

    const codehash = await getBytecodeHash(contract, chain.axelarId);
    printInfo('Deployed Contract bytecode hash', codehash);

    await updateConfig(
        contractName,
        contract,
        contractConfig,
        options,
        wallet,
        deployMethod,
        codehash,
        predeployCodehash,
        salt,
        constructorArgs,
    );

    saveConfig(config, options.env);

    printInfo(`${chain.name} | ${contractName}`, contractConfig.address);

    await checkContract(contractName, contract, contractConfig);

    return contract;
}

/**
 * Main entry point for the deploy-contract script.
 * Processes deployment options and executes the deployment across specified chains.
 */
async function main(options) {
    await mainProcessor(options, processCommand);
}

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
    program.addOption(new Option('--forContract <forContract>', 'specify which contract this proxy is for (e.g., AxelarTransceiver)'));
    program.addOption(new Option('--proxyData <data>', 'specify initialization data for proxy (defaults to "0x" if not provided)'));
    program.addOption(
        new Option(
            '--libraries <libraries>',
            'JSON string of library addresses to link (e.g., \'{"full/path/Contract.sol:TransceiverStructs":"0x..."}\')',
        ),
    );
    program.addOption(new Option('--gmpManager <address>', 'GMP Manager address for AxelarTransceiver'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}

module.exports = { processCommand, getConstructorArgs };
