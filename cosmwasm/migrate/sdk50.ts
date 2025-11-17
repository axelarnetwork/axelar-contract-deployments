'use strict';

import { StdFee } from '@cosmjs/stargate';
import { Command } from 'commander';

import { printInfo, printWarn } from '../../common';
import { ConfigManager } from '../../common/config';
import { VERIFIER_CONTRACT_NAME } from '../../common/config';
import { addAmplifierOptions } from '../cli-utils';
import { ClientManager } from '../processor';
import { mainProcessor } from '../processor';
import { Options } from '../processor';
import { encodeMigrate, getAmplifierContractConfig, getCodeId, submitProposal } from '../utils';

interface MigrationOptions extends Options {
    title: string;
    description: string;
    deposit?: string;
    yes?: boolean;
}

async function migrateAllVotingVerifiers(
    client: ClientManager,
    config: ConfigManager,
    options: MigrationOptions,
    _args: string[],
    fee: string | StdFee,
): Promise<void> {
    const { deposit, yes } = options;
    const chains = Object.keys(config.chains);
    const votingVerifiers: Array<{ chainName: string; address: string; codeId: number }> = [];

    const title = 'Migrate Voting Verifiers';
    const description = 'Migrate all voting verifiers to the new version';

    // Collect all voting verifiers that have addresses
    for (const chainName of chains) {
        try {
            const votingVerifierConfig = config.getVotingVerifierContract(chainName);
            if (votingVerifierConfig.address) {
                // Get codeId - check chain-specific config first, then try getCodeId, or use options
                let codeId: number;

                // First, check if codeId is in the chain-specific config
                if (votingVerifierConfig.codeId) {
                    codeId = votingVerifierConfig.codeId;
                    printInfo(`Using codeId from config for ${chainName}: ${codeId}`);
                } else {
                    // Try to get codeId using the utility function
                    try {
                        codeId = await getCodeId(client, config, {
                            ...options,
                            contractName: VERIFIER_CONTRACT_NAME,
                            chainName,
                        });
                        // Update the config with the fetched codeId
                        votingVerifierConfig.codeId = codeId;
                        printInfo(`Fetched codeId for ${chainName}: ${codeId}`);
                    } catch (error) {
                        throw new Error(
                            `CodeId not found for ${chainName}. Use --codeId or --fetchCodeId option, or set codeId in config. Error: ${error instanceof Error ? error.message : String(error)}`,
                        );
                    }
                }

                votingVerifiers.push({
                    chainName,
                    address: votingVerifierConfig.address,
                    codeId,
                });
                printInfo(`Added ${chainName} voting verifier (address: ${votingVerifierConfig.address}, codeId: ${codeId})`);
            } else {
                printWarn(`Skipping ${chainName}: VotingVerifier address not found`);
            }
        } catch (error) {
            // Chain doesn't have a VotingVerifier contract configured, skip it
            printWarn(`Skipping ${chainName}: ${error instanceof Error ? error.message : String(error)}`);
        }
    }

    if (votingVerifiers.length === 0) {
        throw new Error('No voting verifiers found with addresses configured');
    }

    printInfo(`Found ${votingVerifiers.length} voting verifier(s) to migrate`);

    // Create migration messages for all voting verifiers
    const migrationMessages = votingVerifiers.map(({ chainName, address, codeId }) => {
        const { contractConfig } = getAmplifierContractConfig(config, {
            ...options,
            contractName: VERIFIER_CONTRACT_NAME,
            chainName,
        });
        // Update the codeId in config for this chain
        contractConfig.codeId = codeId;

        const msg = {};

        return encodeMigrate(config, {
            ...options,
            contractName: VERIFIER_CONTRACT_NAME,
            chainName,
            address,
            codeId,
            msg,
        });
    });

    printInfo(`Prepared ${migrationMessages.length} migration message(s) for the proposal`);

    // Submit the proposal with all migrations
    const proposalOptions = {
        ...options,
        title,
        description,
        deposit: deposit || config.getProposalDepositAmount(),
    };

    if (prompt(`Proceed with migration of ${migrationMessages} voting verifiers?`, yes ? 'y' : '')) {
        return;
    }

    const proposalId = await submitProposal(client, config, proposalOptions, migrationMessages, fee);
    printInfo('Migration proposal submitted successfully', proposalId);
}

const programHandler = () => {
    const program = new Command();

    program.name('sdk50').description('SDK 50 migration and configuration helpers');

    const migrateVotingVerifiersCmd = program
        .command('migrate-voting-verifiers')
        .description('Migrate all voting verifiers')
        .action((options) => {
            mainProcessor(migrateAllVotingVerifiers, options);
        });

    addAmplifierOptions(migrateVotingVerifiersCmd, {
        codeId: true,
        fetchCodeId: true,
        runAs: true,
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
