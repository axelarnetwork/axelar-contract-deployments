require('dotenv').config();

const { getCreate3Address } = require('@axelar-network/axelar-gmp-sdk-solidity');
const { deployContract, deployCreate3, isAddressArray, writeJSON } = require('./utils');
const {
    Wallet,
    Contract,
    getDefaultProvider,
    utils: { defaultAbiCoder, isAddress, keccak256 },
} = require('ethers');
const chalk = require('chalk');

const TokenManagerDeployer = require('../artifacts/interchain-token-service/contracts/utils/TokenManagerDeployer.sol/TokenManagerDeployer.json');
const StandardizedTokenLockUnlock = require('../artifacts/interchain-token-service/contracts/token-implementations/StandardizedTokenLockUnlock.sol/StandardizedTokenLockUnlock.json');
const StandardizedTokenMintBurn = require('../artifacts/interchain-token-service/contracts/token-implementations/StandardizedTokenMintBurn.sol/StandardizedTokenMintBurn.json');
const StandardizedTokenDeployer = require('../artifacts/interchain-token-service/contracts/utils/StandardizedTokenDeployer.sol/StandardizedTokenDeployer.json');
const RemoteAddressValidator = require('../artifacts/interchain-token-service/contracts/linker-router/RemoteAddressValidator.sol/RemoteAddressValidator.json');
const RemoteAddressValidatorProxy = require('../artifacts/interchain-token-service/contracts/proxies/LinkerRouterProxy.sol/RemoteAddressValidatorProxy.json');
const TokenManagerLockUnlock = require('../artifacts/interchain-token-service/contracts/token-manager/implementations/TokenManagerLockUnlock.sol/TokenManagerLockUnlock.json');
const TokenManagerMintBurn = require('../artifacts/interchain-token-service/contracts/token-manager/implementations/TokenManagerMintBurn.sol/TokenManagerMintBurn.json');
const TokenManagerLiquidityPool = require('../artifacts/interchain-token-service/contracts/token-manager/implementations/TokenManagerLiquidityPool.sol/TokenManagerLiquidityPool.json');
const InterchainTokenService = require('../artifacts/interchain-token-service/contracts/interchain-token-service/InterchainTokenService.sol/InterchainTokenService.json');
const InterchainTokenServiceProxy = require('../artifacts/interchain-token-service/contracts/proxies/InterchainTokenServiceProxy.sol/InterchainTokenServiceProxy.json');
const IInterchainTokenService = require('../artifacts/interchain-token-service/contracts/interfaces/IInterchainTokenService.sol/IInterchainTokenService.json');
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
        `Deployer has ${(await wallet.provider.getBalance(wallet.address)) / 1e18} ${chalk.green(
            chain.tokenSymbol,
        )} and nonce ${await wallet.provider.getTransactionCount(wallet.address)} on ${chain.name}.`,
    );

    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};
    contracts[contractName] = contractConfig;
    const interchainTokenServiceAddress = await getCreate3Address(contracts.Create3Deployer.address, wallet, deploymentKey);

    const deployments = {
        tokenManagerDeployer: {
            name: 'Token Manager Deployer',
            async deploy() {
                return await deployContract(wallet, TokenManagerDeployer, [contracts.Create3Deployer.address], {}, verifyOptions);
            },
        },
        standardizedTokenLockUnlock: {
            name: 'Standardized Token Lock Unlock',
            async deploy() {
                return await deployContract(wallet, StandardizedTokenLockUnlock, [], {}, verifyOptions);
            },
        },
        standardizedTokenMintBurn: {
            name: 'Standardized Token Mint Burn',
            async deploy() {
                return await deployContract(wallet, StandardizedTokenMintBurn, [], {}, verifyOptions);
            },
        },
        standardizedTokenDeployer: {
            name: 'Standardized Token Deployer',
            async deploy() {
                return await deployContract(
                    wallet,
                    StandardizedTokenDeployer,
                    [
                        contracts.Create3Deployer.address,
                        contractConfig.standardizedTokenLockUnlock,
                        contractConfig.standardizedTokenMintBurn,
                    ],
                    {},
                    verifyOptions,
                );
            },
        },
        remoteAddressValidatorImplementation: {
            name: 'Linker Router Implementations',
            async deploy() {
                return await deployContract(wallet, RemoteAddressValidator, [interchainTokenServiceAddress], {}, verifyOptions);
            },
        },
        remoteAddressValidator: {
            name: 'Linker Router Proxy',
            async deploy() {
                const params = defaultAbiCoder.encode(['string[]', 'string[]'], [[], []]);
                return await deployContract(
                    wallet,
                    RemoteAddressValidatorProxy,
                    [contractConfig.remoteAddressValidatorImplementation, wallet.address, params],
                    {},
                    verifyOptions,
                );
            },
        },
        tokenManagerImplementations: {
            name: 'Token Manager Implementations',
            async deploy() {
                const implementations = [];

                for (const contractJson of [TokenManagerLockUnlock, TokenManagerMintBurn, TokenManagerLiquidityPool]) {
                    const impl = await deployContract(wallet, contractJson, [interchainTokenServiceAddress], {}, verifyOptions);
                    implementations.push(impl);
                }

                return implementations;
            },
        },
        implementation: {
            name: 'Interchain Token Service Implementation',
            async deploy() {
                return await deployContract(
                    wallet,
                    InterchainTokenService,
                    [
                        contractConfig.tokenManagerDeployer,
                        contractConfig.standardizedTokenDeployer,
                        contracts.AxelarGateway.address,
                        contracts.AxelarGasService.address,
                        contractConfig.remoteAddressValidator,
                        contractConfig.tokenManagerImplementations,
                        chain.name,
                    ],
                    {},
                    verifyOptions,
                );
            },
        },
        address: {
            name: 'Interchain Token Service Proxy',
            async deploy() {
                return await deployCreate3(
                    contracts.Create3Deployer.address,
                    wallet,
                    InterchainTokenServiceProxy,
                    [contractConfig.implementation, wallet.address, operatorAddress],
                    deploymentKey,
                    null,
                    verifyOptions,
                );
            },
        },
    };

    for (const key in deployments) {
        if (skipExisting && (isAddress(contractConfig[key]) || isAddressArray(contractConfig[key]))) continue;

        const deployment = deployments[key];
        console.log(`Deploying ${deployment.name}.`);

        const contract = await deployment.deploy();

        if (Array.isArray(contract)) {
            const addresses = contract.map((val) => val.address);
            contractConfig[key] = addresses;
            console.log(`Deployed ${deployment.name} at ${JSON.stringify(addresses)}`);
        } else {
            contractConfig[key] = contract.address;
            console.log(`Deployed ${deployment.name} at ${contract.address}`);
        }

        if (saveFunc) await saveFunc();
    }

    return new Contract(deployments.address, IInterchainTokenService.abi, wallet);
}

