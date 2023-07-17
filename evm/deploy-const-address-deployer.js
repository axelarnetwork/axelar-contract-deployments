'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const { Wallet, ContractFactory, getDefaultProvider } = ethers;
const readlineSync = require('readline-sync');
const { Command, Option } = require('commander');
const chalk = require('chalk');

const { printInfo, writeJSON, predictAddressCreate, deployContract } = require('./utils');
const contractJson = require('@axelar-network/axelar-gmp-sdk-solidity/dist/ConstAddressDeployer.json');
const contractName = 'ConstAddressDeployer';

async function deployConstAddressDeployer(wallet, chain, privateKey, verifyOptions) {
    const deployerWallet = new Wallet(privateKey, wallet.provider);

    printInfo('Deployer address', wallet.address);

    const nonce = await wallet.provider.getTransactionCount(wallet.address);

    if (nonce !== 0) {
        throw new Error(`Nonce value must be zero.`);
    }

    const balance = await wallet.provider.getBalance(deployerWallet.address);
    console.log(`Deployer has ${balance / 1e18} ${chalk.green(chain.tokenSymbol)} and nonce ${nonce} on ${chain.name}.`);

    const contracts = chain.contracts;

    if (!contracts[contractName]) {
        contracts[contractName] = {};
    }

    const contractConfig = contracts[contractName];
    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    console.log(`Gas override for chain ${chain.name}: ${JSON.stringify(gasOptions)}`);

    const constAddressDeployerAddress = await predictAddressCreate(deployerWallet.address, 0);
    printInfo('ConstAddressDeployer will be deployed to', constAddressDeployerAddress);

    console.log('Does this match any existing deployments?');
    const anwser = readlineSync.question(`Proceed with deployment on ${chain.name}? ${chalk.green('(y/n)')} `);
    if (anwser !== 'y') return;

    if (!gasOptions.gasLimit) {
        const contractFactory = new ContractFactory(contractJson.abi, contractJson.bytecode, wallet);
        const tx = contractFactory.getDeployTransaction();
        gasOptions.gasLimit = Math.floor((await wallet.provider.estimateGas(tx)) * 1.5);
    }

    if (!gasOptions.gasPrice) {
        gasOptions.gasPrice = Math.floor((await wallet.provider.getGasPrice()) * 1.2);
    }

    const requiredBalance = gasOptions.gasLimit * gasOptions.gasPrice;

    if (balance < requiredBalance) {
        await (await wallet.sendTransaction({ to: deployerWallet.address, value: requiredBalance - balance })).wait();
    }

    const contract = await deployContract(deployerWallet, contractJson, [], gasOptions, verifyOptions);

    contractConfig.address = contract.address;
    contractConfig.deployer = deployerWallet.address;

    printInfo(`${chain.name} | ConstAddressDeployer`, contractConfig.address);

    return constAddressDeployerAddress;
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

        let wallet;

        if (options.env === 'local') {
            const [funder] = await ethers.getSigners();
            wallet = new Wallet(options.privateKey, funder.provider);
            await (await funder.sendTransaction({ to: wallet.address, value: BigInt(1e21) })).wait();
        } else {
            const provider = getDefaultProvider(chain.rpc);
            wallet = new Wallet(options.privateKey, provider);
        }

        const verifyOptions = options.verify ? { env: options.env, chain: chain.name } : null;
        await deployConstAddressDeployer(wallet, config.chains[chainName.toLowerCase()], options.privateKey, verifyOptions);
        writeJSON(config, `${__dirname}/../info/${options.env}.json`);
    }
}

if (require.main === module) {
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
    program.addOption(new Option('-v, --verify', 'verify the deployed contract on the explorer').env('VERIFY'));

    program.action((options) => {
        main(options);
    });

    program.parse();
} else {
    module.exports = {
        deployConstAddressDeployer,
    };
}
