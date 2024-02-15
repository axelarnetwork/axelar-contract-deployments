'use strict';

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
    isNumberArray,
} = require('./utils');
const { addExtendedOptions } = require('./cli-utils');

async function getConstructorArgs(contractName, chain, wallet) {
    const config = chain.contracts;
    const contractConfig = config[contractName];

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

            const cosigners = contractConfig.cosigners;

            if (!isAddressArray(cosigners)) {
                throw new Error(`Missing AxelarServiceGovernance.cosigners in the chain info.`);
            }

            const threshold = contractConfig.threshold;

            if (!isNumber(threshold)) {
                throw new Error(`Missing AxelarServiceGovernance.threshold in the chain info.`);
            }

            return [gateway, governanceChain, governanceAddress, minimumTimeDelay, cosigners, threshold];
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
            contractConfig.minimumTimeDelay = minimumTimeDelay;

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

        case 'InterchainMultisig': {
            const chainName = chain.axelarId;

            const signers = contractConfig.signers;

            if (!isAddressArray(signers)) {
                throw new Error(`Missing InterchainMultisig.signers in the chain info.`);
            }

            const weights = contractConfig.weights;

            if (!isNumberArray(weights)) {
                throw new Error(`Missing InterchainMultisig.weights in the chain info.`);
            }

            const threshold = contractConfig.threshold;

            if (!isNumber(threshold)) {
                throw new Error(`Missing InterchainMultisig.threshold in the chain info.`);
            }

            return [chainName, [signers, weights, threshold]];
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
    }
}

async function processCommand(config, chain, options) {
    const { env, artifactPath, contractName, deployMethod, privateKey, verify, yes, predictOnly } = options;
    const verifyOptions = verify ? { env, chain: chain.name, only: verify === 'only' } : null;

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

    const predeployCodehash = await getBytecodeHash(contractJson, chain.axelarId);
    printInfo('Pre-deploy Contract bytecode hash', predeployCodehash);

    const constructorArgs = await getConstructorArgs(contractName, chain, wallet, options);
    const gasOptions = await getGasOptions(chain, options, contractName);

    printInfo(`Constructor args for chain ${chain.name}`, constructorArgs);

    const salt = options.salt || contractName;
    let deployerContract = deployMethod === 'create3' ? contracts.Create3Deployer?.address : contracts.ConstAddressDeployer?.address;

    if (deployMethod === 'create') {
        deployerContract = null;
    }

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

    const existingAddress = config.chains.ethereum?.contracts?.[contractName]?.address;

    if (existingAddress !== undefined && predictedAddress !== existingAddress) {
        printWarn(
            `Predicted address ${predictedAddress} does not match existing deployment ${existingAddress} on chain ${config.chains.ethereum.name}.`,
        );

        const existingCodeHash = config.chains.ethereum.contracts[contractName].predeployCodehash;

        if (predeployCodehash !== existingCodeHash) {
            printWarn(
                `Pre-deploy bytecode hash ${predeployCodehash} does not match existing deployment's predeployCodehash ${existingCodeHash} on chain ${config.chains.ethereum.name}.`,
            );
        }

        printWarn('For official deployment, recheck the deployer, salt, args, or contract bytecode.');
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

    if (deployMethod !== 'create') {
        contractConfig.salt = salt;
    }

    saveConfig(config, options.env);

    printInfo(`${chain.name} | ${contractName}`, contractConfig.address);

    await checkContract(contractName, contract, contractConfig);
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-contract').description('Deploy contracts using create, create2, or create3');

    addExtendedOptions(program, {
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

    program.action((options) => {
        main(options);
    });

    program.parse();
}
