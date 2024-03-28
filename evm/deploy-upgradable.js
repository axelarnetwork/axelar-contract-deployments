'use strict';

const chalk = require('chalk');
const { ethers } = require('hardhat');
const {
    Contract,
    Wallet,
    getDefaultProvider,
    utils: { isAddress },
} = ethers;
const IUpgradable = require('@axelar-network/axelar-gmp-sdk-solidity/interfaces/IUpgradable.json');
const { Command, Option } = require('commander');

const { deployUpgradable, deployCreate2Upgradable, deployCreate3Upgradable, upgradeUpgradable } = require('./upgradable');
const { printInfo, printError, printWalletInfo, getDeployedAddress, prompt, getGasOptions, mainProcessor } = require('./utils');
const { addExtendedOptions } = require('./cli-utils');

function getProxy(wallet, proxyAddress) {
    return new Contract(proxyAddress, IUpgradable.abi, wallet);
}

async function getImplementationArgs(contractName, config, options) {
    const contractConfig = config[contractName];

    switch (contractName) {
        case 'AxelarGasService': {
            if (options.args) {
                contractConfig.collector = options.args;
            }

            const collector = contractConfig.collector;

            if (!isAddress(collector)) {
                throw new Error(`Missing AxelarGasService.collector ${collector}.`);
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

function getInitArgs(contractName) {
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

function getUpgradeArgs(contractName) {
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
async function processCommand(_, chain, options) {
    const { contractName, deployMethod, privateKey, upgrade, verifyEnv, yes, predictOnly } = options;
    const verifyOptions = verifyEnv ? { env: verifyEnv, chain: chain.name } : null;

    if (deployMethod === 'create3' && (contractName === 'AxelarGasService' || contractName === 'AxelarDepositService')) {
        printError(`${deployMethod} not supported for ${contractName}`);
        return;
    }

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);
    const wallet = new Wallet(privateKey, provider);
    await printWalletInfo(wallet);

    const artifactPath =
        options.artifactPath ||
        '@axelar-network/axelar-cgp-solidity/artifacts/contracts/' +
            (contractName === 'AxelarGasService' ? 'gas-service/' : 'deposit-service/');

    const implementationPath = artifactPath + contractName + '.sol/' + contractName + '.json';
    const proxyPath = artifactPath + contractName + 'Proxy.sol/' + contractName + 'Proxy.json';
    const implementationJson = require(implementationPath);
    const proxyJson = require(proxyPath);

    const contracts = chain.contracts;

    if (!contracts[contractName]) {
        contracts[contractName] = {};
    }

    const contractConfig = contracts[contractName];
    const implArgs = await getImplementationArgs(contractName, contracts, options);
    const gasOptions = await getGasOptions(chain, options, contractName);
    printInfo(`Implementation args for chain ${chain.name}`, implArgs);
    const salt = options.salt || contractName;
    let deployerContract = deployMethod === 'create3' ? contracts.Create3Deployer?.address : contracts.ConstAddressDeployer?.address;

    if (deployMethod === 'create') {
        deployerContract = null;
    }

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

        if (predictOnly || prompt(`Perform an upgrade for ${chain.name}?`, yes)) {
            return;
        }

        await upgradeUpgradable(
            deployMethod,
            contractConfig.address,
            wallet.connect(provider),
            implementationJson,
            implArgs,
            getUpgradeArgs(contractName, chain),
            {
                deployerContract,
                salt: `${salt} Implementation`,
            },
            gasOptions,
            verifyOptions,
            chain.name,
            options,
        );

        contractConfig.implementation = await contract.implementation();

        console.log(`${chain.name} | New Implementation for ${contractName} is at ${contractConfig.implementation}`);
        console.log(`${chain.name} | Upgraded.`);
    } else {
        const setupArgs = getInitArgs(contractName, contracts);
        printInfo('Proxy setup args', setupArgs);

        const predictedAddress = await getDeployedAddress(wallet.address, deployMethod, {
            salt,
            deployerContract,
            contractJson: proxyJson,
            constructorArgs: [],
            provider: wallet.provider,
            nonce: (await wallet.provider.getTransactionCount(wallet.address)) + 1,
        });

        if (deployMethod !== 'create') {
            printInfo(`${contractName} deployment salt`, salt);
        }

        printInfo('Deployment method', deployMethod);
        printInfo('Deployer contract', deployerContract);
        printInfo(`${contractName} will be deployed to`, predictedAddress, chalk.cyan);

        if (predictOnly || prompt(`Does derived address match existing deployments? Proceed with deployment on ${chain.name}?`, yes)) {
            return;
        }

        let contract;

        switch (deployMethod) {
            case 'create': {
                contract = await deployUpgradable(
                    wallet,
                    implementationJson,
                    proxyJson,
                    implArgs,
                    [],
                    setupArgs,
                    gasOptions,
                    verifyOptions,
                );
                break;
            }

            case 'create2': {
                contract = await deployCreate2Upgradable(
                    deployerContract,
                    wallet,
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
                printInfo(`${chain.name} | ConstAddressDeployer`, deployerContract);
                break;
            }

            case 'create3': {
                contract = await deployCreate3Upgradable(
                    deployerContract,
                    wallet,
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
                printInfo(`${chain.name} | Create3Deployer`, deployerContract);
                break;
            }

            default: {
                throw new Error(`Unknown deployment method ${deployMethod}`);
            }
        }

        contractConfig.address = contract.address;
        contractConfig.implementation = await contract.implementation();
        contractConfig.deployer = wallet.address;

        printInfo(`${chain.name} | Implementation for ${contractName}`, contractConfig.implementation);
        printInfo(`${chain.name} | Proxy for ${contractName}`, contractConfig.address);

        const owner = await contract.owner();

        if (owner !== wallet.address) {
            printError(`${chain.name} | Signer ${wallet.address} does not match contract owner ${owner} for chain ${chain.name} in info.`);
        }
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-upgradable').description('Deploy upgradable contracts');

    addExtendedOptions(program, { artifactPath: true, contractName: true, salt: true, skipChains: true, upgrade: true, predictOnly: true });

    program.addOption(
        new Option('-m, --deployMethod <deployMethod>', 'deployment method').choices(['create', 'create2', 'create3']).default('create2'),
    );
    program.addOption(new Option('--args <args>', 'customize deployment args'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
