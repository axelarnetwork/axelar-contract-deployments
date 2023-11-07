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
const readlineSync = require('readline-sync');
const chalk = require('chalk');
const { printInfo, loadConfig, saveConfig } = require('./utils');
const { addExtendedOptions } = require('./cli-utils');

const TokenManagerDeployer = require('@axelar-network/interchain-token-service/artifacts/contracts/utils/TokenManagerDeployer.sol/TokenManagerDeployer.json');
const InterchainToken = require('@axelar-network/interchain-token-service/artifacts/contracts/interchain-token/InterchainToken.sol/InterchainToken.json');
const InterchainTokenDeployer = require('@axelar-network/interchain-token-service/artifacts/contracts/utils/InterchainTokenDeployer.sol/InterchainTokenDeployer.json');
const InterchainTokenFactory = require('@axelar-network/interchain-token-service/artifacts/contracts/InterchainTokenFactory.sol/InterchainTokenFactory.json');
const InterchainTokenFactoryProxy = require('@axelar-network/interchain-token-service/artifacts/contracts/proxies/InterchainTokenFactoryProxy.sol/InterchainTokenFactoryProxy.json');
const TokenManagerLockUnlock = require('@axelar-network/interchain-token-service/artifacts/contracts/token-manager/TokenManagerLockUnlock.sol/TokenManagerLockUnlock.json');
const TokenManagerMintBurn = require('@axelar-network/interchain-token-service/artifacts/contracts/token-manager/TokenManagerMintBurn.sol/TokenManagerMintBurn.json');
const TokenManagerMintBurnFrom = require('@axelar-network/interchain-token-service/artifacts/contracts/token-manager/TokenManagerMintBurnFrom.sol/TokenManagerMintBurnFrom.json');
const TokenManagerLockUnlockFee = require('@axelar-network/interchain-token-service/artifacts/contracts/token-manager/TokenManagerLockUnlockFee.sol/TokenManagerLockUnlockFee.json');
const InterchainTokenService = require('@axelar-network/interchain-token-service/artifacts/contracts/InterchainTokenService.sol/InterchainTokenService.json');
const InterchainTokenServiceProxy = require('@axelar-network/interchain-token-service/artifacts/contracts/proxies/InterchainTokenServiceProxy.sol/InterchainTokenServiceProxy.json');
const { Command, Option } = require('commander');
const { deployConstAddressDeployer } = require('./deploy-const-address-deployer');
const { deployCreate3Deployer } = require('./deploy-create3-deployer');
const { deployGatewayv5 } = require('./deploy-gateway-v6.2.x');

/**
 * Function that handles the ITS deployment.
 * @param {*} wallet
 * @param {*} chain
 * @param {*} deployOptions
 * @param {*} operatorAddress
 * @param {*} skipExisting
 * @param {*} verifyOptions
 * @param {*} saveFunc
 */

