'use strict';

require('dotenv').config();

const {
    Contract,
    Wallet,
    getDefaultProvider,
    utils: { isAddress },
} = require('ethers');
const readlineSync = require('readline-sync');
const { predictContractConstant } = require('@axelar-network/axelar-gmp-sdk-solidity');
const IUpgradable = require('@axelar-network/axelar-gmp-sdk-solidity/dist/IUpgradable.json');
const { Command, Option } = require('commander');
const chalk = require('chalk');

const { deployCreate2Upgradable, upgradeUpgradable } = require('./upgradable');
const { printInfo, writeJSON } = require('./utils');

function getProxy(wallet, proxyAddress) {
    return new Contract(proxyAddress, IUpgradable.abi, wallet);
}

async function getImplementationArgs(contractName, config) {
    const contractConfig = config[contractName];

    switch (contractName) {
        case 'AxelarGasService': {
            const collector = contractConfig.collector;

            if (!isAddress(collector)) {
                throw new Error(`Missing AxelarGasService.collector in the chain info.`);
            }

            return [collector];
        }

        case 'AxelarDepositService': {
            const symbol = contractConfig.wrappedSymbol;

            if (symbol === undefined) {
                throw new Error(`Missing AxelarDepositService.wrappedSymbol in the chain info.`);
            } else if (symbol === '') {
                console.log(`${config.name} | AxelarDepositService.wrappedSymbol: wrapped token is disabled`);
            }

            const refundIssuer = contractConfig.refundIssuer;

            if (!isAddress(refundIssuer)) {
                throw new Error(`${config.name} | Missing AxelarDepositService.refundIssuer in the chain info.`);
            }

            const gateway = config.AxelarGateway?.address;

            if (!isAddress(gateway)) {
                throw new Error(`Missing AxelarGateway address in the chain info.`);
            }

            return [gateway, symbol, refundIssuer];
        }
    }

    throw new Error(`${contractName} is not supported.`);
}

function getInitArgs(contractName, config) {
    switch (contractName) {
        case 'AxelarGasService': {
            return '0x';
        }

        case 'AxelarDepositService': {
            return '0x';
        }
    }

    throw new Error(`${contractName} is not supported.`);
}

function getUpgradeArgs(contractName, config) {
    switch (contractName) {
        case 'AxelarGasService': {
            return '0x';
        }

        case 'AxelarDepositService': {
            return '0x';
        }
    }

    throw new Error(`${contractName} is not supported.`);
}

/*
 * Deploy or upgrade an upgradable contract that's based on the init proxy pattern.
 */
async function deploy(options, chain) {
    const { artifactPath, contractName, privateKey, upgrade, verifyEnv, yes } = options;
    const verifyOptions = verifyEnv ? { env: verifyEnv, chain: chain.name } : null;
    const wallet = new Wallet(privateKey);

    const implementationPath = artifactPath + contractName + '.sol/' + contractName + '.json';
    const proxyPath = artifactPath + contractName + 'Proxy.sol/' + contractName + 'Proxy.json';
    const implementationJson = require(implementationPath);
    const proxyJson = require(proxyPath);
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
    const implArgs = await getImplementationArgs(contractName, contracts);
    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    printInfo(`Implementation args for chain ${chain.name}`, implArgs);
    console.log(`Gas override for chain ${chain.name}: ${JSON.stringify(gasOptions)}`);

    if (upgrade) {
        if (!contractConfig.address) {
            throw new Error(`${chain.name} | Contract ${contractName} is not deployed.`);
        }

        const contract = getProxy(wallet.connect(provider), contractConfig.address);
        const owner = await contract.owner();
        printInfo(`Upgrading proxy on ${chain.name}`, contract.address);
        printInfo('Existing implementation', await contract.implementation());
        printInfo('Existing owner', owner);

        if (wallet.address !== owner) {
            throw new Error(
                `${chain.name} | Signer ${wallet.address} does not match contract owner ${owner} for chain ${chain.name} in info.`,
            );
        }

        if (!yes) {
            const anwser = readlineSync.question(`Perform an upgrade for ${chain.name}? ${chalk.green('(y/n)')} `);
            if (anwser !== 'y') return;
        }

        await upgradeUpgradable(
            contractConfig.address,
            wallet.connect(provider),
            implementationJson,
            implArgs,
            {},
            getUpgradeArgs(contractName, chain),
            verifyOptions,
        );

        contractConfig.implementation = await contract.implementation();

        console.log(`${chain.name} | New Implementation for ${contractName} is at ${contractConfig.implementation}`);
        console.log(`${chain.name} | Upgraded.`);
    } else {
        const salt = options.salt || contractName;
        const setupArgs = getInitArgs(contractName, contracts);
        printInfo('Proxy setup args', setupArgs);
        printInfo('Proxy deployment salt', salt);

        const constAddressDeployer = contracts.ConstAddressDeployer.address;
        const proxyAddress = await predictContractConstant(constAddressDeployer, wallet.connect(provider), proxyJson, salt);
        printInfo('Proxy will be deployed to', proxyAddress);

        if (!yes) {
            console.log('Does this match any existing deployments?');
            const anwser = readlineSync.question(`Proceed with deployment on ${chain.name}? ${chalk.green('(y/n)')} `);
            if (anwser !== 'y') return;
        }

        const contract = await deployCreate2Upgradable(
            constAddressDeployer,
            wallet.connect(provider),
            implementationJson,
            proxyJson,
            implArgs,
            [],
            setupArgs,
            salt,
            gasOptions,
            verifyOptions,
        );

        contractConfig.salt = salt;
        contractConfig.address = contract.address;
        contractConfig.implementation = await contract.implementation();
        contractConfig.deployer = wallet.address;

        printInfo(`${chain.name} | ConstAddressDeployer:`, constAddressDeployer);
        printInfo(`${chain.name} | Implementation for ${contractName}`, contractConfig.implementation);
        printInfo(`${chain.name} | Proxy for ${contractName}`, contractConfig.address);
    }
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

program.name('deploy-upgradable').description('Deploy upgradable contracts');

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
program.addOption(new Option('-u, --upgrade', 'upgrade a deployed contract'));
program.addOption(new Option('-v, --verify', 'verify the deployed contract on the explorer').env('VERIFY'));
program.addOption(new Option('-y, --yes', 'skip deployment prompt confirmation').env('YES'));

program.action((options) => {
    main(options);
});

program.parse();
