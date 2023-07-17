'use strict';

require('dotenv').config();

const { Wallet, getDefaultProvider } = require('ethers');
const readlineSync = require('readline-sync');
const { Command, Option } = require('commander');
const chalk = require('chalk');

const { printInfo, writeJSON, predictAddressCreate, deployContract } = require('./utils');
const contractJson = require('@axelar-network/axelar-gmp-sdk-solidity/dist/ConstAddressDeployer.json');
const contractName = 'ConstAddressDeployer';

async function deploy(options, chain) {
    const { privateKey, ignore, verify, yes, force } = options;
    const wallet = new Wallet(privateKey);

    printInfo('Deployer address', wallet.address);

    const contracts = chain.contracts;

    if (!contracts[contractName]) {
        contracts[contractName] = {};
    }

    const contractConfig = contracts[contractName];

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);
    const expectedAddress = contractConfig.address ? contracts[contractName].address : await predictAddressCreate(wallet.address, 0);

    if (!force && (await provider.getCode(expectedAddress)) !== '0x') {
        console.log(`ConstAddressDeployer already deployed at address ${expectedAddress}`);
        contractConfig.address = expectedAddress;
        contractConfig.deployer = wallet.address;
        return;
    }

    const nonce = await provider.getTransactionCount(wallet.address);

    if (nonce !== 0 && !ignore) {
        throw new Error(`Nonce value must be zero.`);
    }

    const balance = await provider.getBalance(wallet.address);

    if (balance.lte(0)) {
        throw new Error(`Deployer account has no funds.`);
    }

    console.log(`Deployer has ${balance / 1e18} ${chalk.green(chain.tokenSymbol)} and nonce ${nonce} on ${chain.name}.`);

    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    console.log(`Gas override for chain ${chain.name}: ${JSON.stringify(gasOptions)}`);

    const constAddressDeployerAddress = await predictAddressCreate(wallet.address, nonce);
    printInfo('ConstAddressDeployer will be deployed to', constAddressDeployerAddress);

    if (!yes) {
        console.log('Does this match any existing deployments?');
        const anwser = readlineSync.question(`Proceed with deployment on ${chain.name}? ${chalk.green('(y/n)')} `);
        if (anwser !== 'y') return;
    }

    const contract = await deployContract(wallet.connect(provider), contractJson, [], gasOptions, verify);

    contractConfig.address = contract.address;
    contractConfig.deployer = wallet.address;

    printInfo(`${chain.name} | ConstAddressDeployer:`, contractConfig.address);
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

program.name('deploy-const-address-deployer').description('Deploy const address deployer');

program.addOption(
    new Option('-e, --env <env>', 'environment')
        .choices(['local', 'devnet', 'testnet', 'mainnet'])
        .default('testnet')
        .makeOptionMandatory(true)
        .env('ENV'),
);
program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
program.addOption(new Option('-i, --ignore', 'ignore the nonce value check'));
program.addOption(new Option('-f, --force', 'proceed with contract deployment even if address already returns a bytecode'));
program.addOption(new Option('-v, --verify', 'verify the deployed contract on the explorer').env('VERIFY'));
program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));

program.action((options) => {
    main(options);
});

program.parse();