async function deployImplementation(wallet, chain, deployOptions, skipExisting = true, verifyOptions = null, saveFunc = null) {
    const contractName = 'InterchainTokenService';
    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};

    contractConfig.salt = deployOptions.salt;
    contractConfig.deployer = wallet.address;

    contracts[contractName] = contractConfig;
    const interchainTokenServiceAddress = await getCreate3Address(contracts.Create3Deployer.address, wallet, deployOptions.salt);
    printInfo('Interchain Token Service will be deployed to', interchainTokenServiceAddress);

    console.log('Does this match any existing deployments?');
    const anwser = readlineSync.question(`Proceed with deployment on ${chain.name}? ${chalk.green('(y/n)')} `);
    if (anwser !== 'y') return;

    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};

    const deployments = {
        tokenManagerDeployer: {
            name: 'Token Manager Deployer',
            async deploy() {
                return await deployContract(
                    deployOptions.deployMethod,
                    wallet,
                    TokenManagerDeployer,
                    [],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                );
            },
        },
        standardizedToken: {
            name: 'Interchain Token Lock Unlock',
            async deploy() {
                return await deployContract(
                    deployOptions.deployMethod,
                    wallet,
                    InterchainToken,
                    [],
                    deployOptions,
                    gasOptions,
                    verifyOptions,
                );
            },
        },
        standardizedTokenDeployer: {
            name: 'Interchain Token Deployer',
            async deploy() {
                return await deployContract(
                    deployOptions.deployMethod,
                    wallet,
                    InterchainTokenDeployer,
                    [contractConfig.standardizedToken],
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
                            deployOptions.deployMethod,
                            wallet,
                            TokenManagerMintBurn,
                            [interchainTokenServiceAddress],
                            deployOptions,
                            gasOptions,
                            verifyOptions,
                            chain,
                        )
                    ).address,
                    tokenManagerMintBurnFrom: (
                        await deployContract(
                            deployOptions.deployMethod,
                            wallet,
                            TokenManagerMintBurnFrom,
                            [interchainTokenServiceAddress],
                            deployOptions,
                            gasOptions,
                            verifyOptions,
                            chain,
                        )
                    ).address,
                    tokenManagerLockUnlock: (
                        await deployContract(
                            deployOptions.deployMethod,
                            wallet,
                            TokenManagerLockUnlock,
                            [interchainTokenServiceAddress],
                            deployOptions,
                            gasOptions,
                            verifyOptions,
                            chain,
                        )
                    ).address,
                    tokenManagerLockUnlockFee: (
                        await deployContract(
                            deployOptions.deployMethod,
                            wallet,
                            TokenManagerLockUnlockFee,
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
                    deployOptions.deployMethod,
                    wallet,
                    InterchainTokenService,
                    [
                        contractConfig.tokenManagerDeployer,
                        contractConfig.standardizedTokenDeployer,
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
        console.log(`Deploying ${chalk.green(deployment.name)}.`);

        const contract = await deployment.deploy();

        if (contract.address === undefined) {
            contractConfig[key] = contract;
            console.log(`Deployed ${deployment.name} at ${JSON.stringify(contract)}`);
        } else {
            contractConfig[key] = contract.address;
            console.log(`Deployed ${deployment.name} at ${contract.address}`);
        }

        if (!verifyOptions?.only && saveFunc) await saveFunc();
    }
}

async function deployITS(
    wallet,
    chain,
    deployOptions,
    operatorAddress = wallet.address,
    skipExisting = true,
    verifyOptions = null,
    saveFunc = null,
) {
    const contractName = 'InterchainTokenService';

    console.log(
        `Deployer ${wallet.address} has ${(await wallet.provider.getBalance(wallet.address)) / 1e18} ${chalk.green(
            chain.tokenSymbol,
        )} and nonce ${await wallet.provider.getTransactionCount(wallet.address)} on ${chain.name}.`,
    );

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};

    contractConfig.salt = deployOptions.salt;
    contractConfig.deployer = wallet.address;

    contracts[contractName] = contractConfig;

    await deployImplementation(wallet, chain, deployOptions, skipExisting, verifyOptions, saveFunc);

    if (skipExisting && isAddress(contractConfig.address)) return;

    console.log(`Deploying Interchain Token Service.`);
    const deploymentParams = defaultAbiCoder.encode(['address', 'string', 'string[]', 'string[]'], [operatorAddress, chain.name, [], []]);
    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    const contract = await deployContract(
        'create3',
        wallet,
        InterchainTokenServiceProxy,
        [contractConfig.implementation, wallet.address, deploymentParams],
        { ...deployOptions, deployerContract: contracts.Create3Deployer.address },
        gasOptions,
        { ...verifyOptions, contractPath: 'contracts/proxies/InterchainTokenServiceProxy.sol:InterchainTokenServiceProxy' },
    );
    contractConfig.address = contract.address;

    console.log(`Deployed Interchain Token Service at ${contract.address}`);

    if (!verifyOptions?.only && saveFunc) await saveFunc();
}

async function deployTokenFactory(wallet, chain, deployOptions, skipExisting = true, verifyOptions = null, saveFunc = null) {
    const contractName = 'InterchainTokenService';

    console.log(
        `Deployer ${wallet.address} has ${(await wallet.provider.getBalance(wallet.address)) / 1e18} ${chalk.green(
            chain.tokenSymbol,
        )} and nonce ${await wallet.provider.getTransactionCount(wallet.address)} on ${chain.name}.`,
    );

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};
    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};

    if (skipExisting && isAddress(contractConfig.interchainTokenFactoryImplementation)) return;

    console.log(`Deploying Interchain Token Factory Implementation.`);

    const implementation = await deployContract(
        'create',
        wallet,
        InterchainTokenFactory,
        [contractConfig.address],
        deployOptions,
        gasOptions,
        {
            ...verifyOptions,
            contractPath: 'contracts/InterchainTokenService.sol:InterchainTokenFactory',
        },
    );

    contractConfig.interchainTokenFactoryImplementation = implementation.address;
    console.log(`Deployed Interchain Token Factroy Implementations at ${implementation.address}`);
    if (!verifyOptions?.only && saveFunc) await saveFunc();

    console.log(`Deploying Interchain Token Factory Proxy.`);

    const proxy = await deployContract(
        'create3',
        wallet,
        InterchainTokenFactoryProxy,
        [implementation.address, wallet.address],
        { ...deployOptions, deployerContract: contracts.Create3Deployer.address },
        gasOptions,
        { ...verifyOptions, contractPath: 'contracts/proxies/InterchainTokenServiceProxy.sol:InterchainTokenFactoryProxy' },
    );

    console.log(`Deployed Interchain Token Factroy Proxy at ${proxy.address}`);

    contractConfig.interchainTokenFactory = proxy.address;
    if (!verifyOptions?.only && saveFunc) await saveFunc();
}

