require('dotenv').config();

const { getCreate3Address } = require("@axelar-network/axelar-gmp-sdk-solidity");
const { deployContract, deployCreate3, deployMultiple } = require("./utils");
const { ethers } = require('hardhat');
const { Wallet, utils: { defaultAbiCoder } } = ethers;
const { keccak256 } = require("ethers/lib/utils");

const Create3Deployer = require('../artifacts/axelar-gmp-sdk-solidity/contracts/deploy/Create3Deployer.sol/Create3Deployer.json');
const TokenManagerDeployer = require('../artifacts/interchain-token-service/contracts/utils/TokenManagerDeployer.sol/TokenManagerDeployer.json');
const StandardizedTokenLockUnlock = require('../artifacts/interchain-token-service/contracts/token-implementations/StandardizedTokenLockUnlock.sol/StandardizedTokenLockUnlock.json');
const StandardizedTokenMintBurn = require('../artifacts/interchain-token-service/contracts/token-implementations/StandardizedTokenMintBurn.sol/StandardizedTokenMintBurn.json');
const StandardizedTokenDeployer = require('../artifacts/interchain-token-service/contracts/utils/StandardizedTokenDeployer.sol/StandardizedTokenDeployer.json');
const LinkerRouter = require('../artifacts/interchain-token-service/contracts/linker-router/LinkerRouter.sol/LinkerRouter.json');
const LinkerRouterProxy = require('../artifacts/interchain-token-service/contracts/proxies/LinkerRouterProxy.sol/LinkerRouterProxy.json');
const TokenManagerLockUnlock = require('../artifacts/interchain-token-service/contracts/token-manager/implementations/TokenManagerLockUnlock.sol/TokenManagerLockUnlock.json');
const TokenManagerMintBurn = require('../artifacts/interchain-token-service/contracts/token-manager/implementations/TokenManagerMintBurn.sol/TokenManagerMintBurn.json');
const TokenManagerLiquidityPool = require('../artifacts/interchain-token-service/contracts/token-manager/implementations/TokenManagerLiquidityPool.sol/TokenManagerLiquidityPool.json');
const InterchainTokenService = require('../artifacts/interchain-token-service/contracts/interchain-token-service/InterchainTokenService.sol/InterchainTokenService.json');
const InterchainTokenServiceProxy = require('../artifacts/interchain-token-service/contracts/proxies/InterchainTokenServiceProxy.sol/InterchainTokenServiceProxy.json');
const { getDefaultProvider } = require("ethers");
const { Command, Option } = require("commander");

async function deployITS(wallet, chain, deploymentKey, operatorAddress = wallet.address, skipExisting = true, verifyOptions = null) {
    const contractName = 'InterchainTokenService';
    
    
    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};
    contracts[contractName] = contractConfig;
    const interchainTokenServiceAddress = await getCreate3Address(contracts.create3Deployer.address, wallet, deploymentKey);

    const deployments = {
        tokenManagerDeployer: async () => {
            return await deployContract(wallet, TokenManagerDeployer, [contracts.create3Deployer.address], {}, verifyOptions);
        },
        standardizedTokenLockUnlock: async () => {
            return await deployContract(wallet, StandardizedTokenLockUnlock, [], {}, verifyOptions);
        },
        standardizedTokenMintBurn: async () => {
            return await deployContract(wallet, StandardizedTokenMintBurn, [], {}, verifyOptions);
        },
        standardizedTokenDeployer: async () => {
            return await deployContract(wallet, StandardizedTokenDeployer, [
                contracts.create3Deployer.address,
                contractConfig.standardizedTokenLockUnlock,
                contractConfig.standardizedTokenMintBurn,
            ], {}, verifyOptions);
        },
        linkerRouterImplementation : async () => {
            return await deployContract(wallet, LinkerRouter, [interchainTokenServiceAddress], {}, verifyOptions);
        },
        linkerRouter: async () => {
            const params = defaultAbiCoder.encode(['string[]', 'string[]'], [[], []]);
            return await deployContract(wallet, LinkerRouterProxy, [contractConfig.linkerRouterImplementation, wallet.address, params], {}, verifyOptions);
        },
        tokenManagerImplementations: async () => {
            const implementations = [];

            for (const type of ['LockUnlock', 'MintBurn', 'LiquidityPool']) {
                const impl = await deployContract(wallet, eval(`TokenManager${type}`), [interchainTokenServiceAddress], {}, verifyOptions);
                implementations.push(impl);
            }
        
            return implementations;
        },
        implementation: async () => {
            return await deployContract(wallet, InterchainTokenService, [
                contractConfig.tokenManagerDeployer,
                contractConfig.standardizedTokenDeployer,
                contracts.AxelarGateway.address,
                contracts.AxelarGasService.address,
                contractConfig.linkerRouter,
                contractConfig.tokenManagerImplementations,
                chain.name,
            ], {}, verifyOptions);
        },
        address: async () => {
            return await deployCreate3(contracts.create3Deployer.address, wallet, InterchainTokenServiceProxy, [
                contractConfig.implementation,
                wallet.address,
                operatorAddress,
            ], deploymentKey, null, verifyOptions);
        },
    }
    const deploymentOptions = {
        contractName,
        skipExisting,
    };
    await deployMultiple(deploymentOptions, chain, deployments);
}

async function main(options) {
    const config = require(`${__dirname}/../info/${options.env}.json`);

    const chains = options.chainNames.split(',');

    for (const chain of chains) {
        if (config.chains[chain.toLowerCase()] === undefined) {
            throw new Error(`Chain ${chain} is not defined in the info file`);
        }
    }

    for (const chainName of chains) {
        //const provider = getDefaultProvider(chain.rpc);
        const [funder] = await ethers.getSigners();
        const wallet = new Wallet(options.privateKey, funder.provider);
        await (await funder.sendTransaction({to: wallet.address, value: BigInt(1e21)})).wait();

        const chain = config.chains[chainName.toLowerCase()]

        const create3Deployer = await deployContract(wallet, Create3Deployer);
        chain.contracts.create3Deployer = {address: create3Deployer.address};

        const verifyOptions = options.verify ? {env: options.env, chain: chain.name} : null;
        await deployITS(wallet, chain, options.key, options.operatorAddress, options.skipExisting, verifyOptions);
        //writeJSON(config, `${__dirname}/../info/${options.env}.json`);
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
    program.addOption(new Option('-k, --key <key>', 'deployment key to use for create3 deployment').makeOptionMandatory(true).env('DEPLOYMENT_KEY'));
    program.addOption(new Option('-v, --verify <boolean>', 'verify the deployed contract on the explorer').env('VERIFY'));
    program.addOption(new Option('-s, --skipExisting <boolean>', 'skip deploying contracts if they already exist').env('SKIP_EXISTING'));
    program.addOption(new Option('-o, --operator', 'address of the ITS operator').env('OPERATOR_ADDRESS'));

    program.action((options) => {
        console.log(options)
        main(options);
    });

    program.parse();
} else {
    module.exports = { deployITS };
}