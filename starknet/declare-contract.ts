#!/usr/bin/env ts-node

'use strict';

import { Command } from 'commander';
import { loadConfig, saveConfig, prompt } from '../common';
import { addStarknetOptions } from './cli-utils';
import {
    loadContractArtifact,
    handleOfflineDeclareTransaction,
    validateStarknetOptions,
} from './utils';
import {
    Config,
    ChainConfig,
    DeployContractOptions,
    OfflineTransactionResult
} from './types';

async function processCommand(
    _config: Config,
    chain: ChainConfig & { name: string },
    options: DeployContractOptions
): Promise<Config | OfflineTransactionResult> {
    const {
        privateKey,
        accountAddress,
        offline,
        env,
        compiledClassHash,
        contractName,
    } = options;

    // Declare script is offline-only, use starkli for online declarations
    if (!offline) {
        throw new Error('This script only supports offline declare transaction generation. For online contract declaration, use: starkli declare <contract_class.json> --compiled-class-hash <compiled_class_hash>');
    }

    console.log(`\nGenerating unsigned declare transaction on ${chain.name}...`);

    if (!compiledClassHash) {
        throw new Error('Compiled class hash is required for offline declare transaction. Use --compiledClassHash flag. Generate it with: starkli class-hash <compiled_contract_class.json>');
    }

    // Validate execution options for offline mode
    validateStarknetOptions(env, offline, privateKey, accountAddress);

    // Load contract artifact
    console.log(`Loading contract artifact for ${contractName}...`);
    const contractArtifact = loadContractArtifact(contractName);

    // Use offline declare transaction handler
    const operationName = contractName;
    return handleOfflineDeclareTransaction(options, chain.name, contractArtifact, operationName);
}

async function main(): Promise<void> {
    const program = new Command();

    program
        .name('declare-contract')
        .description('Generate offline declare transactions for Starknet contracts. For online declarations, use starkli.')
        .version('1.0.0');

    addStarknetOptions(program, {
        ignorePrivateKey: true, // Private key not needed for offline-only script
        declaration: true,
        offlineSupport: true,
    });

    // Make offline flag mandatory
    program.hook('preAction', (thisCommand) => {
        const opts = thisCommand.opts();
        if (!opts.offline) {
            console.error('Error: --offline flag is required. This script only generates offline transactions.');
            console.error('For online declarations, use: starkli declare <contract_class.json> --compiled-class-hash <compiled_class_hash>');
            process.exit(1);
        }
    });

    program.parse();

    const options = program.opts() as DeployContractOptions;
    const { env, chainNames } = options;

    // Note: validation happens inside processCommand after offline check

    const config = loadConfig(env);
    const chains = chainNames.split(',').map(name => name.trim());

    for (const chainName of chains) {
        const chain = config.chains[chainName];
        if (!chain) {
            throw new Error(`Chain ${chainName} not found in environment ${env}`);
        }

        try {
            const result = await processCommand(config, { ...chain, name: chainName }, options);
            if (result && 'offline' in result) {
                console.log(`✅ Offline declare transaction generated for ${chainName}\n`);
                return; // Exit early for offline mode
            }
        } catch (error) {
            console.error(`❌ Declare transaction generation failed for ${chainName}: ${error.message}\n`);
            process.exit(1);
        }
    }

    if (!options.offline) {
        saveConfig(config, env);
        console.log('Configuration updated successfully.');
    }
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
