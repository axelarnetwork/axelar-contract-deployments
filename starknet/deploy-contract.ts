#!/usr/bin/env ts-node

'use strict';

import { Command } from 'commander';
import { loadConfig, saveConfig, prompt } from '../common';
import { addStarknetOptions } from './cli-utils';
import {
    deployContract,
    declareContract,
    loadContractArtifact,
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
        contractName,
        classHash,
        constructorCalldata,
        salt,
        yes,
        offline,
        env,
    } = options;

    // Validate execution options
    validateStarknetOptions(env, offline, privateKey, accountAddress);

    // Handle offline mode
    if (offline) {
        console.log(`\nGenerating unsigned transaction for deploying ${contractName} on ${chain.name}...`);

        // For deployment using Universal Deployer Contract (UDC)
        if (!classHash) {
            throw new Error('Class hash is required for offline deployment tx generation. Declare the contract first and provide --classHash.');
        }

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
        const operationName = `deploy_${contractName}`;
        return handleOfflineTransaction(options, chain.name, targetContractAddress, entrypoint, calldata, operationName);
    }

    console.log(`\nDeploying ${contractName} on ${chain.name}...`);

    // Initialize account for online operations
    const provider = getStarknetProvider(chain);
    const account = getStarknetAccount(privateKey!, accountAddress!, provider);

    // Deploy new contract
    let finalClassHash = classHash;

    if (!finalClassHash) {
        // Need to declare the contract first
        console.log(`Loading contract artifact for ${contractName}...`);
        const contractArtifact = loadContractArtifact(contractName);

        console.log(`Declaring contract ${contractName}...`);
        const declareResult = await declareContract(account, contractArtifact);

        console.log(`Contract declared successfully!`);
        console.log(`Class Hash: ${declareResult.classHash}`);
        console.log(`Transaction Hash: ${declareResult.transactionHash}`);

        finalClassHash = declareResult.classHash;
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

    if (!yes) {
        const confirmDeploy = prompt(`Deploy ${contractName} with class hash ${finalClassHash}?`);
        if (!confirmDeploy) {
            console.log('Deployment cancelled.');
            return config;
        }
    }

    console.log(`Deploying contract ${contractName}...`);
    const deployResult = await deployContract(account, finalClassHash, parsedCalldata, salt);

    console.log(`Contract deployed successfully!`);
    console.log(`Contract Address: ${deployResult.contractAddress}`);
    console.log(`Transaction Hash: ${deployResult.transactionHash}`);
    console.log(`Class Hash: ${deployResult.classHash}`);

    // Save deployment info to config
    saveContractConfig(config, chain.name, contractName, {
        address: deployResult.contractAddress,
        classHash: deployResult.classHash,
        deploymentTransactionHash: deployResult.transactionHash,
        deployer: accountAddress,
        salt,
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
    const { env, chainNames } = options;

    // Validate execution options before processing any chains
    validateStarknetOptions(env, options.offline, options.privateKey, options.accountAddress);

    const config = loadConfig(env);
    const chains = chainNames.split(',').map(name => name.trim());

    for (const chainName of chains) {
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
