const { getCreate3Address } = require("@axelar-network/axelar-gmp-sdk-solidity");
const { deployContract, deployCreate3, deployMultiple } = require("./utils");
const { ethers } = require('hardhat');

async function deployITS(options, chain) {
    const { privateKey, verifyEnv, deploymentKey, operatorAddress, skipExisting } = options;
    const verifyOptions = verifyEnv ? {env: verifyEnv, chain: chain.name} : null;

    const contractName = 'InterchainTokenService';
    
    
    const contracts = chain.contracts;
    const contractConfig = contracts[contractName] || {};
    contracts[contractName] = contractConfig;
    const interchainTokenServiceAddress = await getCreate3Address(contractConfig.create3Deployer.address, wallet, deploymentKey);

    const deployments = {
        tokenManagerDeployer: async (wallet) => {
            return await deployContract(wallet, 'TokenManagerDeployer', [contracts.create3Deployer.address], {}, verifyOptions);
        },
        standardizedTokenLockUnlock: async (wallet) => {
            return await deployContract(wallet, 'StandardizedTokenLockUnlock', [], {}, verifyOptions);
        },
        standardizedTokenMintBurn: async (wallet) => {
            return await deployContract(wallet, 'StandardizedTokenMintBurn', [], {}, verifyOptions);
        },
        standardizedTokenDeployer: async (wallet) => {
            return await deployContract(wallet, 'StandardizedTokenDeployer', [
                contracts.create3Deployer.address,
                contractConfig.standardizedTokenLockUnlock,
                contractConfig.standardizedTokenMintBurn,
            ], {}, verifyOptions);
        },
        linkerRouterImplementation : async (wallet) => {
            return await deployContract(wallet, 'LinkerRouter', [[interchainTokenServiceAddress]], {}, verifyOptions);
        },
        linkerRouter: async (wallet) => {
            const params = defaultAbiCoder.encode(['string[]', 'string[]'], [[], []]);
            return await deployContract(wallet, 'LinkerRouterProxy', [contractConfig.linkerRouterImplementation, wallet.address, params], {}, verifyOptions);
        },
        tokenManagerImplementations: async (wallet) => {
            const implementations = [];

            for (const type of ['LockUnlock', 'MintBurn', 'LiquidityPool']) {
                const impl = await deployContract(wallet, `TokenManager${type}`, [interchainTokenServiceAddress], {}, verifyOptions);
                implementations.push(impl);
            }
        
            return implementations;
        },
        implementation: async (wallet) => {
            return await deployContract(wallet, 'InterchainTokenService', [
                contractConfig.tokenManagerDeployer,
                contractConfig.standardizedTokenDeployer,
                contracts.AxelarGateway.address,
                contracts.AxelarGasService.address,
                contractConfig.linkerRouter,
                contractConfig.tokenManagerImplementations,
                chain.name,
            ], {}, verifyOptions);
        },
        address: async (wallet) => {
            return await deployCreate3(contracts.create3Deployer.address, wallet, 'InterchainTokenServiceProxy', [
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
    const [wallet, operator] = await ethers.getSigners();
    const options = { 
        privateKey : wallet.privateKey, 
        verifyEnv: true, 
        deploymentKey: 'ITS', 
        operatorAddressL: operator.address, 
        skipExisting: true
    };
    const info = require('../info/testnet.json');
    const chain = info.chains.ethereum;

    await require('./deploy-create3-deployer').deploy(options, chain);
    
    await deployITS(options, chain);
})();