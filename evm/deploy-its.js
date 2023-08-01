require('dotenv').config();

const { getCreate3Address } = require('@axelar-network/axelar-gmp-sdk-solidity');
const { deployCreate3, deployCreate2 } = require('./utils');
const {
    Wallet,
    Contract,
    getDefaultProvider,
    utils: { defaultAbiCoder, isAddress, keccak256 },
} = require('ethers');
const readlineSync = require('readline-sync');
const chalk = require('chalk');
const { printInfo, loadConfig, saveConfig } = require('./utils');

const TokenManagerDeployer = require('@axelar-network/interchain-token-service/dist/utils/TokenManagerDeployer.sol/TokenManagerDeployer.json');
const StandardizedTokenLockUnlock = require('@axelar-network/interchain-token-service/dist/token-implementations/StandardizedTokenLockUnlock.sol/StandardizedTokenLockUnlock.json');
const StandardizedTokenMintBurn = require('@axelar-network/interchain-token-service/dist/token-implementations/StandardizedTokenMintBurn.sol/StandardizedTokenMintBurn.json');
const StandardizedTokenDeployer = require('@axelar-network/interchain-token-service/dist/utils/StandardizedTokenDeployer.sol/StandardizedTokenDeployer.json');
const RemoteAddressValidator = require('@axelar-network/interchain-token-service/dist/remote-address-validator/RemoteAddressValidator.sol/RemoteAddressValidator.json');
const RemoteAddressValidatorProxy = require('@axelar-network/interchain-token-service/dist/proxies/RemoteAddressValidatorProxy.sol/RemoteAddressValidatorProxy.json');
const TokenManagerLockUnlock = require('@axelar-network/interchain-token-service/dist/token-manager/implementations/TokenManagerLockUnlock.sol/TokenManagerLockUnlock.json');
const TokenManagerMintBurn = require('@axelar-network/interchain-token-service/dist/token-manager/implementations/TokenManagerMintBurn.sol/TokenManagerMintBurn.json');
const TokenManagerLiquidityPool = require('@axelar-network/interchain-token-service/dist/token-manager/implementations/TokenManagerLiquidityPool.sol/TokenManagerLiquidityPool.json');
const InterchainTokenService = require('@axelar-network/interchain-token-service/dist/interchain-token-service/InterchainTokenService.sol/InterchainTokenService.json');
const InterchainTokenServiceProxy = require('@axelar-network/interchain-token-service/dist/proxies/InterchainTokenServiceProxy.sol/InterchainTokenServiceProxy.json');
const { Command, Option } = require('commander');
const { deployConstAddressDeployer } = require('./deploy-const-address-deployer');
const { deployCreate3Deployer } = require('./deploy-create3-deployer');

/**
 * Function that handles the ITS deployment.
 * @param {*} wallet
 * @param {*} chain
 * @param {*} deploymentKey
 * @param {*} operatorAddress
 * @param {*} skipExisting
 * @param {*} verifyOptions
 * @param {*} saveFunc
 */

