require('dotenv').config();

const { getCreate3Address } = require('@axelar-network/axelar-gmp-sdk-solidity');
const { ethers } = require('hardhat');
const {
    Wallet,
    Contract,
    getDefaultProvider,
    utils: { defaultAbiCoder, isAddress },
} = ethers;

const {
    deployContract,
    printWalletInfo,
    saveConfig,
    printInfo,
    getContractJSON,
    mainProcessor,
    prompt,
    sleep,
    getBytecodeHash,
    getGasOptions,
} = require('./utils');
const { addExtendedOptions } = require('./cli-utils');
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

async function deployImplementation(config, wallet, chain, options) {
    const { env, artifactPath, salt, factorySalt, deployMethod, skipExisting, verify, yes } = options;
    const verifyOptions = verify ? { env, chain: chain.name, only: verify === 'only' } : null;

    const InterchainTokenService = artifactPath
        ? getContractJSON('InterchainTokenService', artifactPath)
        : getContractJSON('InterchainTokenService');

    const contractName = 'InterchainTokenService';
    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};

    contractConfig.salt = salt;
    contractConfig.deployer = wallet.address;

    contracts[contractName] = contractConfig;

    const interchainTokenServiceAddress = await getCreate3Address(contracts.Create3Deployer.address, wallet, salt);
    printInfo('Interchain Token Service will be deployed to', interchainTokenServiceAddress);

    // Register all chains that ITS is or will be deployed on.
    // Add a "skip": true under ITS key in the config if the chain will not have ITS.
    const itsChains = Object.values(config.chains).filter((chain) => chain.contracts?.InterchainTokenService?.skip !== true);
    const trustedChains = itsChains.map((chain) => chain.id);
    const trustedAddresses = itsChains.map((_) => chain.contracts?.InterchainTokenService?.address || interchainTokenServiceAddress);

    const interchainTokenFactory = await getCreate3Address(contracts.Create3Deployer.address, wallet, factorySalt);
    printInfo('Interchain Token Factory will be deployed to', interchainTokenFactory);

    if (prompt(`Does this match any existing deployments? Proceed with deployment on ${chain.name}?`, yes)) {
        return;
    }

    const gasOptions = await getGasOptions(chain, options, contractName);

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
                    chain,
                );
            },
        },
        interchainToken: {
            name: 'Interchain Token',
            async deploy() {
                return await deployContract(
                    deployMethod,
                    wallet,
                    getContractJSON('InterchainToken'),
                    [interchainTokenServiceAddress],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                    chain,
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
                    chain,
                );
            },
        },
        tokenManager: {
            name: 'Token Manager',
            async deploy() {
                return await deployContract(
                    deployMethod,
                    wallet,
                    getContractJSON('TokenManager'),
                    [interchainTokenServiceAddress],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                    chain,
                );
            },
        },
        tokenHandler: {
            name: 'Token Handler',
            async deploy() {
                return await deployContract(
                    deployMethod,
                    wallet,
                    getContractJSON('TokenHandler'),
                    [],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                    chain,
                );
            },
        },
        implementation: {
            name: 'Interchain Token Service Implementation',
            async deploy() {
                return await deployContract(
                    deployMethod,
                    wallet,
                    InterchainTokenService,
                    [
                        contractConfig.tokenManagerDeployer,
                        contractConfig.interchainTokenDeployer,
                        contracts.AxelarGateway.address,
                        contracts.AxelarGasService.address,
                        interchainTokenFactory,
                        chain.id,
                        contractConfig.tokenManager,
                        contractConfig.tokenHandler,
                    ],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                    chain,
                );
            },
        },
        address: {
            name: 'Interchain Token Service Proxy',
            async deploy() {
                const operatorAddress = options.operatorAddress || wallet.address;

                const deploymentParams = defaultAbiCoder.encode(
                    ['address', 'string', 'string[]', 'string[]'],
                    [operatorAddress, chain.id, trustedChains, trustedAddresses],
                );

                return await deployContract(
                    'create3',
                    wallet,
                    getContractJSON('Proxy'),
                    [contractConfig.implementation, wallet.address, deploymentParams],
                    { salt, deployerContract: contracts.Create3Deployer.address },
                    gasOptions,
                    verifyOptions,
                    chain,
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
                    [interchainTokenServiceAddress],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                    chain,
                );
            },
        },
        interchainTokenFactory: {
            name: 'Interchain Token Factory Proxy',
            async deploy() {
                return await deployContract(
                    'create3',
                    wallet,
                    getContractJSON('Proxy'),
                    [contractConfig.interchainTokenFactoryImplementation, wallet.address, '0x'],
                    { salt: factorySalt, deployerContract: contracts.Create3Deployer.address },
                    gasOptions,
                    verifyOptions,
                    chain,
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

        saveConfig(config, options.env);

        if (chain.chainId !== 31337) {
            await sleep(2000);
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

    await deployImplementation(config, wallet, chain, options);
}

async function upgrade(config, chain, options) {
    const { artifactPath, salt, privateKey } = options;

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

    await deployImplementation(config, wallet, chain, options);

    printInfo(`Upgrading Interchain Token Service.`);

    const InterchainTokenService = artifactPath
        ? getContractJSON('InterchainTokenService', artifactPath)
        : getContractJSON('InterchainTokenService');

    const gasOptions = await getGasOptions(chain, options, contractName);
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

    program.addOption(new Option('--contractName <contractName>', 'contract name').default('InterchainTokenService')); // added for consistency
    program.addOption(new Option('-s, --salt <key>', 'deployment salt to use for ITS deployment').makeOptionMandatory(true).env('SALT'));
    program.addOption(
        new Option('-f, --factorySalt <key>', 'deployment salt to use for Interchain Token Factory deployment')
            .makeOptionMandatory(true)
            .env('FACTORY_SALT'),
    );
    program.addOption(
        new Option('-o, --operatorAddress <operatorAddress>', 'address of the ITS operator/rate limiter').env('OPERATOR_ADDRESS'),
    );

    program.action(async (options) => {
        await main(options);
    });

    program.parse();
} else {
    module.exports = { deployITS: deploy };
}
