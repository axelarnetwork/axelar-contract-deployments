'use strict';

/**
 * @fileoverview EVM Upgradable Contract Deployment Script
 *
 * This script provides functionality to deploy upgradable contracts using the proxy pattern
 * on EVM-compatible chains. It supports multiple deployment methods (create, create2, create3)
 * and handles contract verification, configuration management, and deployment validation.
 *
 * Supported upgradable contract types:
 * - AxelarGasService: Gas service contract with upgrade capability
 * - AxelarDepositService: Deposit service contract with upgrade capability
 */

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
const {
    printInfo,
    printError,
    printWalletInfo,
    getDeployedAddress,
    prompt,
    getGasOptions,
    getDeployOptions,
    mainProcessor,
} = require('./utils');
const { addEvmOptions } = require('./cli-utils');

/**
 * Creates a proxy contract instance for interacting with an upgradable contract.
 */
function getProxy(wallet, proxyAddress) {
    return new Contract(proxyAddress, IUpgradable.abi, wallet);
}

/**
 * Generates implementation constructor arguments for a given contract based on its configuration and options.
 */
async function getImplementationArgs(contractConfig, contractName, gatewayAddress, options) {
    let args;

    try {
        args = options.args ? JSON.parse(options.args) : {};
    } catch (error) {
        printError('Error parsing args:\n', error.message);
    }

    Object.assign(contractConfig, args);

    switch (contractName) {
        case 'AxelarGasService': {
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
                console.log(`AxelarDepositService.wrappedSymbol: wrapped token is disabled`);
            }

            const refundIssuer = contractConfig.refundIssuer;

            if (!isAddress(refundIssuer)) {
                throw new Error(`Missing AxelarDepositService.refundIssuer in the chain info.`);
            }

            if (!isAddress(gatewayAddress)) {
                throw new Error(`Missing AxelarGateway address in the chain info.`);
            }

            return [gatewayAddress, symbol, refundIssuer];
        }
    }

    throw new Error(`${contractName} is not supported.`);
}

/**
 * Generates initialization arguments for proxy setup.
 */
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

/**
 * Generates upgrade arguments for contract upgrades.
 */
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

/**
 * Deploy or upgrade an upgradable contract that's based on the init proxy pattern.
 * This function handles both initial deployment and upgrades of upgradable contracts.
 */
async function processCommand(_axelar, chain, _chains, options) {
    const { contractName, deployMethod, privateKey, upgrade, verifyEnv, yes, predictOnly } = options;
    const verifyOptions = verifyEnv ? { env: verifyEnv, chain: chain.axelarId } : null;

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
    const implArgs = await getImplementationArgs(contractConfig, contractName, contracts.AxelarGateway?.address, options);
    const gasOptions = await getGasOptions(chain, options, contractName);
    printInfo(`Implementation args for chain ${chain.name}`, implArgs);
    const { deployerContract, salt } = getDeployOptions(deployMethod, options.salt || contractName, chain);

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
            getUpgradeArgs(contractName),
            {
                deployerContract,
                salt: `${salt} Implementation`,
            },
            gasOptions,
            verifyOptions,
            chain.axelarId,
            options,
        );

        contractConfig.implementation = await contract.implementation();

        printInfo(`${chain.name} | New Implementation for ${contractName} is at ${contractConfig.implementation}`);
        printInfo(`${chain.name} | Upgraded.`);
    } else {
        const setupArgs = getInitArgs(contractName);
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

        return contract;
    }
}

/**
 * Main entry point for the deploy-upgradable script.
 * Processes deployment options and executes the deployment across specified chains.
 */
async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-upgradable').description('Deploy upgradable contracts');

    addEvmOptions(program, { artifactPath: true, contractName: true, salt: true, skipChains: true, upgrade: true, predictOnly: true });

    program.addOption(
        new Option('-m, --deployMethod <deployMethod>', 'deployment method').choices(['create', 'create2', 'create3']).default('create2'),
    );
    program.addOption(new Option('--args <args>', 'customize deployment args'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
