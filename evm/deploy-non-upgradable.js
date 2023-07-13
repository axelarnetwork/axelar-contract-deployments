'use strict';

require('dotenv').config();

const {
    Wallet,
    getDefaultProvider,
    utils: { isAddress },
} = require('ethers');
const readlineSync = require('readline-sync');
const { getCreate3Address } = require('@axelar-network/axelar-gmp-sdk-solidity');
const { Command, Option } = require('commander');
const chalk = require('chalk');

const { printInfo, writeJSON, isString, isNumber, isAddressArray, deployCreate3 } = require('./utils');

async function getConstructorArgs(contractName, config) {
    const contractConfig = config[contractName];

    switch (contractName) {
        case 'InterchainGovernance': {
            const gateway = config.AxelarGateway?.address;

            if (!isAddress(gateway)) {
                throw new Error(`Missing AxelarGateway address in the chain info.`);
            }

            const governanceChain = contractConfig.governanceChain;

            if (!isString(governanceChain)) {
                throw new Error(`Missing InterchainGovernance.governanceChain in the chain info.`);
            }

            const governanceAddress = contractConfig.governanceAddress;

            if (!isAddress(governanceAddress)) {
                throw new Error(`Missing InterchainGovernance.governanceAddress in the chain info.`);
            }

            const minimumTimeDelay = contractConfig.minimumTimeDelay;

            if (!isNumber(minimumTimeDelay)) {
                throw new Error(`Missing InterchainGovernance.minimumTimeDelay in the chain info.`);
            }

            return [gateway, governanceChain, governanceAddress, minimumTimeDelay];
        }

        case 'AxelarMultisigMintLimiter': {
            const signers = contractConfig.signers;

            if (!isAddressArray(signers)) {
                throw new Error(`Missing AxelarMultisigMintLimiter.signers in the chain info.`);
            }

            const threshold = contractConfig.threshold;

            if (!isNumber(threshold)) {
                throw new Error(`Missing AxelarMultisigMintLimiter.threshold in the chain info.`);
            }

            return [signers, threshold];
        }

        case 'Operators': {
            return [];
        }
    }

    throw new Error(`${contractName} is not supported.`);
}

/*
 * Deploy a non-upgradable smart contract using the create3 deployment method.
 */
async function deploy(options, chain) {
    const { artifactPath, contractName, privateKey, verifyEnv } = options;
    const verifyOptions = verifyEnv ? { env: verifyEnv, chain: chain.name } : null;

    const wallet = new Wallet(privateKey);

    const implementationPath = artifactPath + contractName + '.sol/' + contractName + '.json';
    const implementationJson = require(implementationPath);
    printInfo('Deployer address', wallet.address);

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    console.log(
        `Deployer has ${(await provider.getBalance(wallet.address)) / 1e18} ${chalk.green(
            chain.tokenSymbol,
        )} and nonce ${await provider.getTransactionCount(wallet.address)} on ${chain.name}.`,
    );

    const contracts = chain.contracts;

    if (!contracts[contractName]) {
        contracts[contractName] = {};
    }

    const contractConfig = contracts[contractName];
    const constructorArgs = await getConstructorArgs(contractName, contracts);
    const gasOptions = contractConfig.gasOptions || chain.gasOptions || null;
    printInfo(`Constructor args for chain ${chain.name}`, constructorArgs);
    console.log(`Gas override for chain ${chain.name}: ${JSON.stringify(gasOptions)}`);

    const salt = options.salt || contractName;
    printInfo('Contract deployment salt', salt);

    let create3Deployer;

    if (contracts.Create3Deployer && isAddress(contracts.Create3Deployer.address)) {
        create3Deployer = contracts.Create3Deployer.address;
    } else {
        throw new Error(`Create3 deployer does not exist on ${chain.name}.`);
    }

    const contractAddress = await getCreate3Address(create3Deployer, wallet.connect(provider), salt);
    printInfo(`${contractName} will be deployed to`, contractAddress);

    console.log('Does this match any existing deployments?');
    const anwser = readlineSync.question(`Proceed with deployment on ${chain.name}? ${chalk.green('(y/n)')} `);
    if (anwser !== 'y') return;

    const contract = await deployCreate3(
        create3Deployer,
        wallet.connect(provider),
        implementationJson,
        constructorArgs,
        salt,
        gasOptions,
        verifyOptions,
    );

    contractConfig.salt = salt;
    contractConfig.address = contract.address;
    contractConfig.deployer = wallet.address;

    printInfo(`${chain.name} | Create3Deployer:`, create3Deployer);
    printInfo(`${chain.name} | ${contractName}`, contractConfig.address);
}

async function main(options) {
    const config = require(`${__dirname}/../info/${options.env}.json`);

    const chains = options.chainNames.split(',');

    for (const chain of chains) {
        if (config.chains[chain.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chain} is not defined in the info file`);
        }
    }

    for (const chain of chains) {
        await deploy(options, config.chains[chain.toLowerCase()]);
        writeJSON(config, `${__dirname}/../info/${options.env}.json`);
    }
}

const program = new Command();

program.name('deploy-non-upgradable').description('Deploy non-upgradable contracts');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(new Option('-a, --artifactPath <artifactPath>', 'artifact path').makeOptionMandatory(true));
program.addOption(new Option('-c, --contractName <contractName>', 'contract name').makeOptionMandatory(true));
program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
program.addOption(new Option('-s, --salt <salt>', 'salt to use for create2 deployment'));
program.addOption(new Option('-v, --verify', 'verify the deployed contract on the explorer').env('VERIFY'));

program.action((options) => {
    main(options);
});

program.parse();