async function upgradeITS(wallet, chain, deployOptions, operatorAddress = wallet.address, verifyOptions = null, saveFunc = null) {
    const contractName = 'InterchainTokenService';

    console.log(
        `Deployer has ${(await wallet.provider.getBalance(wallet.address)) / 1e18} ${chalk.green(
            chain.tokenSymbol,
        )} and nonce ${await wallet.provider.getTransactionCount(wallet.address)} on ${chain.name}.`,
    );

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};

    contractConfig.salt = deployOptions.salt;
    contractConfig.deployer = wallet.address;

    contracts[contractName] = contractConfig;

    await deployImplementation(wallet, chain, deployOptions, false, verifyOptions, saveFunc);

    console.log(`Upgrading Interchain Token Service.`);

    const gasOptions = chain.gasOptions || {};
    const contract = new Contract(contractConfig.address, InterchainTokenService.abi, wallet);

    const codehash = keccak256(await wallet.provider.getCode(contractConfig.implementation));
    await (await contract.upgrade(contractConfig.implementation, codehash, '0x', gasOptions)).wait();

    console.log(`Upgraded Interchain Token Service`);

    if (saveFunc) await saveFunc();
}

async function main(options) {
    const config = loadConfig(options.env);

    const chains = options.chainNames.split(',').map((str) => str.trim());

    for (const chain of chains) {
        if (config.chains[chain.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chain} is not defined in the info file`);
        }
    }

    for (const chainName of chains) {
        const chain = config.chains[chainName.toLowerCase()];

        let wallet;
        const verifyOptions = options.verify ? { env: options.env, chain: chain.name, only: options.verify === 'only' } : null;

        if (options.env === 'local') {
            const [funder] = await require('hardhat').ethers.getSigners();
            wallet = new Wallet(options.privateKey, funder.provider);
            await (await funder.sendTransaction({ to: wallet.address, value: BigInt(1e21) })).wait();
            await deployConstAddressDeployer(wallet, chain, { yes: true }, verifyOptions);
            await deployCreate3Deployer(wallet, chain, { yes: true }, verifyOptions);
            await deployGatewayv5(config, chain, {
                env: 'local',
                deployMethod: 'create',
                mintLimiter: '0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266',
                salt: 'AxelarGateway v6.2',
                governance: '0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266',
                keyID: '0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266',
                yes: true,
                chainNames: 'test',
                privateKey: '0xdf57089febbacf7ba0bc227dafbffa9fc08a93fdc68e1e42411a14efcf23656e',
            });
            chain.contracts.AxelarGasService = { address: wallet.address };
        } else {
            const provider = getDefaultProvider(chain.rpc);
            wallet = new Wallet(options.privateKey, provider);
        }

        const operator = options.operatorAddress || wallet.address;

        if (options.upgrade) {
            await upgradeITS(wallet, chain, options.salt, operator, verifyOptions);
            return;
        }

        const deployOptions = {
            deployMethod: options.deployMethod,
            salt: options.salt,
        };

        if (options.deployMethod === 'create2') {
            deployOptions.deployerContract = chain.contracts.ConstAddressDeployer.address;
        } else if (options.deployMethod === 'create3') {
            deployOptions.deployerContract = chain.contracts.Create3Deployer.address;
        }

        chain.contracts.InterchainTokenService = chain.contracts.InterchainTokenService || {};
        chain.contracts.InterchainTokenService.interchainTokenFactory = await getCreate3Address(
            chain.contracts.Create3Deployer.address,
            wallet,
            options.factorySalt,
        );
        await deployITS(wallet, chain, deployOptions, operator, options.skipExisting, verifyOptions, () => saveConfig(config, options.env));
        chain.contracts.InterchainTokenService.interchainTokenFactory = null;

        deployOptions.salt = options.factorySalt;
        await deployTokenFactory(wallet, chain, deployOptions, options.skipExisting, verifyOptions, () => saveConfig(config, options.env));
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-its').description('Deploy interchain token service');


    program.addOption(
        new Option('--deployMethod <deployMethod>', 'deployment method').choices(['create', 'create2', 'create3']).default('create2'),
    );
    addExtendedOptions(program, { skipExisting: true, upgrade: true });

    program.addOption(new Option('-s, --salt <key>', 'deployment salt to use for ITS deployment').makeOptionMandatory(true).env('SALT'));
    program.addOption(
        new Option('-f, --factorySalt <key>', 'deployment salt to use for Interchain Token Factory deployment')
            .makeOptionMandatory(true)
            .env('FACTORY_SALT'),
    );
    program.addOption(new Option('-o, --operator', 'address of the ITS operator').env('OPERATOR_ADDRESS'));

    program.action(async (options) => {
        options.skipExisting = options.skipExisting === 'true';
        await main(options);
    });

    program.parse();
} else {
    module.exports = { deployITS };
}
