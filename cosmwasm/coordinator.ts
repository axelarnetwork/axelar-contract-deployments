#!/usr/bin/env ts-node
import { Command, Option } from 'commander';
import 'dotenv/config';

import { printError } from '../common';
import { ChainConfigManager } from './coordinator/chain-config';
import { ConfigManager } from './coordinator/config';
import { DeploymentManager } from './coordinator/deployment';
import { GovernanceManager } from './coordinator/governance';
import { InstantiationManager } from './coordinator/instantiation';
import { OptionProcessor } from './coordinator/option-processor';
import type {
    ConfigureChainOptions,
    DeployContractsOptions,
    InstantiateChainOptions,
    RegisterDeploymentOptions,
    RegisterProtocolOptions,
} from './coordinator/option-processor';

const program = new Command();

program.name('coordinator').description('Submit governance proposal to instantiate chain contracts using Coordinator');

program
    .command('deploy')
    .description('Deploy VotingVerifier, MultisigProver, and Gateway contracts without instantiating them')
    .addOption(
        new Option('-e, --env <environment>', 'Environment (testnet, mainnet, devnet-amplifier, stagenet)')
            .env('ENV')
            .makeOptionMandatory(true),
    )
    .addOption(new Option('-m, --mnemonic <mnemonic>', 'Mnemonic for signing').env('MNEMONIC').makeOptionMandatory(true))
    .option('-y, --yes', 'Skip confirmation prompts')
    .option('--deposit <deposit>', 'Proposal deposit amount', '1000000000')
    .option('--run-as <address>', 'Address to run the contract as')
    .option('--artifact-dir <path>', 'Path to contract artifacts directory')
    .option('--version <version>', 'Contract version for artifact downloading (e.g., v1.0.0 or commit hash)')
    .action(async (options) => {
        try {
            const processedOptions = OptionProcessor.processOptions(options);
            const configManager = new ConfigManager(processedOptions.env);
            const deploymentManager = new DeploymentManager(configManager);
            await deploymentManager.deployContracts(processedOptions as DeployContractsOptions);
        } catch (error) {
            printError('Error in CLI:', (error as Error).message);
            throw error;
        }
    });

program
    .command('register-protocol')
    .description('Submit governance proposal to register protocol contracts with Coordinator')
    .addOption(
        new Option('-e, --env <environment>', 'Environment (testnet, mainnet, devnet-amplifier, stagenet)')
            .env('ENV')
            .makeOptionMandatory(true),
    )
    .addOption(new Option('-m, --mnemonic <mnemonic>', 'Mnemonic for signing').env('MNEMONIC').makeOptionMandatory(true))
    .option('-y, --yes', 'Skip confirmation prompts')
    .option('--deposit <deposit>', 'Proposal deposit amount', '1000000000')
    .option('--run-as <address>', 'Address to run the contract as')
    .action(async (options) => {
        try {
            const processedOptions = OptionProcessor.processOptions(options);
            const configManager = new ConfigManager(processedOptions.env);
            const governanceManager = new GovernanceManager(configManager);
            await governanceManager.registerProtocol(processedOptions as RegisterProtocolOptions);
        } catch (error) {
            printError('Error in CLI:', (error as Error).message);
            throw error;
        }
    });

program
    .command('register-deployment')
    .description('Submit governance proposal to register a new deployment with Coordinator')
    .addOption(
        new Option('-n, --chainName <chainName>', 'Chain name (e.g., avalanche, ethereum-sepolia, celo)')
            .env('CHAIN')
            .makeOptionMandatory(true),
    )
    .addOption(
        new Option('-e, --env <environment>', 'Environment (testnet, mainnet, devnet-amplifier, stagenet)')
            .env('ENV')
            .makeOptionMandatory(true),
    )
    .addOption(new Option('-m, --mnemonic <mnemonic>', 'Mnemonic for signing').env('MNEMONIC').makeOptionMandatory(true))
    .option('-y, --yes', 'Skip confirmation prompts')
    .option('--deposit <deposit>', 'Proposal deposit amount', '1000000000')
    .option('--run-as <address>', 'Address to run the contract as')
    .action(async (options) => {
        try {
            const processedOptions = OptionProcessor.processOptions(options);
            const configManager = new ConfigManager(processedOptions.env);
            const governanceManager = new GovernanceManager(configManager);
            await governanceManager.registerDeployment(processedOptions as RegisterDeploymentOptions);
        } catch (error) {
            printError('Error in CLI:', (error as Error).message);
            throw error;
        }
    });

