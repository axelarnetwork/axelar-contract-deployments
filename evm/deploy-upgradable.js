'use strict';

require('dotenv').config();

const { get, getOr, isEmpty } = require('lodash/fp');
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

const { deployCreate2Upgradable, upgradeUpgradable } = require('./upgradable');
const { writeJSON } = require('./utils');

function getProxy(wallet, proxyAddress) {
    return new Contract(proxyAddress, IUpgradable.abi, wallet);
}

async function getImplementationArgs(contractName, config) {
    if (contractName === 'AxelarGasService') {
        const collector = get('AxelarGasService.collector', config);
        if (!isAddress(collector)) throw new Error(`Missing AxelarGasService.collector in the chain info.`);
        return [collector];
    }

    if (contractName === 'AxelarDepositService') {
        const symbol = getOr('', 'AxelarDepositService.wrappedSymbol', config);
        if (isEmpty(symbol)) console.log(`${config.name} | AxelarDepositService.wrappedSymbol: wrapped token is disabled`);

        const refundIssuer = get('AxelarDepositService.refundIssuer', config);
        if (!isAddress(refundIssuer)) throw new Error(`${config.name} | Missing AxelarDepositService.refundIssuer in the chain info.`);

        return [config.gateway, symbol, refundIssuer];
    }

    throw new Error(`${contractName} is not supported.`);
}

function getInitArgs(contractName, config) {
    if (contractName === 'AxelarGasService') return '0x';
    if (contractName === 'AxelarDepositService') return '0x';
    throw new Error(`${contractName} is not supported.`);
}

function getUpgradeArgs(contractName, config) {
    if (contractName === 'AxelarGasService') return '0x';
    if (contractName === 'AxelarDepositService') return '0x';
    throw new Error(`${contractName} is not supported.`);
}

async function deploy(env, wallet, artifactPath, contractName, chain, salt, upgrade) {
    const implementationPath = artifactPath + contractName + '.sol/' + contractName + '.json';
    const proxyPath = artifactPath + contractName + 'Proxy.sol/' + contractName + 'Proxy.json';
    const implementationJson = require(implementationPath);
    const proxyJson = require(proxyPath);
    const shouldVerifyContract = process.env.VERIFY_CONTRACT === 'true';
    console.log(`Deployer address ${wallet.address}`);

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);
    console.log(
        `Deployer has ${(await provider.getBalance(wallet.address)) / 1e18} ${
            chain.tokenSymbol
        } and nonce ${await provider.getTransactionCount(wallet.address)} on ${chain.name}.`,
    );

    const contracts = chain.contracts;
    if (!contracts[contractName]) {
        contracts[contractName] = {};
    }
    const contractConfig = contracts[contractName];
    const args = await getImplementationArgs(contractName, contracts);
    console.log(`Implementation args for chain ${chain.name}: ${args}`);
    console.log(`Gas override for chain ${chain.name}:`, chain.gasOptions);

    if (upgrade) {
        if (!contractConfig.address) {
            throw new Error(`${chain.name} | Contract ${contractName} is not deployed.`);
        }

        const contract = getProxy(wallet.connect(provider), contractConfig.address);
        const owner = await contract.owner();
        console.log(`Upgrading proxy on ${chain.name}: ${contract.address}`);
        console.log(`Existing implementation ${await contract.implementation()}`);
        console.log(`Existing owner ${owner}`);

        if (wallet.address !== owner) {
            throw new Error(
                `${chain.name} | Signer ${wallet.address} does not match contract owner ${owner} for chain ${chain.name} in info.`,
            );
        }

        const anwser = readlineSync.question(`Perform an upgrade for ${chain.name}? (y/n) `);
        if (anwser !== 'y') return;

        await upgradeUpgradable(
            wallet.connect(provider),
            contractConfig.address,
            implementationJson,
            args,
            getUpgradeArgs(contractName, chain),
            get('gasOptions.gasLimit', chain),
            env,
            chain.name,
            shouldVerifyContract,
        );

        contractConfig.implementation = await contract.implementation();

        console.log(`${chain.name} | New Implementation for ${contractName} is at ${contractConfig.implementation}`);
        console.log(`${chain.name} | Upgraded.`);
    } else {
        salt = salt || contractName;
        const setupArgs = getInitArgs(contractName, contracts);
        console.log(`Proxy setup args: ${setupArgs}`);
        console.log(`Proxy deployment salt: '${salt}'`);

        const constAddressDeployer = contracts.ConstAddressDeployer.address;
        const proxyAddress = await predictContractConstant(constAddressDeployer, wallet.connect(provider), proxyJson, salt);
        console.log(`Proxy will be deployed to ${proxyAddress}. Does this match any existing deployments?`);
        const anwser = readlineSync.question(`Proceed with deployment on ${chain.name}? (y/n) `);
        if (anwser !== 'y') return;

        const contract = await deployCreate2Upgradable(
            constAddressDeployer,
            wallet.connect(provider),
            implementationJson,
            proxyJson,
            args,
            [],
            setupArgs,
            salt,
            get('gasOptions.gasLimit', chain),
            env,
            chain.name,
            shouldVerifyContract,
        );

        contractConfig.salt = salt;
        contractConfig.address = contract.address;
        contractConfig.implementation = await contract.implementation();
        contractConfig.deployer = wallet.address;

        console.log(`${chain.name} | ConstAddressDeployer is at ${constAddressDeployer}`);
        console.log(`${chain.name} | Implementation for ${contractName} is at ${contractConfig.implementation}`);
        console.log(`${chain.name} | Proxy for ${contractName} is at ${contractConfig.address}`);
    }
}

async function main(options) {
    const config = require(`${__dirname}/../info/${options.env}.json`);

    const wallet = new Wallet(options.privateKey);
    const chains = options.chainNames.split(',');

    for (const chain of chains) {
        if (config.chains[chain.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chain} is not defined in the info file`);
        }
    }

    for (const chain of chains) {
        await deploy(
            options.env,
            wallet,
            options.artifactPath,
            options.contractName,
            config.chains[chain.toLowerCase()],
            options.salt,
            options.upgrade,
        );
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

program.action((options) => {
    main(options);
});

program.parse();
