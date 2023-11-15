require('dotenv').config();

const { getCreate3Address } = require('@axelar-network/axelar-gmp-sdk-solidity');
const { deployContract, printWalletInfo } = require('./utils');
const { ethers } = require('hardhat');
const {
    Wallet,
    Contract,
    getDefaultProvider,
    utils: { defaultAbiCoder, isAddress, keccak256 },
} = ethers;
const chalk = require('chalk');
const { printInfo, getContractJSON, mainProcessor, prompt } = require('./utils');
const { addExtendedOptions } = require('./cli-utils');
const InterchainTokenService = getContractJSON('InterchainTokenService');
const { Command, Option } = require('commander');

/**
 * Function that handles the ITS deployment.
 * @param {*} wallet
 * @param {*} chain
 * @param {*} deployOptions
 * @param {*} operatorAddress
 * @param {*} skipExisting
 * @param {*} verifyOptions
 */

async function deployImplementation(wallet, chain, options) {
    const { env, salt, factorySalt, deployMethod, skipExisting, verify, yes } = options;
    const verifyOptions = verify ? { env, chain: chain.name, only: verify === 'only' } : null;

    const contractName = 'InterchainTokenService';
    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};

    contractConfig.salt = salt;
    contractConfig.deployer = wallet.address;

    contracts[contractName] = contractConfig;

    const interchainTokenServiceAddress = await getCreate3Address(contracts.Create3Deployer.address, wallet, salt);
    printInfo('Interchain Token Service will be deployed to', interchainTokenServiceAddress);

    const trustedChains = config.chains.map((chain) => chain.id);
    const trustedAddresses = config.chains.map((_) => interchainTokenServiceAddress);

    contracts.InterchainTokenService.interchainTokenFactory = await getCreate3Address(
        contracts.Create3Deployer.address,
        wallet,
        factorySalt,
    );
    printInfo('Interchain Token Factory will be deployed to', contracts.InterchainTokenService.interchainTokenFactory);

    if (prompt(`Does this match any existing deployments? Proceed with deployment on ${chain.name}?`, yes)) {
        return;
    }

    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};

    const deployOptions =
        deployMethod === 'create'
            ? {}
            : {
                  salt,
                  deployerContract: deployMethod === 'create2' ? contracts.ConstAddressDeployer.address : contracts.Create3Deployer.address,
              };

    const deployments = {
        tokenManagerDeployer: {
            name: 'Token Manager Deployer',
            async deploy() {
                return await deployContract(
                    deployMethod,
                    wallet,
                    getContractJSON('TokenManagerDeployer'),
                    [],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                );
            },
        },
        interchainToken: {
            name: 'Interchain Token Lock Unlock',
            async deploy() {
                return await deployContract(
                    deployMethod,
                    wallet,
                    getContractJSON('InterchainToken'),
                    [],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                );
            },
        },
        interchainTokenDeployer: {
            name: 'Interchain Token Deployer',
            async deploy() {
                return await deployContract(
                    deployMethod,
                    wallet,
                    getContractJSON('InterchainTokenDeployer'),
                    [contractConfig.interchainToken],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                );
            },
        },
        tokenManagerImplementations: {
            name: 'Token Manager Implementations',
            async deploy() {
                const implementations = {
                    tokenManagerMintBurn: (
                        await deployContract(
                            deployMethod,
                            wallet,
                            getContractJSON('TokenManagerMintBurn'),
                            [interchainTokenServiceAddress],
                            deployOptions,
                            gasOptions,
                            verifyOptions,
                            chain,
                        )
                    ).address,
                    tokenManagerMintBurnFrom: (
                        await deployContract(
                            deployMethod,
                            wallet,
                            getContractJSON('TokenManagerMintBurnFrom'),
                            [interchainTokenServiceAddress],
                            deployOptions,
                            gasOptions,
                            verifyOptions,
                            chain,
                        )
                    ).address,
                    tokenManagerLockUnlock: (
                        await deployContract(
                            deployMethod,
                            wallet,
                            getContractJSON('TokenManagerLockUnlock'),
                            [interchainTokenServiceAddress],
                            deployOptions,
                            gasOptions,
                            verifyOptions,
                            chain,
                        )
                    ).address,
                    tokenManagerLockUnlockFee: (
                        await deployContract(
                            deployMethod,
                            wallet,
                            getContractJSON('TokenManagerLockUnlockFee'),
                            [interchainTokenServiceAddress],
                            deployOptions,
                            gasOptions,
                            verifyOptions,
                            chain,
                        )
                    ).address,
                };

                return implementations;
            },
        },
        implementation: {
            name: 'Interchain Token Service Implementation',
            async deploy() {
                return await deployContract(
                    deployMethod,
                    wallet,
                    getContractJSON('InterchainTokenService'),
                    [
                        contractConfig.tokenManagerDeployer,
                        contractConfig.interchainTokenDeployer,
                        contracts.AxelarGateway.address,
                        contracts.AxelarGasService.address,
                        contractConfig.interchainTokenFactory,
                        chain.id,
                        Object.values(contractConfig.tokenManagerImplementations),
                    ],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                );
            },
        },
        address: {
            name: 'Interchain Token Service Proxy',
            async deploy() {
                const operatorAddress = options.operatorAddress || wallet.address;

                const deploymentParams = defaultAbiCoder.encode(['address', 'string', 'string[]', 'string[]'], [operatorAddress, chain.id, trustedChains, trustedAddresses]);

                return await deployContract(
                    'create3',
                    wallet,
                    getContractJSON('InterchainTokenServiceProxy'),
                    [contractConfig.implementation, wallet.address, deploymentParams],
                    { salt, deployerContract: contracts.Create3Deployer.address },
                    gasOptions,
                    verifyOptions,
                );
            },
        },
        interchainTokenFactoryImplementation: {
            name: 'Interchain Token Factory Implementation',
            async deploy() {
                return await deployContract(
                    deployMethod,
                    wallet,
                    getContractJSON('InterchainTokenFactory'),
                    [
                        interchainTokenServiceAddress,
                    ],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                );
            },
        },
        interchainTokenFactory: {
            name: 'Interchain Token Factory Proxy',
            async deploy() {
                return await deployContract(
                    'create3',
                    wallet,
                    getContractJSON('InterchainTokenFactoryProxy'),
                    [contractConfig.interchainTokenFactoryImplementation, wallet.address],
                    { salt: factorySalt, deployerContract: contracts.Create3Deployer.address },
                    gasOptions,
                    verifyOptions,
                );
            },
        },
    };

    for (const key in deployments) {
        if (skipExisting && contractConfig[key]) continue;

        const deployment = deployments[key];
        printInfo(`Deploying ${deployment.name}.`);

        const contract = await deployment.deploy();

        if (contract.address === undefined) {
            contractConfig[key] = contract;
            printInfo(`Deployed ${deployment.name} at ${JSON.stringify(contract)}`);
        } else {
            contractConfig[key] = contract.address;
            printInfo(`Deployed ${deployment.name} at ${contract.address}`);
        }
    }
}

