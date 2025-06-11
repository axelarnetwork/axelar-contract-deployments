'use strict';

import { Command } from 'commander';
import { loadConfig, saveConfig, prompt } from '../common';
import { addStarknetOptions } from './cli-utils';
import {
    deployContract,
    getContractConfig,
    saveContractConfig,
    handleOfflineTransaction,
    validateStarknetOptions,
    getStarknetAccount,
    getStarknetProvider
} from './utils';
import { CallData } from 'starknet';
import {
    Config,
    ChainConfig,
    DeployContractOptions,
    OfflineTransactionResult
} from './types';

async function processCommand(
    config: Config,
    chain: ChainConfig & { name: string },
    options: DeployContractOptions
): Promise<Config | OfflineTransactionResult> {
    const {
        privateKey,
        accountAddress,
        contractConfigName,
        constructorCalldata,
        salt,
        yes,
        offline,
        env,
    } = options;

    // Validate execution options
    validateStarknetOptions(env, offline, privateKey, accountAddress);

    // Get class hash from config
    const contractConfig = getContractConfig(config, chain.name, contractConfigName!);
    if (!contractConfig.classHash) {
        throw new Error(`Class hash not found in config for ${contractConfigName}. Please declare the contract first.`);
    }
    const classHash = contractConfig.classHash;

    // Handle offline mode
    if (offline) {
        console.log(`\nGenerating unsigned transaction for deploying ${contractConfigName} on ${chain.name}...`);

        // Get Universal Deployer Address from config
        const universalDeployerAddress = chain.universalDeployerAddress;
        if (!universalDeployerAddress) {
            throw new Error('Universal Deployer Address not found in chain configuration');
        }

        // Parse constructor calldata if provided
        let parsedCalldata = [];
        if (constructorCalldata) {
            try {
                parsedCalldata = JSON.parse(constructorCalldata);
            } catch (error) {
                throw new Error(`Invalid constructor calldata JSON: ${error.message}`);
            }
        }

        const targetContractAddress = universalDeployerAddress;
        const entrypoint = 'deployContract';
        const calldata = CallData.compile([
            classHash,
            salt,
            true, // origin dependant deployment
            parsedCalldata,
        ]);

        // Use common offline transaction handler
        const operationName = `deploy_${contractConfigName}`;
        return handleOfflineTransaction(options, chain.name, targetContractAddress, entrypoint, calldata, operationName);
    }

    console.log(`\nDeploying ${contractConfigName} on ${chain.name}...`);

    // Initialize account for online operations
    const provider = getStarknetProvider(chain);
    const account = getStarknetAccount(privateKey!, accountAddress!, provider);

    // Deploy contract using class hash from config

    // Parse constructor calldata if provided
    let parsedCalldata = [];
    if (constructorCalldata) {
        try {
            parsedCalldata = JSON.parse(constructorCalldata);
        } catch (error) {
            throw new Error(`Invalid constructor calldata JSON: ${error.message}`);
        }
    }

    if (!yes) {
        const shouldCancel = prompt(`Deploy ${contractConfigName} with class hash ${classHash}?`);
        if (shouldCancel) {
            console.log('Deployment cancelled.');
            process.exit(1);
        }
    }

    console.log(`Deploying contract ${contractConfigName}...`);
    const deployResult = await deployContract(account, classHash, parsedCalldata, salt);

    console.log(`Contract deployed successfully!`);
    console.log(`Contract Address: ${deployResult.contractAddress}`);
    console.log(`Transaction Hash: ${deployResult.transactionHash}`);
    console.log(`Class Hash: ${deployResult.classHash}`);

    // Save deployment info to config
    saveContractConfig(config, chain.name, contractConfigName!, {
        address: deployResult.contractAddress,
        deploymentTransactionHash: deployResult.transactionHash,
        deployer: accountAddress,
        salt,
        deployedAt: new Date().toISOString(),
    });

    return config;
}

async function main(): Promise<void> {
    const program = new Command();

    program
        .name('deploy-contract')
        .description('Deploy Starknet contracts')
        .version('1.0.0');

    addStarknetOptions(program, {
        deployment: true,
        offlineSupport: true,
    });

    program.parse();

    const options = program.opts() as DeployContractOptions;
    const { env } = options;

    // Validate execution options
    validateStarknetOptions(env, options.offline, options.privateKey, options.accountAddress);

    const config = loadConfig(env);
    const chainName = 'starknet';
    const chain = config.chains[chainName];
    
    if (!chain) {
        throw new Error(`Chain ${chainName} not found in environment ${env}`);
    }

    try {
        await processCommand(config, { ...chain, name: chainName }, options);
        console.log(`✅ Deployment completed for ${chainName}\n`);
    } catch (error) {
        console.error(`❌ Deployment failed for ${chainName}: ${error.message}\n`);
        process.exit(1);
    }

    saveConfig(config, env);
    console.log('Configuration updated successfully.');
}

if (require.main === module) {
    main().catch((error) => {
        console.error('Script failed:', error);
        process.exit(1);
    });
}

export {
    processCommand,
};
