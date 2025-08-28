'use strict';

const chalk = require('chalk');
const { ethers } = require('hardhat');
const { Contract } = ethers;
const {
    Wallet,
    getDefaultProvider,
    utils: { keccak256, toUtf8Bytes },
} = ethers;
const { Command, Option } = require('commander');
const {
    printInfo,
    printWarn,
    printError,
    getGasOptions,
    getBytecodeHash,
    printWalletInfo,
    getDeployedAddress,
    deployContract,
    prompt,
    mainProcessor,
    isContract,
    getContractJSON,
    getDeployOptions,
    validateParameters,
} = require('./utils');
const { addEvmOptions } = require('./cli-utils');

async function upgradeMonadAxelarTransceiver(contractConfig, contractAbi, wallet, chain, options, gasOptions) {
    const proxyAddress = contractConfig.address;
    // using new MonadAxelarTransceiver contract's address, which is recently deployed; part of the two-step upgrade process
    const newImplementation = contractConfig.implementation;

    validateParameters({
        isAddress: { proxyAddress, newImplementation },
    });

    const proxyContract = new Contract(proxyAddress, contractAbi, wallet);

    printInfo(`MonadAxelarTransceiver Proxy`, proxyAddress);
    printInfo(`New implementation`, newImplementation);

    if (prompt(`Proceed with upgrade on MonadAxelarTransceiver on ${chain.name}?`, options.yes)) {
        return;
    }

    const upgradeTx = await proxyContract.upgrade(newImplementation, gasOptions);
    await upgradeTx.wait();

    printInfo('Upgrade completed successfully');
}

/**
 * Generates constructor arguments for a given contract based on its configuration and options.
 */
