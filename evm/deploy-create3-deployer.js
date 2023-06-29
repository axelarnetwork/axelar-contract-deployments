'use strict';

require('dotenv').config();

const { Wallet, getDefaultProvider } = require('ethers');
const readlineSync = require('readline-sync');
const { predictContractConstant } = require('@axelar-network/axelar-gmp-sdk-solidity');
const { Command, Option } = require('commander');
const chalk = require('chalk');

const { deployCreate2 } = require('./upgradable');
const { printInfo, writeJSON } = require('./utils');
const implementationJson = require('../artifacts/contracts/deploy/Create3Deployer.sol/Create3Deployer.json');
const contractName = 'Create3Deployer';

async function deploy(options, chain) {
    const { privateKey, verifyEnv } = options;
    const verifyOptions = verifyEnv ? {env: verifyEnv, chain: chain.name} : null;
    const wallet = new Wallet(privateKey);

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
    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    console.log(`Gas override for chain ${chain.name}: ${JSON.stringify(gasOptions)}`);

    const salt = options.salt || contractName;
    printInfo('Create3 deployer deployment salt', salt);

    const constAddressDeployer = contracts.ConstAddressDeployer.address;
    const create3DeployerAddress = await predictContractConstant(constAddressDeployer, wallet.connect(provider), implementationJson, salt);
    printInfo('Create3 deployer will be deployed to', create3DeployerAddress);

    console.log('Does this match any existing deployments?');
    const anwser = readlineSync.question(`Proceed with deployment on ${chain.name}? ${chalk.green('(y/n)')} `);
    if (anwser !== 'y') return;

    const contract = await deployCreate2(
        constAddressDeployer,
        wallet.connect(provider),
        implementationJson,
        salt,
        [],
        gasOptions,
        verifyOptions,
    );

    contractConfig.salt = salt;
    contractConfig.address = contract.address;
    contractConfig.deployer = wallet.address;

    printInfo(`${chain.name} | ConstAddressDeployer:`, constAddressDeployer);
    printInfo(`${chain.name} | Create3Deployer`, contractConfig.address);
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

program.name('deploy-create3-deployer').description('Deploy create3 deployer');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
program.addOption(new Option('-s, --salt <salt>', 'salt to use for create2 deployment'));
program.addOption(new Option('-v, --verify', 'verify the deployed contract on the explorer').env('VERIFY'));

program.action((options) => {
    main(options);
});

program.parse();
