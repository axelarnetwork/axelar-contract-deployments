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

async function deployITS(wallet, options, chain) {
    const { privateKey, verifyEnv, deploymentKey, operatorAddress, skipExisting } = options;
    const verifyOptions = verifyEnv ? {env: verifyEnv, chain: chain.name} : null;

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
        privateKey,
        contractName,
        skipExisting,
    };
    await deployMultiple(deploymentOptions, chain, deployments);
}

(async () => {
    const [funder, operator] = await ethers.getSigners();
    const privateKey = keccak256('0x123456');
    console.log(privateKey);
    const wallet = new Wallet(privateKey, funder.provider);
    await (await funder.sendTransaction({to: wallet.address, value: BigInt(1e21)})).wait();
    const options = { 
        privateKey : privateKey, 
        deploymentKey: 'ITS', 
        operatorAddress: operator.address, 
        skipExisting: true
    };
    const info = require('../info/testnet.json');
    const chain = info.chains.ethereum;

    const create3Deployer = await deployContract(wallet, Create3Deployer);
    chain.contracts.create3Deployer = {address: create3Deployer.address};

    console.log(chain);
    await deployITS(wallet, options, chain);
    console.log(chain);
    await deployITS(wallet, options, chain);
    console.log(chain);
})();