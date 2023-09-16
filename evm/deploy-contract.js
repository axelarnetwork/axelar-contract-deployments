'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const {
    Wallet,
    getDefaultProvider,
    utils: { isAddress },
} = ethers;
const readlineSync = require('readline-sync');
const { Command, Option } = require('commander');
const chalk = require('chalk');

const {
    printInfo,
    printError,
    isString,
    isNumber,
    isAddressArray,
    getBytecodeHash,
    printWalletInfo,
    getDeployedAddress,
    deployContract,
    loadConfig,
    saveConfig,
} = require('./utils');

async function getConstructorArgs(contractName, chain, wallet, options) {
    const config = chain.contracts;
    const contractConfig = config[contractName];

    switch (contractName) {
        case 'AxelarServiceGovernance': {
            const gateway = config.AxelarGateway?.address;

            if (!isAddress(gateway)) {
                throw new Error(`Missing AxelarGateway address in the chain info.`);
            }

            const governanceChain = contractConfig.governanceChain;

            if (!isString(governanceChain)) {
                throw new Error(`Missing AxelarServiceGovernance.governanceChain in the chain info.`);
            }

            const governanceAddress = contractConfig.governanceAddress;

            if (!isString(governanceAddress)) {
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

            if (!isString(governanceChain)) {
                throw new Error(`Missing InterchainGovernance.governanceChain in the chain info.`);
            }

            const governanceAddress = contractConfig.governanceAddress || 'axelar10d07y265gmmuvt4z0w9aw880jnsr700j7v9daj';

            if (!isString(governanceAddress)) {
                throw new Error(`Missing InterchainGovernance.governanceAddress in the chain info.`);
            }

            const minimumTimeDelay = contractConfig.minimumTimeDelay || parseInt(options.args, 10);

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
    }
}

async function deploy(options, chain, config) {
    const { env, artifactPath, contractName, deployMethod, privateKey, verify, yes } = options;
    const verifyOptions = verify ? { env, chain: chain.name, only: verify === 'only' } : null;

    const contracts = chain.contracts;

    if (!contracts[contractName]) {
        contracts[contractName] = {};
    }

    printInfo('Deploying to', chain.name);

    const contractConfig = contracts[contractName];

    if (contractConfig.address && options.skipExisting) {
        printInfo(`Skipping ${contractName} deployment on ${chain.name} because it is already deployed.`);
        return;
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    printInfo('Contract name', contractName);

    const contractPath = artifactPath.charAt(0) === '@' ? artifactPath : artifactPath + contractName + '.sol/' + contractName + '.json';
    printInfo('Contract path', contractPath);

    const contractJson = require(contractPath);

    printInfo('Contract bytecode hash', await getBytecodeHash(contractJson, chain.id));

    const constructorArgs = await getConstructorArgs(contractName, chain, wallet, options);
    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    printInfo(`Constructor args for chain ${chain.name}`, constructorArgs);
    console.log(`Gas override for chain ${chain.name}: ${JSON.stringify(gasOptions)}`);

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

    if (deployMethod !== 'create') {
        printInfo(`${contractName} deployment salt`, salt);
    }

    printInfo('Deployment method', deployMethod);
    printInfo('Deployer contract', deployerContract);
    printInfo(`${contractName} will be deployed to`, predictedAddress);

    if (!yes) {
        console.log('Does this match any existing deployments?');
        const anwser = readlineSync.question(`Proceed with deployment on ${chain.name}? ${chalk.green('(y/n)')} `);
        if (anwser !== 'y') return;
    }

    const contract = await deployContract(
        deployMethod,
        wallet,
        contractJson,
        constructorArgs,
        { salt, deployerContract },
        gasOptions,
        verifyOptions,
    );

    contractConfig.address = contract.address;
    contractConfig.deployer = wallet.address;
    contractConfig.deploymentMethod = deployMethod;

    if (deployMethod !== 'create') {
        contractConfig.salt = salt;
    }

    saveConfig(config, options.env);

    printInfo(`${chain.name} | ${contractName}`, contractConfig.address);

    await checkContract(contractName, contract, contractConfig);
}

async function main(options) {
    const config = loadConfig(options.env);

    let chains = options.chainNames.split(',').map((str) => str.trim());

    if (options.chainNames === 'all') {
        chains = Object.keys(config.chains);
    }

    for (const chain of chains) {
        if (config.chains[chain.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chain} is not defined in the info file`);
        }
    }

    for (const chain of chains) {
        await deploy(options, config.chains[chain.toLowerCase()], config);
    }

    saveConfig(config, options.env);
}

const program = new Command();

program.name('deploy-contract').description('Deploy contracts using create, create2, or create3');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(new Option('-a, --artifactPath <artifactPath>', 'artifact path').makeOptionMandatory(true));
program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
program.addOption(
    new Option('-m, --deployMethod <deployMethod>', 'deployment method').choices(['create', 'create2', 'create3']).default('create2'),
);
program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
program.addOption(new Option('-s, --salt <salt>', 'salt to use for create2 deployment'));
program.addOption(new Option('-v, --verify <verify>', 'verify the deployed contract on the explorer').env('VERIFY'));
program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));
program.addOption(new Option('-x, --skipExisting', 'skip existing if contract was already deployed on chain').env('YES'));

program.action((options) => {
    main(options);
});

program.parse();
