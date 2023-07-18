'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const { Wallet, getDefaultProvider } = ethers;
const readlineSync = require('readline-sync');
const { predictContractConstant } = require('@axelar-network/axelar-gmp-sdk-solidity');
const { Command, Option } = require('commander');
const chalk = require('chalk');

const { printInfo, writeJSON, deployCreate2 } = require('./utils');
const implementationJson = require('@axelar-network/axelar-gmp-sdk-solidity/dist/Create3Deployer.json');
const { deployConstAddressDeployer } = require('./deploy-const-address-deployer');
const { keccak256 } = require('ethers/lib/utils');
const contractName = 'Create3Deployer';

async function deployCreate3Deployer(wallet, chain, salt = null, verifyOptions = null) {
    printInfo('Deployer address', wallet.address);

    console.log(
        `Deployer has ${(await wallet.provider.getBalance(wallet.address)) / 1e18} ${chalk.green(
            chain.tokenSymbol,
        )} and nonce ${await wallet.provider.getTransactionCount(wallet.address)} on ${chain.name}.`,
    );

    const contracts = chain.contracts;

    if (!contracts[contractName]) {
        contracts[contractName] = {};
    }

    const contractConfig = contracts[contractName];
    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    console.log(`Gas override for chain ${chain.name}: ${JSON.stringify(gasOptions)}`);

    salt = salt || contractName;
    printInfo('Create3 deployer deployment salt', salt);

    const constAddressDeployer = contracts.ConstAddressDeployer.address;

    const create3DeployerAddress = await predictContractConstant(constAddressDeployer, wallet, implementationJson, salt);
    printInfo('Create3 deployer will be deployed to', create3DeployerAddress);

    console.log('Does this match any existing deployments?');
    const anwser = readlineSync.question(`Proceed with deployment on ${chain.name}? ${chalk.green('(y/n)')} `);
    if (anwser !== 'y') return;

    const contract = await deployCreate2(constAddressDeployer, wallet, implementationJson, [], salt, gasOptions.gasLimit, verifyOptions);

    contractConfig.salt = salt;
    contractConfig.address = contract.address;
    contractConfig.deployer = wallet.address;

    printInfo(`${chain.name} | ConstAddressDeployer:`, constAddressDeployer);
    printInfo(`${chain.name} | Create3Deployer`, contractConfig.address);
}

async function main(options) {
    const config = require(`${__dirname}/../info/${options.env === 'local' ? 'testnet' : options.env}.json`);

    const chains = options.chainNames.split(',').map((str) => str.trim());

    for (const chainName of chains) {
        if (config.chains[chainName.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chainName} is not defined in the info file`);
        }
    }

    for (const chainName of chains) {
        const chain = config.chains[chainName.toLowerCase()];
        const verifyOptions = options.verify ? { env: options.env, chain: chain.name } : null;

        let wallet;

        if (options.env === 'local') {
            const [funder] = await ethers.getSigners();
            wallet = new Wallet(options.privateKey, funder.provider);
            await (await funder.sendTransaction({ to: wallet.address, value: BigInt(1e21) })).wait();
            await deployConstAddressDeployer(wallet, config.chains[chains[0].toLowerCase()], keccak256('0x9123'));
        } else {
            const provider = getDefaultProvider(chain.rpc);
            wallet = new Wallet(options.privateKey, provider);
        }

        await deployCreate3Deployer(wallet, chain, options.salt, verifyOptions);
        writeJSON(config, `${__dirname}/../info/${options.env}.json`);
    }
}

if (require.main === module) {
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
} else {
    module.exports = { deployCreate3Deployer };
}