async function getConstructorArgs(contractName, contracts, contractConfig, wallet, options) {
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
            const gateway = contracts.AxelarGateway?.address;
            const governanceChain = contractConfig.governanceChain || 'Axelarnet';
            contractConfig.governanceChain = governanceChain;
            const governanceAddress = contractConfig.governanceAddress || 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj';
            contractConfig.governanceAddress = governanceAddress;
            const minimumTimeDelay = contractConfig.minimumTimeDelay;
            const multisig = contractConfig.multisig;

            validateParameters({
                isAddress: { gateway, multisig },
                isNonEmptyString: { governanceChain, governanceAddress },
                isNumber: { minimumTimeDelay },
            });

            return [gateway, governanceChain, governanceAddress, minimumTimeDelay, multisig];
        }

        case 'InterchainProposalSender': {
            const gateway = contracts.AxelarGateway?.address;
            const gasService = contracts.AxelarGasService?.address;

            validateParameters({
                isAddress: { gateway, gasService },
            });

            return [gateway, gasService];
        }

        case 'InterchainGovernance': {
            const gateway = contracts.AxelarGateway?.address;
            const governanceChain = contractConfig.governanceChain || 'Axelarnet';
            contractConfig.governanceChain = governanceChain;
            const governanceAddress = contractConfig.governanceAddress || 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj';
            contractConfig.governanceAddress = governanceAddress;
            const minimumTimeDelay = contractConfig.minimumTimeDelay;

            validateParameters({
                isAddress: { gateway },
                isNonEmptyString: { governanceChain, governanceAddress },
                isNumber: { minimumTimeDelay },
            });

            return [gateway, governanceChain, governanceAddress, minimumTimeDelay];
        }

        case 'Multisig': {
            const signers = contractConfig.signers;
            const threshold = contractConfig.threshold;

            validateParameters({
                isAddressArray: { signers },
                isNumber: { threshold },
            });

            return [signers, threshold];
        }

        case 'Operators': {
            let owner = contractConfig.owner;

            if (!owner) {
                owner = wallet.address;
                contractConfig.owner = owner;
            } else {
                validateParameters({
                    isAddress: { owner },
                });
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

        case 'MonadAxelarTransceiver': {
            const gateway = contracts.AxelarGateway?.address;
            const gasService = contracts.AxelarGasService?.address;
            const gmpManager = options.gmpManager ? options.gmpManager : contracts.MonadAxelarTransceiver.gmpManager;

            if (!options.gmpManager) {
                printWarn(`--gmpManager is not provided. Using gmpManager from chain config.`);
            }

            validateParameters({
                isAddress: { gateway, gasService, gmpManager },
            });

            return [gateway, gasService, gmpManager];
        }

        case 'ERC1967Proxy': {
            const forContract = options.forContract;
            const proxyData = options.proxyData || '0x';

            const implementationAddress = contracts[forContract]?.implementation;

            validateParameters({
                isAddress: { implementationAddress },
            });

            return [implementationAddress, proxyData];
        }
    }

    throw new Error(`${contractName} is not supported.`);
}

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

        case 'MonadAxelarTransceiver': {
            const gateway = await contract.gateway();
            const gasService = await contract.gasService();
            const gmpManager = await contract.nttManager();

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

async function processCommand(_axelar, chain, chains, options) {
    const { env, artifactPath, contractName, privateKey, verify, yes, predictOnly, upgrade, reuseProxy } = options;

    let { deployMethod } = options;
    const verifyOptions = verify ? { env, chain: chain.axelarId, only: verify === 'only' } : null;

    if (!chain.contracts) {
        chain.contracts = {};
    }

    const contracts = chain.contracts;

    let contractConfig;
    switch (contractName) {
        case 'ERC1967Proxy': {
            if (!artifactPath) {
                printError(`--artifactPath is required. Please provide the path to the compiled artifacts.`);
                return;
            }

            if (!options.forContract) {
                printError('--forContract is required. Please specify which contract this proxy is for.');
                return;
            }
            contractConfig = contracts[options.forContract];
            if (!contractConfig) {
                printError(`Contract ${options.forContract} not found in chain config.`);
                return;
            }

            if (contractConfig.address && options.skipExisting) {
                printWarn(`Skipping ${options.forContract} on ${chain.name} because it is already deployed.`);
                return;
            }
            break;
        }

        case 'MonadAxelarTransceiver': {
            if (!artifactPath) {
                printError('--artifactPath is required. Please provide the path to the compiled artifacts.');
                return;
            }

            if (!contracts[contractName]) {
                contracts[contractName] = {};
            }
            contractConfig = contracts[contractName];

            // Handle reuseProxy case
            if (reuseProxy) {
                if (!contractConfig.implementation) {
                    printError(`MonadAxelarTransceiver is not deployed on ${chain.name}. Cannot reuse proxy.`);
                    return;
                }
                printInfo(`Reusing existing MonadAxelarTransceiver proxy on ${chain.name}`);
            }

            if (contractConfig.implementation && options.skipExisting) {
                printWarn(`Skipping ${contractName} deployment on ${chain.name} because it is already deployed.`);
                return;
            }
            break;
        }

        default: {
            if (!contracts[contractName]) {
                contracts[contractName] = {};
            }
            contractConfig = contracts[contractName];

            if (contractConfig.address && options.skipExisting) {
                printWarn(`Skipping ${contractName} deployment on ${chain.name} because it is already deployed.`);
                return;
            }
            break;
        }
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    printInfo('Contract name', contractName);

    const contractJson = getContractJSON(contractName === 'MonadAxelarTransceiver' ? 'AxelarTransceiver' : contractName, artifactPath);
    const constructorArgs = await getConstructorArgs(contractName, contracts, contractConfig, wallet, options);

    const predeployCodehash = await getBytecodeHash(contractJson, chain.axelarId);
    printInfo('Pre-deploy Contract bytecode hash', predeployCodehash);
    const gasOptions = await getGasOptions(chain, options, contractName);

    // Handle upgrade for MonadAxelarTransceiver
    if (upgrade && contractName === 'MonadAxelarTransceiver') {
        await upgradeMonadAxelarTransceiver(contractConfig, contractJson.abi, wallet, chain, options, gasOptions);
        return;
    }

    printInfo(`Constructor args for chain ${chain.name}`, constructorArgs);

    const { deployerContract, salt } = getDeployOptions(deployMethod, options.salt || contractName, chain);

    const predictedAddress = await getDeployedAddress(wallet.address, deployMethod, {
        salt,
        deployerContract,
        contractJson: contractJson,
        constructorArgs,
        provider: wallet.provider,
    });

    if ((await isContract(predictedAddress, provider)) && !reuseProxy) {
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

    for (const chainConfig of Object.values(chains)) {
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

    // Update configuration
    if (contractName === 'ERC1967Proxy') {
        const targetContract = options.forContract;
        if (targetContract && contractConfig) {
            contractConfig.address = contract.address;
            if (constructorArgs[0] !== contractConfig.implementation) {
                printWarn(
                    `Proxy deployed with implementation ${constructorArgs[0]} but contract config has implementation ${contractConfig.implementation}`,
                );
            }
        }
    } else if (contractName === 'MonadAxelarTransceiver') {
        contractConfig.implementation = contract.address;
        contractConfig.gateway = await contract.gateway();
        contractConfig.gasService = await contract.gasService();
        contractConfig.gmpManager = await contract.nttManager();
    } else {
        contractConfig.address = contract.address;
        contractConfig.predeployCodehash = predeployCodehash;
    }

    // Common fields for all contracts
    contractConfig.deployer = wallet.address;
    contractConfig.deploymentMethod = deployMethod;
    contractConfig.codehash = codehash;
    if (deployMethod !== 'create') {
        contractConfig.salt = salt;
    }

    printInfo(
        `${chain.name} | ${contractName}`,
        contractName === 'MonadAxelarTransceiver' ? contractConfig.implementation : contractConfig.address,
    );

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
        upgrade: true,
    });

    program.addOption(
        new Option('-m, --deployMethod <deployMethod>', 'deployment method').choices(['create', 'create2', 'create3']).default('create2'),
    );
    program.addOption(new Option('--args <args>', 'custom deployment args'));
    program.addOption(new Option('--forContract <forContract>', 'specify which contract this proxy is for (e.g., MonadAxelarTransceiver)'));
    program.addOption(new Option('--proxyData <data>', 'specify initialization data for proxy (defaults to "0x" if not provided)'));
    program.addOption(new Option('--gmpManager <address>', 'specify the GMP manager address for MonadAxelarTransceiver deployment'));
    program.addOption(new Option('--reuseProxy', 'reuse existing proxy contract (useful for upgrade deployments)'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}

module.exports = { processCommand, getConstructorArgs };
