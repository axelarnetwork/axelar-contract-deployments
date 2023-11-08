require('dotenv').config();

const { getCreate3Address } = require('@axelar-network/axelar-gmp-sdk-solidity');
const { deployContract } = require('./utils');
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
    const { env, salt, deployMethod, skipExisting, verify, yes } = options;
    const verifyOptions = verify ? { env, chain: chain.name, only: verify === 'only' } : null;

    const contractName = 'InterchainTokenService';
    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};

    contractConfig.salt = salt;
    contractConfig.deployer = wallet.address;

    contracts[contractName] = contractConfig;
    const interchainTokenServiceAddress = await getCreate3Address(contracts.Create3Deployer.address, wallet, salt);
    printInfo('Interchain Token Service will be deployed to', interchainTokenServiceAddress);

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
                        chain.name,
                        Object.values(contractConfig.tokenManagerImplementations),
                    ],
                    deployOptions,
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
    const { env, privateKey, salt, factorySalt, operatorAddress, skipExisting, verify } = options;
    const verifyOptions = verify ? { env, chain: chain.name, only: verify === 'only' } : null;

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey, provider);
    const contractName = 'InterchainTokenService';

    printInfo(
        `Deployer ${wallet.address} has ${(await wallet.provider.getBalance(wallet.address)) / 1e18} ${chalk.green(
            chain.tokenSymbol,
        )} and nonce ${await wallet.provider.getTransactionCount(wallet.address)} on ${chain.name}.`,
    );

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};

    contractConfig.salt = salt;
    contractConfig.deployer = wallet.address;

    contracts[contractName] = contractConfig;

    if (!isAddress(operatorAddress)) {
        throw new Error(`Invalid operator address: ${operatorAddress}`);
    }

    contracts.InterchainTokenService.interchainTokenFactory = await getCreate3Address(
        contracts.Create3Deployer.address,
        wallet,
        factorySalt,
    );

    await deployImplementation(wallet, chain, options);

    if (skipExisting && isAddress(contractConfig.address)) return;

    printInfo(`Deploying Interchain Token Service.`);

    const deploymentParams = defaultAbiCoder.encode(['address', 'string', 'string[]', 'string[]'], [operatorAddress, chain.name, [], []]);
    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    const contract = await deployContract(
        'create3',
        wallet,
        getContractJSON('InterchainTokenServiceProxy'),
        [contractConfig.implementation, wallet.address, deploymentParams],
        { salt, deployerContract: contracts.Create3Deployer.address },
        gasOptions,
        { ...verifyOptions, contractPath: 'contracts/proxies/InterchainTokenServiceProxy.sol:InterchainTokenServiceProxy' },
    );
    contractConfig.address = contract.address;

    printInfo(`Deployed Interchain Token Service at ${contract.address}`);

    printInfo(`Deploying Interchain Token Factory Implementation.`);

    chain.contracts.InterchainTokenService.interchainTokenFactory = null;

    const implementation = await deployContract(
        'create',
        wallet,
        getContractJSON('InterchainTokenFactory'),
        [contractConfig.address],
        {},
        gasOptions,
        {
            ...verifyOptions,
            contractPath: 'contracts/InterchainTokenService.sol:InterchainTokenFactory',
        },
    );

    contractConfig.interchainTokenFactoryImplementation = implementation.address;

    printInfo(`Deployed Interchain Token Factroy Implementations at ${implementation.address}`);

    printInfo(`Deploying Interchain Token Factory Proxy.`);

    const proxy = await deployContract(
        'create3',
        wallet,
        getContractJSON('InterchainTokenFactoryProxy'),
        [implementation.address, wallet.address],
        { salt, deployerContract: contracts.Create3Deployer.address },
        gasOptions,
        { ...verifyOptions, contractPath: 'contracts/proxies/InterchainTokenServiceProxy.sol:InterchainTokenFactoryProxy' },
    );

    printInfo(`Deployed Interchain Token Factroy Proxy at ${proxy.address}`);

    contractConfig.interchainTokenFactory = proxy.address;
}

async function upgrade(config, chain, options) {
    const { salt, privateKey } = options;

    const rpc = chain.rpc;
    const provider = getDefaultProvider(rpc);

    const wallet = new Wallet(privateKey, provider);
    const contractName = 'InterchainTokenService';

    printInfo(
        `Deployer has ${(await wallet.provider.getBalance(wallet.address)) / 1e18} ${chalk.green(
            chain.tokenSymbol,
        )} and nonce ${await wallet.provider.getTransactionCount(wallet.address)} on ${chain.name}.`,
    );

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};

    contractConfig.salt = salt;
    contractConfig.deployer = wallet.address;

    contracts[contractName] = contractConfig;

    await deployImplementation(wallet, chain, options);

    printInfo(`Upgrading Interchain Token Service.`);

    const gasOptions = chain.gasOptions || {};
    const contract = new Contract(contractConfig.address, InterchainTokenService.abi, wallet);

    const codehash = keccak256(await wallet.provider.getCode(contractConfig.implementation));
    await (await contract.upgrade(contractConfig.implementation, codehash, '0x', gasOptions)).wait();

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
        new Option('-m, --deployMethod <deployMethod>', 'deployment method').choices(['create', 'create2', 'create3']).default('create2'),
    );
    addExtendedOptions(program, { skipExisting: true, upgrade: true });

    program.addOption(new Option('-s, --salt <key>', 'deployment salt to use for ITS deployment').makeOptionMandatory(true).env('SALT'));
    program.addOption(
        new Option('-f, --factorySalt <key>', 'deployment salt to use for Interchain Token Factory deployment')
            .makeOptionMandatory(true)
            .env('FACTORY_SALT'),
    );
    program.addOption(new Option('-o, --operatorAddress <operatorAddress>', 'address of the ITS operator').env('OPERATOR_ADDRESS'));

    program.action(async (options) => {
        options.skipExisting = options.skipExisting === 'true';
        await main(options);
    });

    program.parse();
} else {
    module.exports = { deployITS: deploy };
}