async function deployImplementation(wallet, chain, deploymentKey, skipExisting = true, verifyOptions = null, saveFunc = null) {
    const contractName = 'InterchainTokenService';
    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};

    contractConfig.salt = deploymentKey;
    contractConfig.deployer = wallet.address;

    contracts[contractName] = contractConfig;
    const interchainTokenServiceAddress = await getCreate3Address(contracts.Create3Deployer.address, wallet, deploymentKey);
    printInfo('Interchain Token Service will be deployed to', interchainTokenServiceAddress);

    console.log('Does this match any existing deployments?');
    const anwser = readlineSync.question(`Proceed with deployment on ${chain.name}? ${chalk.green('(y/n)')} `);
    if (anwser !== 'y') return;

    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    const create2Deployer = contracts.ConstAddressDeployer.address;

    const deployments = {
        tokenManagerDeployer: {
            name: 'Token Manager Deployer',
            async deploy() {
                return await deployCreate2(
                    create2Deployer,
                    wallet,
                    TokenManagerDeployer,
                    [contracts.Create3Deployer.address],
                    deploymentKey,
                    gasOptions,
                    verifyOptions,
                );
            },
        },
        standardizedTokenLockUnlock: {
            name: 'Standardized Token Lock Unlock',
            async deploy() {
                return await deployCreate2(
                    create2Deployer,
                    wallet,
                    StandardizedTokenLockUnlock,
                    [],
                    deploymentKey,
                    gasOptions,
                    verifyOptions,
                );
            },
        },
        standardizedTokenMintBurn: {
            name: 'Standardized Token Mint Burn',
            async deploy() {
                return await deployCreate2(
                    create2Deployer,
                    wallet,
                    StandardizedTokenMintBurn,
                    [],
                    deploymentKey,
                    gasOptions,
                    verifyOptions,
                );
            },
        },
        standardizedTokenDeployer: {
            name: 'Standardized Token Deployer',
            async deploy() {
                return await deployCreate2(
                    contracts.ConstAddressDeployer.address,
                    wallet,
                    StandardizedTokenDeployer,
                    [
                        contracts.Create3Deployer.address,
                        contractConfig.standardizedTokenLockUnlock,
                        contractConfig.standardizedTokenMintBurn,
                    ],
                    deploymentKey,
                    gasOptions,
                    verifyOptions,
                );
            },
        },
        remoteAddressValidatorImplementation: {
            name: 'Remote Address Validator Implementation',
            async deploy() {
                return await deployCreate2(
                    create2Deployer,
                    wallet,
                    RemoteAddressValidator,
                    [interchainTokenServiceAddress],
                    deploymentKey,
                    gasOptions,
                    verifyOptions,
                );
            },
        },
        remoteAddressValidator: {
            name: 'Remote Address Validator',
            async deploy() {
                const params = defaultAbiCoder.encode(['string[]', 'string[]'], [[], []]);
                return await deployCreate2(
                    create2Deployer,
                    wallet,
                    RemoteAddressValidatorProxy,
                    [contractConfig.remoteAddressValidatorImplementation, wallet.address, params],
                    deploymentKey,
                    gasOptions,
                    {
                        ...verifyOptions,
                        contractPath: 'contracts/proxies/RemoteAddressValidatorProxy.sol:RemoteAddressValidatorProxy',
                    },
                );
            },
        },
        tokenManagerImplementations: {
            name: 'Token Manager Implementations',
            async deploy() {
                const implementations = {
                    tokenManagerLockUnlock: (
                        await deployCreate2(
                            create2Deployer,
                            wallet,
                            TokenManagerLockUnlock,
                            [interchainTokenServiceAddress],
                            deploymentKey,
                            gasOptions,
                            verifyOptions,
                        )
                    ).address,
                    tokenManagerMintBurn: (
                        await deployCreate2(
                            create2Deployer,
                            wallet,
                            TokenManagerMintBurn,
                            [interchainTokenServiceAddress],
                            deploymentKey,
                            gasOptions,
                            verifyOptions,
                        )
                    ).address,
                    tokenManagerLiquidityPool: (
                        await deployCreate2(
                            create2Deployer,
                            wallet,
                            TokenManagerLiquidityPool,
                            [interchainTokenServiceAddress],
                            deploymentKey,
                            gasOptions,
                            verifyOptions,
                        )
                    ).address,
                };

                return implementations;
            },
        },
        implementation: {
            name: 'Interchain Token Service Implementation',
            async deploy() {
                return await deployCreate2(
                    create2Deployer,
                    wallet,
                    InterchainTokenService,
                    [
                        contractConfig.tokenManagerDeployer,
                        contractConfig.standardizedTokenDeployer,
                        contracts.AxelarGateway.address,
                        contracts.AxelarGasService.address,
                        contractConfig.remoteAddressValidator,
                        Object.values(contractConfig.tokenManagerImplementations),
                        chain.name,
                    ],
                    deploymentKey,
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
    deploymentKey,
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

    contractConfig.salt = deploymentKey;
    contractConfig.deployer = wallet.address;

    contracts[contractName] = contractConfig;

    await deployImplementation(wallet, chain, deploymentKey, skipExisting, verifyOptions, saveFunc);

    if (skipExisting && isAddress(contractConfig.address)) return;

    console.log(`Deploying Interchain Token Service.`);

    const gasOptions = contractConfig.gasOptions || chain.gasOptions || {};
    const contract = await deployCreate3(
        contracts.Create3Deployer.address,
        wallet,
        InterchainTokenServiceProxy,
        [contractConfig.implementation, wallet.address, operatorAddress],
        deploymentKey,
        gasOptions,
        { ...verifyOptions, contractPath: 'contracts/proxies/InterchainTokenServiceProxy.sol:InterchainTokenServiceProxy' },
    );

    contractConfig.address = contract.address;
    console.log(`Deployed Interchain Token Service at ${contract.address}`);

    if (!verifyOptions?.only && saveFunc) await saveFunc();
}

async function upgradeITS(wallet, chain, deploymentKey, operatorAddress = wallet.address, verifyOptions = null, saveFunc = null) {
    const contractName = 'InterchainTokenService';

    console.log(
        `Deployer has ${(await wallet.provider.getBalance(wallet.address)) / 1e18} ${chalk.green(
            chain.tokenSymbol,
        )} and nonce ${await wallet.provider.getTransactionCount(wallet.address)} on ${chain.name}.`,
    );

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};

    contractConfig.salt = deploymentKey;
    contractConfig.deployer = wallet.address;

    contracts[contractName] = contractConfig;

    await deployImplementation(wallet, chain, deploymentKey, false, verifyOptions, saveFunc);

    console.log(`Upgrading Interchain Token Service.`);

    const gasOptions = chain.gasOptions || {};
    const contract = new Contract(contractConfig.address, InterchainTokenService.abi, wallet);

    const codehash = keccak256(await wallet.provider.getCode(contractConfig.implementation));
    console.log(codehash);
    await (await contract.upgrade(contractConfig.implementation, codehash, '0x', gasOptions)).wait();

    console.log(`Deployed Interchain Token Service`);

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
            await deployConstAddressDeployer(wallet, chain, null, verifyOptions);
            await deployCreate3Deployer(wallet, chain, null, verifyOptions);
        } else {
            const provider = getDefaultProvider(chain.rpc);
            wallet = new Wallet(options.privateKey, provider);
        }

        const operator = options.operatorAddress || wallet.address;

        if (options.upgrade) {
            await upgradeITS(wallet, chain, options.salt, operator, verifyOptions);
            return;
        }

        await deployITS(wallet, chain, options.salt, operator, options.skipExisting, verifyOptions, () => saveConfig(config, options.env));
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-its').description('Deploy interchain token service');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'stagenet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
    program.addOption(new Option('-s, --salt <key>', 'deployment salt to use for ITS deployment').makeOptionMandatory(true).env('SALT'));
    program.addOption(new Option('-v, --verify <verify>', 'verify the deployed contract on the explorer [true|false|only]').env('VERIFY'));
    program.addOption(new Option('-x, --skipExisting <boolean>', 'skip deploying contracts if they already exist').env('SKIP_EXISTING'));
    program.addOption(new Option('-o, --operator', 'address of the ITS operator').env('OPERATOR_ADDRESS'));
    program.addOption(new Option('-u, --upgrade', 'upgrade ITS').env('UPGRADE'));

    program.action(async (options) => {
        options.skipExisting = options.skipExisting === 'true';
        await main(options);
    });

    program.parse();
} else {
    module.exports = { deployITS };
}
