'use strict';

import { StdFee } from '@cosmjs/stargate';
import { Command } from 'commander';

import { printInfo, printWarn, prompt } from '../../common';
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
    const chains = Object.entries(config.chains)
        .filter(([, chainConfig]) => chainConfig.contracts?.AxelarGateway?.connectionType === 'amplifier')
        .map(([chainName]) => chainName);
    const votingVerifiers: Array<{ chainName: string; address: string; codeId: number }> = [];
    const title = 'Migrate Voting Verifiers to update block time related parameters';
    const description = 'Migrate all voting verifiers to update block time related parameters';

    for (const chainName of chains) {
        try {
            const votingVerifierConfig = config.getVotingVerifierContract(chainName);
            const codeId = await getCodeId(client, config, {
                ...options,
                contractName: config.getVotingVerifierContractForChainType(chainName),
                chainName,
            });
            votingVerifierConfig.codeId = codeId;
            printInfo(`Using codeId from config for ${chainName}: ${codeId}`);

            votingVerifiers.push({
                chainName,
                address: votingVerifierConfig.address,
                codeId,
            });
            printInfo(`Added ${chainName} voting verifier (address: ${votingVerifierConfig.address}, codeId: ${codeId})`);
        } catch (error) {
            printWarn(`Skipping ${chainName}: ${error}`);
        }
    }

    printInfo(`Found ${votingVerifiers.length} voting verifier(s) to migrate`);

    const migrationMessages = votingVerifiers.map(({ chainName, address, codeId }) => {
        const { contractConfig } = getAmplifierContractConfig(config, {
            ...options,
            contractName: config.getVotingVerifierContractForChainType(chainName),
            chainName,
        });

        contractConfig.codeId = codeId;

        const msg = '{}';

        return encodeMigrate(config, {
            ...options,
            contractName: config.getVotingVerifierContractForChainType(chainName),
            chainName,
            address,
            codeId,
            msg,
        });
    });

    printInfo(`Prepared ${migrationMessages.length} migration message(s) for the proposal`);

    const proposalOptions = {
        ...options,
        title,
        description,
        deposit,
    };

    if (prompt(`Proceed with migration of ${migrationMessages.length} voting verifier(s)?`, yes)) {
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