async function main(options) {
    const config = require(`${__dirname}/../info/${options.env === 'local' ? 'testnet' : options.env}.json`);

    const chains = options.chainNames.split(',').map((str) => str.trim());

    for (const chain of chains) {
        if (config.chains[chain.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chain} is not defined in the info file`);
        }
    }

    for (const chainName of chains) {
        const chain = config.chains[chainName.toLowerCase()];

        let wallet;
        const verifyOptions = options.verify ? { env: options.env, chain: chain.name } : null;
        console.log(options);
        if (options.env === 'local') {
            const [funder] = await require('hardhat').ethers.getSigners();
            wallet = new Wallet(options.privateKey, funder.provider);
            await (await funder.sendTransaction({ to: wallet.address, value: BigInt(1e21) })).wait();
            await deployConstAddressDeployer(wallet, chain, keccak256('0x1234'), verifyOptions);
            await deployCreate3Deployer(wallet, chain, keccak256('0x0123'), verifyOptions);
            
        } else {
            const provider = getDefaultProvider(chain.rpc);
            wallet = new Wallet(options.privateKey, provider);
        }

        await deployITS(wallet, chain, options.key, options.operatorAddress, options.skipExisting, verifyOptions, () =>
            writeJSON(config, `${__dirname}/../info/${options.env}.json`),
        );
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('deploy-create3-deployer').description('Deploy create3 deployer');

    program.addOption(
        new Option('-e, --env <env>', 'environment')
            .choices(['local', 'devnet', 'testnet', 'mainnet'])
            .default('testnet')
            .makeOptionMandatory(true)
            .env('ENV'),
    );
    program.addOption(new Option('-n, --chainNames <chainNames>', 'chain names').makeOptionMandatory(true));
    program.addOption(new Option('-p, --privateKey <privateKey>', 'private key').makeOptionMandatory(true).env('PRIVATE_KEY'));
    program.addOption(
        new Option('-k, --key <key>', 'deployment key to use for create3 deployment').makeOptionMandatory(true).env('DEPLOYMENT_KEY'),
    );
    program.addOption(new Option('-v, --verify <boolean>', 'verify the deployed contract on the explorer').env('VERIFY'));
    program.addOption(new Option('-s, --skipExisting <boolean>', 'skip deploying contracts if they already exist').env('SKIP_EXISTING'));
    program.addOption(new Option('-o, --operator', 'address of the ITS operator').env('OPERATOR_ADDRESS'));

    program.action(async (options) => {
        await main(options);
    });

    program.parse();
} else {
    module.exports = { deployITS };
}
