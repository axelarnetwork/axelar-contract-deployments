'use strict';

import { Option, Command } from 'commander';
import { addBaseOptions } from '../common/cli-utils';
import { CliOptionConfig } from './types';

/**
 * Add Starknet-specific command line options to a Commander program
 * Extends base options with Starknet-specific parameters for key management,
 * offline workflows, Ledger support, and contract operations
 */
export const addStarknetOptions = (program: Command, config: CliOptionConfig = {}): Command => {
    addBaseOptions(program, config);

    if (!config.ignorePrivateKey) {
        program.addOption(
            new Option('--privateKey <privateKey>', 'private key for Starknet account (testnet only, not required for offline tx generation)')
                .env('STARKNET_PRIVATE_KEY')
        );
    }

    if (!config.ignoreAccountAddress) {
        program.addOption(
            new Option('--accountAddress <accountAddress>', 'Starknet account address')
                .env('STARKNET_ACCOUNT_ADDRESS')
        );
    }

    if (config.offlineSupport) {
        program.addOption(
            new Option('--offline', 'generate unsigned transaction for offline signing')
                .env('OFFLINE')
        );

        program.addOption(
            new Option('--outputDir <outputDir>', 'output directory for unsigned transactions (required for --offline)')
                .default('./starknet-offline-txs')
                .env('OUTPUT_DIR')
        );

        program.addOption(
            new Option('--nonce <nonce>', 'nonce for offline transaction generation (required for --offline)')
                .env('NONCE')
        );
    }

    if (config.declaration) {
        program.addOption(
            new Option('--compiledClassHash <compiledClassHash>', 'compiled class hash for declare transactions (generate with: starkli class-hash <compiled_contract_class.json>)')
                .env('COMPILED_CLASS_HASH')
        );
        program.addOption(
            new Option('--contractName <contractName>', 'contract name')
                .makeOptionMandatory(true)
        );
    }

    if (config.deployment) {
        program.addOption(
            new Option('--contractName <contractName>', 'contract name')
                .makeOptionMandatory(true)
        );
        program.addOption(
            new Option('--classHash <classHash>', 'class hash for contract deployment')
        );
        program.addOption(
            new Option('--constructorCalldata <constructorCalldata>', 'constructor calldata as JSON array')
        );
        program.addOption(
            new Option('--salt <salt>', 'salt for deterministic deployment')
                .default('0')
                .env('SALT')
        );
    }

    if (config.upgrade) {
        program.addOption(
            new Option('--contractAddress <contractAddress>', 'contract address')
                .env('CONTRACT_ADDRESS')
        );
        program.addOption(
            new Option('--contractName <contractName>', 'contract name')
                .makeOptionMandatory(true)
        );
        program.addOption(
            new Option('--classHash <classHash>', 'class hash for contract deployment')
        );
    }

    program.addOption(
        new Option('--l1GasMaxAmount <l1GasMaxAmount>', 'maximum L1 gas amount (default: 0)')
            .default('0')
            .env('L1_GAS_MAX_AMOUNT')
    );

    program.addOption(
        new Option('--l1GasMaxPricePerUnit <l1GasMaxPricePerUnit>', 'maximum L1 gas price per unit in wei (default: 0)')
            .default('0')
            .env('L1_GAS_MAX_PRICE_PER_UNIT')
    );

    program.addOption(
        new Option('--l2GasMaxAmount <l2GasMaxAmount>', 'maximum L2 gas amount (default: 0)')
            .default('0')
            .env('L2_GAS_MAX_AMOUNT')
    );

    program.addOption(
        new Option('--l2GasMaxPricePerUnit <l2GasMaxPricePerUnit>', 'maximum L2 gas price per unit in wei (default: 0)')
            .default('0')
            .env('L2_GAS_MAX_PRICE_PER_UNIT')
    );

    program.addOption(
        new Option('--l1DataMaxAmount <l1DataMaxAmount>', 'maximum L1 data amount (default: 0)')
            .default('0')
            .env('L1_DATA_MAX_AMOUNT')
    );

    program.addOption(
        new Option('--l1DataMaxPricePerUnit <l1DataMaxPricePerUnit>', 'maximum L1 data price per unit in wei (default: 0)')
            .default('0')
            .env('L1_DATA_MAX_PRICE_PER_UNIT')
    );

    return program;
};
