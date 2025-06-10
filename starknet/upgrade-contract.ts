#!/usr/bin/env ts-node

'use strict';

import { Command } from 'commander';
import { loadConfig, saveConfig, prompt } from '../common';
import { addStarknetOptions } from './cli-utils';
import {
    upgradeContract,
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
    UpgradeContractOptions,
    OfflineTransactionResult
} from './types';

async function processCommand(
    config: Config,
    chain: ChainConfig & { name: string },
    options: UpgradeContractOptions
): Promise<Config | OfflineTransactionResult> {
    const {
        privateKey,
        accountAddress,
        contractConfigName,
        classHash,
        contractAddress,
        yes,
        offline,
        env,
    } = options;

    // Validate execution options
    validateStarknetOptions(env, offline, privateKey, accountAddress);

    // Get target contract address
    const targetAddress = contractAddress || getContractConfig(config, chain.name, contractConfigName!).address;
    if (!targetAddress) {
        throw new Error('Contract address required for upgrade. Provide --contractAddress or ensure contract exists in config.');
    }

    if (!classHash) {
        throw new Error('Class hash required for upgrade. Provide --classHash.');
    }

    // Handle offline mode
    if (offline) {
        console.log(`\nGenerating unsigned transaction for upgrading ${contractConfigName} on ${chain.name}...`);

        // Prepare upgrade call
        const entrypoint = 'upgrade';
        const calldata = CallData.compile([classHash]);

        // Use common offline transaction handler
        const operationName = `upgrade_${contractConfigName}`;
        return handleOfflineTransaction(options, chain.name, targetAddress, entrypoint, calldata, operationName);
    }

    console.log(`\nUpgrading ${contractConfigName} on ${chain.name}...`);

    // Initialize account for online operations
    const provider = getStarknetProvider(chain);
    const account = getStarknetAccount(privateKey!, accountAddress!, provider);

    if (!yes) {
        const shouldCancel = prompt(`Are you sure you want to upgrade ${contractConfigName} at ${targetAddress} to class hash ${classHash}?`);
        if (shouldCancel) {
            console.log('Upgrade cancelled.');
            process.exit(1);
        }
    }

    console.log(`Upgrading contract at ${targetAddress} to class hash ${classHash}...`);
    const upgradeResult = await upgradeContract(account, targetAddress, classHash);

    console.log(`Contract upgraded successfully!`);
    console.log(`Contract Address: ${upgradeResult.contractAddress}`);
    console.log(`Transaction Hash: ${upgradeResult.transactionHash}`);
    console.log(`New Class Hash: ${upgradeResult.newClassHash}`);

    // Update config with new class hash
    saveContractConfig(config, chain.name, contractConfigName!, {
        classHash: upgradeResult.newClassHash,
        lastUpgradeTransactionHash: upgradeResult.transactionHash,
    });

    return config;
}

async function main(): Promise<void> {
    const program = new Command();

    program
        .name('upgrade-contract')
        .description('Upgrade Starknet contracts')
        .version('1.0.0');

    addStarknetOptions(program, {
        upgrade: true,
        offlineSupport: true,
    });

    program.parse();

    const options = program.opts() as UpgradeContractOptions;
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
        console.log(`✅ Upgrade completed for ${chainName}\n`);
    } catch (error) {
        console.error(`❌ Upgrade failed for ${chainName}: ${error.message}\n`);
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
