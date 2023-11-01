'use strict';

require('dotenv').config();

const { ethers } = require('hardhat');
const { Wallet, getDefaultProvider, ContractFactory } = ethers;
const readlineSync = require('readline-sync');
const { Command, Option } = require('commander');
const chalk = require('chalk');

const { printInfo, writeJSON, predictAddressCreate, deployCreate } = require('./utils');
const contractJson = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/deploy/ConstAddressDeployer.sol/ConstAddressDeployer.json');
const contractName = 'ConstAddressDeployer';

async function deployConstAddressDeployer(wallet, chain, options = null, verifyOptions = null) {
    printInfo('Deployer address', wallet.address);

    const contracts = chain.contracts;

    if (!contracts[contractName]) {
        contracts[contractName] = {};
    }

    const contractConfig = contracts[contractName];

    const provider = wallet.provider;
    const expectedAddress = contractConfig.address ? contractConfig.address : await predictAddressCreate(wallet.address, 0);

    if (!options.force && (await provider.getCode(expectedAddress)) !== '0x') {
        console.log(`ConstAddressDeployer already deployed at address ${expectedAddress}`);
        contractConfig.address = expectedAddress;
        contractConfig.deployer = wallet.address;
        return;
    }

    const nonce = await provider.getTransactionCount(wallet.address);

    if (nonce !== 0 && !options.ignore) {
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

    if (!options.yes) {
        console.log('Does this match any existing deployments?');
        const anwser = readlineSync.question(`Proceed with deployment on ${chain.name}? ${chalk.green('(y/n)')} `);
        if (anwser !== 'y') return;
    }

    if (!gasOptions.gasLimit) {
        const contractFactory = new ContractFactory(contractJson.abi, contractJson.bytecode, wallet);
        const tx = contractFactory.getDeployTransaction();
        gasOptions.gasLimit = Math.floor((await wallet.provider.estimateGas(tx)) * 1.5);
    }

    if (!gasOptions.gasPrice) {
        gasOptions.gasPrice = Math.floor((await wallet.provider.getGasPrice()) * 1.2);
    }

    const requiredBalance = gasOptions.gasLimit * gasOptions.gasPrice;

    if (!options.ignore && balance < requiredBalance) {
        await (await wallet.sendTransaction({ to: wallet.address, value: requiredBalance - balance })).wait();
    }

    const contract = await deployCreate(wallet, contractJson, [], gasOptions, verifyOptions);

    contractConfig.address = contract.address;
    contractConfig.deployer = wallet.address;

    printInfo(`${chain.name} | ConstAddressDeployer:`, contractConfig.address);
}

async function main(options) {
    const config = require(`${__dirname}/../axelar-chains-config/info/${options.env === 'local' ? 'testnet' : options.env}.json`);

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
        await deployConstAddressDeployer(
            wallet,
            config.chains[chainName.toLowerCase()],
            { yes: options.yes, force: options.force, ignore: options.ignore },
            verifyOptions,
        );
        writeJSON(config, `${__dirname}/../axelar-chains-config/info/${options.env}.json`);
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-const-address-deployer').description('Deploy const address deployer');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
    program.addOption(new Option('-i, --ignore', 'ignore the nonce value check'));
    program.addOption(new Option('-v, --verify', 'verify the deployed contract on the explorer').env('VERIFY'));
    program.addOption(new Option('-f, --force', 'proceed with contract deployment even if address already returns a bytecode'));
    program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));

    program.action((options) => {
        main(options);
    });

    program.parse();
} else {
    module.exports = {
        deployConstAddressDeployer,
    };
}
