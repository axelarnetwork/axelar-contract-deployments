'use strict';

import { Option, Command } from 'commander';
import { addBaseOptions } from '../common/cli-utils';
import { CliOptionConfig } from './types';

/**
 * Add Starknet-specific command line options to a Commander program
 * Extends base options with Starknet-specific parameters for key management,
 * offline workflows, Ledger support, and contract operations
 */
export const addStarknetOptions = (program: Command, options: CliOptionConfig = {}): Command => {
    addBaseOptions(program, options);

    if (!options.ignorePrivateKey) {
        program.addOption(
            new Option('-p, --privateKey <privateKey>', 'private key for Starknet account (testnet only)')
                .env('STARKNET_PRIVATE_KEY')
        );
    }

    if (!options.ignoreAccountAddress) {
        program.addOption(
            new Option('-a, --accountAddress <accountAddress>', 'Starknet account address')
                .env('STARKNET_ACCOUNT_ADDRESS')
        );
    }

    if (options.offlineSupport) {
        program.addOption(
            new Option('--offline', 'generate unsigned transaction for offline signing')
                .env('OFFLINE')
        );

        program.addOption(
            new Option('--outputDir <outputDir>', 'output directory for unsigned transactions')
                .default('./starknet-offline-txs')
                .env('OUTPUT_DIR')
        );

        program.addOption(
            new Option('--package', 'create offline package with dependencies')
                .env('PACKAGE')
        );

        program.addOption(
            new Option('--nonce <nonce>', 'nonce for offline transaction generation (required for --offline)')
                .env('NONCE')
        );

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
    }

    if (options.ledgerSupport) {
        program.addOption(
            new Option('--useLedger', 'use Ledger hardware wallet for signing (mainnet)')
                .env('USE_LEDGER')
        );

        program.addOption(
            new Option('--derivationPath <derivationPath>', 'Ledger derivation path')
                .default("m/2645'/579218131'/0'/0'")
                .env('DERIVATION_PATH')
        );
    }

    if (options.signatureSupport) {
        program.addOption(
            new Option('--combineSignatures', 'combine multiple signatures')
                .env('COMBINE_SIGNATURES')
        );

        program.addOption(
            new Option('--multisigSignatures <multisigSignatures>', 'comma-separated list of signature hex values')
                .env('MULTISIG_SIGNATURES')
        );
    }

    if (options.contractName) {
        program.addOption(
            new Option('-c, --contractName <contractName>', 'contract name')
                .makeOptionMandatory(true)
        );
    }

    if (options.classHash) {
        program.addOption(
            new Option('--classHash <classHash>', 'class hash for contract deployment')
        );
    }

    if (options.constructorCalldata) {
        program.addOption(
            new Option('--constructorCalldata <constructorCalldata>', 'constructor calldata as JSON array')
        );
    }

    if (options.salt) {
        program.addOption(
            new Option('-s, --salt <salt>', 'salt for deterministic deployment')
                .env('SALT')
        );
    }

    if (options.verify) {
        program.addOption(
            new Option('-v, --verify', 'verify the deployed contract')
                .env('VERIFY')
        );
    }

    if (options.upgrade) {
        program.addOption(
            new Option('--upgrade', 'upgrade existing contract instead of deploying new one')
                .env('UPGRADE')
        );
    }

    if (options.contractAddress) {
        program.addOption(
            new Option('--contractAddress <contractAddress>', 'contract address for upgrade operations')
                .env('CONTRACT_ADDRESS')
        );
    }

    return program;
};