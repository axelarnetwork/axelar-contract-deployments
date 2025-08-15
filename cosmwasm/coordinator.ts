#!/usr/bin/env ts-node
import { Command } from 'commander';

import { printError } from '../common';
import { ConfigManager } from './coordinator/config';
import { DeploymentManager } from './coordinator/deployment';
import { GovernanceManager } from './coordinator/governance';
import { InstantiationManager } from './coordinator/instantiation';
import { OptionProcessor } from './coordinator/option-processor';

const program = new Command();

program.name('coordinator').description('Submit governance proposal to instantiate chain contracts using Coordinator');

program
    .command('deploy')
    .description('Deploy VotingVerifier, MultisigProver, and Gateway contracts without instantiating them')
    .option('-e, --env <environment>', 'Environment (testnet, mainnet, devnet-amplifier, stagenet)')
    .option('-m, --mnemonic <mnemonic>', 'Mnemonic for signing (or set MNEMONIC environment variable)')
    .option('-y, --yes', 'Skip confirmation prompts')
    .option('--deposit <deposit>', 'Proposal deposit amount', '1000000000')
    .option('--run-as <address>', 'Address to run the contract as')
    .option('--artifact-dir <path>', 'Path to contract artifacts directory')
    .option('--version <version>', 'Contract version for artifact downloading (e.g., v1.0.0 or commit hash)')
    .option('--title <title>', 'Proposal title')
    .option('--description <description>', 'Proposal description')
    .action(async (options) => {
        try {
            const processedOptions = OptionProcessor.processOptions(options);
            const configManager = new ConfigManager(processedOptions.env);
            const deploymentManager = new DeploymentManager(configManager);
            await deploymentManager.deployContracts(processedOptions);
        } catch (error) {
            printError('Error in CLI:', (error as Error).message);
            throw error;
        }
    });

program
    .command('register-protocol')
    .description('Submit governance proposal to register protocol contracts with Coordinator')
    .option('-e, --env <environment>', 'Environment (testnet, mainnet, devnet-amplifier, stagenet)')
    .option('-m, --mnemonic <mnemonic>', 'Mnemonic for signing (or set MNEMONIC environment variable)')
    .option('-y, --yes', 'Skip confirmation prompts')
    .option('--deposit <deposit>', 'Proposal deposit amount', '1000000000')
    .option('--run-as <address>', 'Address to run the contract as')
    .option('--title <title>', 'Proposal title')
    .option('--description <description>', 'Proposal description')
    .action(async (options) => {
        try {
            const processedOptions = OptionProcessor.processOptions(options);
            const configManager = new ConfigManager(processedOptions.env);
            const governanceManager = new GovernanceManager(configManager);
            await governanceManager.registerProtocol(processedOptions);
        } catch (error) {
            printError('Error in CLI:', (error as Error).message);
            throw error;
        }
    });

program
    .command('register-deployment')
    .description('Submit governance proposal to register a new deployment with Coordinator')
    .option('-n, --chain <chain>', 'Chain name (e.g., avalanche, ethereum-sepolia, celo)')
    .option('-e, --env <environment>', 'Environment (testnet, mainnet, devnet-amplifier, stagenet)')
    .option('-m, --mnemonic <mnemonic>', 'Mnemonic for signing (or set MNEMONIC environment variable)')
    .option('-y, --yes', 'Skip confirmation prompts')
    .option('--deposit <deposit>', 'Proposal deposit amount', '1000000000')
    .option('--run-as <address>', 'Address to run the contract as')
    .option('--title <title>', 'Proposal title')
    .option('--description <description>', 'Proposal description')
    .action(async (options) => {
        try {
            const processedOptions = OptionProcessor.processOptions(options);
            const configManager = new ConfigManager(processedOptions.env);
            const governanceManager = new GovernanceManager(configManager);
            await governanceManager.registerDeployment(processedOptions, processedOptions.chain);
        } catch (error) {
            printError('Error in CLI:', (error as Error).message);
            throw error;
        }
    });

program
    .command('instantiate')
    .description('Submit governance proposal to instantiate chain contracts using Coordinator')
    .requiredOption('--contract-admin <address>', 'Admin address')
    .requiredOption('--multisig-admin <address>', 'Multisig admin address passed to the multisigProver contract')
    .option('-n, --chain <chain>', 'Chain name (e.g., ethereum-sepolia, celo)')
    .option('-e, --env <environment>', 'Environment (testnet, mainnet, devnet-amplifier, stagenet)')
    .option('-m, --mnemonic <mnemonic>', 'Mnemonic for signing (or set MNEMONIC environment variable)')
    .option('-y, --yes', 'Skip confirmation prompts')
    .option('--deposit <deposit>', 'Proposal deposit amount', '1000000000')
    .option('--run-as <address>', 'Address to run the contract as')
    .option('--salt <salt>', 'Custom salt for deployment (optional, will generate if not provided)')
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
    .option('--title <title>', 'Proposal title')
    .option('--description <description>', 'Proposal description')
    .action(async (options) => {
        try {
            const processedOptions = OptionProcessor.processOptions(options);
            const configManager = new ConfigManager(processedOptions.env);
            const instantiationManager = new InstantiationManager(configManager);
            await instantiationManager.instantiateChainContracts(processedOptions.chain, processedOptions);
        } catch (error) {
            printError('Error in CLI:', (error as Error).message);
            throw error;
        }
    });

if (require.main === module) {
    program.parse();
}
