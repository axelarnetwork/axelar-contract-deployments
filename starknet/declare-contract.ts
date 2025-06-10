#!/usr/bin/env ts-node

'use strict';

import { Command } from 'commander';
import { loadConfig, saveConfig, prompt } from '../common';
import { addStarknetOptions } from './cli-utils';
import {
    declareContract,
    saveContractConfig,
    validateStarknetOptions,
    getStarknetAccount,
    getStarknetProvider
} from './utils';
import { readFileSync } from 'fs';
import { CompiledContract } from 'starknet';
import {
    Config,
    ChainConfig,
    DeclareContractOptions
} from './types';

async function processCommand(
    config: Config,
    chain: ChainConfig & { name: string },
    options: DeclareContractOptions
): Promise<Config> {
    const {
        privateKey,
        accountAddress,
        contractConfigName,
        contractPath,
        yes,
        env,
    } = options;

    // Validate execution options
    validateStarknetOptions(env, false, privateKey, accountAddress);

    console.log(`\nDeclaring contract on ${chain.name}...`);

    // Initialize account for online operations
    const provider = getStarknetProvider(chain);
    const account = getStarknetAccount(privateKey!, accountAddress!, provider);

    // Load contract artifact from file path
    console.log(`Loading contract artifact from ${contractPath}...`);
    let contractArtifact;
    try {
        const contractData = readFileSync(contractPath, 'utf8');
        contractArtifact = JSON.parse(contractData) as CompiledContract;
    } catch (error) {
        throw new Error(`Failed to load contract artifact from ${contractPath}: ${error.message}`);
    }

    // Load CASM if it exists
    let casmArtifact;
    let casmPath = contractPath.replace('.contract_class.json', '.compiled_contract_class.json');
    try {
        const casmData = readFileSync(casmPath, 'utf8');
        casmArtifact = JSON.parse(casmData) as CompiledContract;
        console.log(`Found CASM file at ${casmPath}`);
    } catch (error) {
        throw new Error(`Failed to parse CASM file at ${casmPath}`);
    }

    if (!yes) {
        const shouldCancel = prompt(`Are you sure you want to declare the contract from ${contractPath}?`);
        if (shouldCancel) {
            console.log('Declaration cancelled.');
            process.exit(1);
        }
    }

    console.log(`Declaring contract...`);

    try {
        const declareResult = await declareContract(account, { contract: contractArtifact, casm: casmArtifact });

        console.log(`Contract declared successfully!`);
        console.log(`Class Hash: ${declareResult.classHash}`);
        console.log(`Transaction Hash: ${declareResult.transactionHash}`);

        // Save class hash to config under the contractConfigName
        saveContractConfig(config, chain.name, contractConfigName, {
            classHash: declareResult.classHash,
            declaredAt: new Date().toISOString(),
        });
    } catch (error: any) {
        throw error;
    }

    return config;
}

async function main(): Promise<void> {
    const program = new Command();

    program
        .name('declare-contract')
        .description('Declare Starknet contracts and save class hash to config')
        .version('1.0.0');

    addStarknetOptions(program, {
        declare: true,
    });

    program.parse();

    const options = program.opts() as DeclareContractOptions;
    const { env } = options;

    // Validate execution options
    validateStarknetOptions(env, false, options.privateKey, options.accountAddress);

    const config = loadConfig(env);
    const chainName = 'starknet';
    const chain = config.chains[chainName];
    
    if (!chain) {
        throw new Error(`Chain ${chainName} not found in environment ${env}`);
    }

    try {
        await processCommand(config, { ...chain, name: chainName }, options);
        console.log(`✅ Declaration completed for ${chainName}\n`);
    } catch (error) {
        console.error(`❌ Declaration failed for ${chainName}: ${error.message}\n`);
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