program
    .command('configure')
    .description('Creates or updates a configuration for a chain')
    .requiredOption('--contract-admin <address>', 'Admin address for MultisigProver, Gateway, and VotingVerifier contracts')
    .requiredOption('--multisig-admin <address>', 'Multisig admin address passed to the MultisigProver contract')
    .addOption(new Option('--salt <salt>', 'Custom salt for contracts instantiation').env('SALT').makeOptionMandatory(true))
    .addOption(
        new Option('-n, --chainName <chainName>', 'Chain name (e.g., ethereum-sepolia, celo)').env('CHAIN').makeOptionMandatory(true),
    )
    .addOption(
        new Option('-e, --env <environment>', 'Environment (testnet, mainnet, devnet-amplifier, stagenet)')
            .env('ENV')
            .makeOptionMandatory(true),
    )
    .option('-y, --yes', 'Skip confirmation prompts')
    .option('--governance-address <address>', 'Governance address')
    .option('--service-name <name>', 'Service name')
    .option('--rewards-address <address>', 'Rewards address')
    .option('--source-gateway-address <address>', 'Source gateway address (optional, defaults to AxelarGateway address)')
    .option('--voting-threshold <threshold>', 'Voting threshold (e.g., "51,100")')
    .option('--block-expiry <expiry>', 'Block expiry', '10')
    .option(
        '--confirmation-height <height>',
        'Confirmation height (default is overvalued for safety - double check with the team)',
        '1000000',
    )
    .option('--msg-id-format <format>', 'Message ID format', 'hex_tx_hash_and_event_index')
    .option('--address-format <format>', 'Address format', 'eip55')
    .option('--signing-threshold <threshold>', 'Signing threshold (e.g., "51,100")')
    .option('--verifier-set-diff-threshold <threshold>', 'Verifier set diff threshold', '1')
    .option('--encoder <encoder>', 'Encoder type', 'abi')
    .option('--key-type <type>', 'Key type', 'ecdsa')
    .option('--domain-separator <separator>', 'Domain separator')
    .action(async (options) => {
        try {
            const processedOptions = OptionProcessor.processOptions(options);
            const configManager = new ConfigManager(processedOptions.env);
            const chainConfigManager = new ChainConfigManager(configManager);
            chainConfigManager.updateChainConfig(processedOptions as ConfigureChainOptions);
        } catch (error) {
            printError('Error in CLI:', (error as Error).message);
            throw error;
        }
    });

program
    .command('instantiate')
    .description('Submit governance proposal to instantiate chain contracts using Coordinator')
    .addOption(
        new Option('-n, --chainName <chainName>', 'Chain name (e.g., ethereum-sepolia, celo)').env('CHAIN').makeOptionMandatory(true),
    )
    .addOption(
        new Option('-e, --env <environment>', 'Environment (testnet, mainnet, devnet-amplifier, stagenet)')
            .env('ENV')
            .makeOptionMandatory(true),
    )
    .addOption(new Option('-m, --mnemonic <mnemonic>', 'Mnemonic for signing').env('MNEMONIC').makeOptionMandatory(true))
    .option('-y, --yes', 'Skip confirmation prompts')
    .option('--deposit <deposit>', 'Proposal deposit amount', '1000000000')
    .option('--run-as <address>', 'Address to run the contract as')
    .action(async (options) => {
        try {
            const processedOptions = OptionProcessor.processOptions(options);
            const configManager = new ConfigManager(processedOptions.env);
            const instantiationManager = new InstantiationManager(configManager);
            await instantiationManager.instantiateChainContracts(processedOptions as InstantiateChainOptions);
        } catch (error) {
            printError('Error in CLI:', (error as Error).message);
            throw error;
        }
    });

if (require.main === module) {
    program.parse();
}