async function deploy(config, chain, options) {
    const { privateKey, salt } = options;

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey, provider);
    const contractName = 'InterchainTokenService';

    await printWalletInfo(wallet, options);

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};

    contractConfig.salt = salt;
    contractConfig.deployer = wallet.address;

    contracts[contractName] = contractConfig;

    const operatorAddress = options.operatorAddress || wallet.address;

    if (!isAddress(operatorAddress)) {
        throw new Error(`Invalid operator address: ${operatorAddress}`);
    }

    await deployImplementation(wallet, chain, options);
}

async function upgrade(config, chain, options) {
    const { salt, privateKey } = options;

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey, provider);
    const contractName = 'InterchainTokenService';

    await printWalletInfo(wallet, options);

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};

    contractConfig.salt = salt;
    contractConfig.deployer = wallet.address;

    contracts[contractName] = contractConfig;

    await deployImplementation(wallet, chain, options);

    printInfo(`Upgrading Interchain Token Service.`);

    const gasOptions = chain.gasOptions || {};
    const contract = new Contract(contractConfig.address, InterchainTokenService.abi, wallet);

    const codehash = await getBytecodeHash(contractConfig.implementation, chain.id, provider);

    await contract.upgrade(contractConfig.implementation, codehash, '0x', gasOptions).then((tx) => tx.wait(chain.confirmations));

    printInfo(`Upgraded Interchain Token Service`);
}

async function processCommand(config, chain, options) {
    if (!options.upgrade) {
        await deploy(config, chain, options);
    } else {
        await upgrade(config, chain, options);
    }
}

async function main(options) {
    await mainProcessor(options, processCommand);
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-its').description('Deploy interchain token service');

    program.addOption(
        new Option('-m, --deployMethod <deployMethod>', 'deployment method').choices(['create', 'create2', 'create3']).default('create'),
    );
    addExtendedOptions(program, { skipExisting: true, upgrade: true });

    program.addOption(new Option('-s, --salt <key>', 'deployment salt to use for ITS deployment').makeOptionMandatory(true).env('SALT'));
    program.addOption(
        new Option('-f, --factorySalt <key>', 'deployment salt to use for Interchain Token Factory deployment')
            .makeOptionMandatory(true)
            .env('FACTORY_SALT'),
    );
    program.addOption(new Option('-o, --operatorAddress <operatorAddress>', 'address of the ITS operator/rate limiter').env('OPERATOR_ADDRESS'));

    program.action(async (options) => {
        options.skipExisting = options.skipExisting === 'true';
        await main(options);
    });

    program.parse();
} else {
    module.exports = { deployITS: deploy };
}
