'use strict';

const { ethers } = require('hardhat');
const { Wallet, getDefaultProvider } = ethers;
const readlineSync = require('readline-sync');
const { predictContractConstant } = require('@axelar-network/axelar-gmp-sdk-solidity');
const { Command } = require('commander');
const chalk = require('chalk');

const { printInfo, writeJSON, deployCreate2, getGasOptions } = require('./utils');
const { addExtendedOptions } = require('./cli-utils');
const contractJson = require('@axelar-network/axelar-gmp-sdk-solidity/artifacts/contracts/deploy/Create3Deployer.sol/Create3Deployer.json');
const { deployConstAddressDeployer } = require('./deploy-const-address-deployer');
const contractName = 'Create3Deployer';

async function deployCreate3Deployer(wallet, chain, provider, options = {}, verifyOptions = null) {
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
    const gasOptions = await getGasOptions(chain, options, contractName);

    const salt = options.salt || contractName;
    printInfo('Create3 deployer deployment salt', salt);

    const constAddressDeployer = contracts.ConstAddressDeployer.address;

    const create3DeployerAddress = await predictContractConstant(constAddressDeployer, wallet, contractJson, salt);
    printInfo('Create3 deployer will be deployed to', create3DeployerAddress);

    if (!options.yes) {
        console.log('Does this match any existing deployments?');
        const anwser = readlineSync.question(`Proceed with deployment on ${chain.name}? ${chalk.green('(y/n)')} `);
        if (anwser !== 'y') return;
    }

    const contract = await deployCreate2(constAddressDeployer, wallet, contractJson, [], salt, gasOptions.gasLimit, verifyOptions);

    contractConfig.salt = salt;
    contractConfig.address = contract.address;
    contractConfig.deployer = wallet.address;

    printInfo(`${chain.name} | ConstAddressDeployer:`, constAddressDeployer);
    printInfo(`${chain.name} | Create3Deployer`, contractConfig.address);
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
        const verifyOptions = options.verify ? { env: options.env, chain: chain.name, only: options.verify } : null;

        let wallet;
        let provider;

        if (options.env === 'local') {
            const [funder] = await ethers.getSigners();
            wallet = new Wallet(options.privateKey, funder.provider);
            await (await funder.sendTransaction({ to: wallet.address, value: BigInt(1e21) })).wait();
            await deployConstAddressDeployer(wallet, config.chains[chains[0].toLowerCase()]);
        } else {
            provider = getDefaultProvider(chain.rpc);
            wallet = new Wallet(options.privateKey, provider);
        }

        await deployCreate3Deployer(wallet, chain, provider, { salt: options.salt, yes: options.yes }, verifyOptions);
        writeJSON(config, `${__dirname}/../axelar-chains-config/info/${options.env}.json`);
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-create3-deployer').description('Deploy create3 deployer');

    addExtendedOptions(program, { salt: true });

    program.action((options) => {
        main(options);
    });

    program.parse();
} else {
    module.exports = { deployCreate3Deployer };
}
