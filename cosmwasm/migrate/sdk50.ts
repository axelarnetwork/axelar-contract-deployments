'use strict';

import { StdFee } from '@cosmjs/stargate';
import { Command } from 'commander';

import { printInfo, prompt } from '../../common';
import { ConfigManager } from '../../common/config';
import { addAmplifierOptions } from '../cli-utils';
import { ClientManager } from '../processor';
import { mainProcessor } from '../processor';
import { Options } from '../processor';
import { encodeMigrate, getCodeId, submitProposal } from '../utils';

interface MigrationOptions extends Options {
    title: string;
    description: string;
    deposit?: string;
    yes?: boolean;
    fetchCodeId?: boolean;
}

async function migrateAllVotingVerifiers(
    client: ClientManager,
    config: ConfigManager,
    options: MigrationOptions,
    _args: string[],
    fee: string | StdFee,
): Promise<void> {
    const { deposit, yes, fetchCodeId } = options;
    const chains = Object.entries(config.chains)
        .filter(([, chainConfig]) => chainConfig.contracts?.AxelarGateway?.connectionType === 'amplifier')
        .map(([chainName]) => chainName);
    const votingVerifiers: Array<{ chainName: string; address: string; codeId: number }> = [];
    const title = 'Migrate Voting Verifiers to update block time related parameters';
    const description = 'Migrate all voting verifiers to update block time related parameters';

    for (const chainName of chains) {
        const votingVerifierConfig = config.getVotingVerifierContract(chainName);
        const codeId = await getCodeId(client, config, {
            fetchCodeId,
            contractName: config.getVotingVerifierContractForChainType(chainName),
        });

        votingVerifierConfig.codeId = codeId;

        votingVerifiers.push({
            chainName,
            address: votingVerifierConfig.address,
            codeId,
        });
        printInfo(`Added ${chainName} voting verifier (address: ${votingVerifierConfig.address}, codeId: ${codeId})`);
    }

    printInfo(`Found ${votingVerifiers.length} voting verifier(s) to migrate`);

    const migrationMessages = votingVerifiers.map(({ chainName, address, codeId }) => ({
        chainName,
        message: encodeMigrate(config, {
            ...options,
            contractName: config.getVotingVerifierContractForChainType(chainName),
            chainName,
            address,
            codeId,
            msg: '{}',
        }),
    }));

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

    for (const { chainName, message } of migrationMessages) {
        const proposalId = await submitProposal(client, config, proposalOptions, message, fee);
        printInfo(`Migration proposal for chain ${chainName} submitted successfully: ${proposalId}`);
    }
}

async function updateBlockTimeRelatedParameters(
    client: ClientManager,
    config: ConfigManager,
    options: MigrationOptions,
    _args: string[],
    fee: string | StdFee,
): Promise<void> {
    const { yes } = options;
    const chains = Object.entries(config.chains)
        .filter(([, chainConfig]) => chainConfig.contracts?.AxelarGateway?.connectionType === 'amplifier')
        .map(([chainName]) => chainName);
    const title = 'Update block time related parameters for all voting verifiers';
    const description = 'Update block time related parameters for all voting verifiers';

    const votingVerifierMessages = await Promise.all(
        chains.map(async (chainName) => {
            const votingVerifierConfig = config.getVotingVerifierContract(chainName);
            config.validateRequired(votingVerifierConfig.address, 'votingVerifierConfig.address');

            const { block_expiry, confirmation_height, voting_threshold } = await client.queryContractSmart(
                votingVerifierConfig.address,
                'voting_parameters',
            );

            const msg = {
                update_voting_parameters: {
                    block_expiry: block_expiry * 5,
                    confirmation_height: confirmation_height * 5,
                    voting_threshold: voting_threshold * 5,
                },
            };
            printInfo(
                `Current voting parameters for ${chainName}: block_expiry: ${block_expiry}, confirmation_height: ${confirmation_height}, voting_threshold: ${voting_threshold}`,
            );
            printInfo(
                `New voting parameters for ${chainName}: block_expiry: ${msg.update_voting_parameters.block_expiry}, confirmation_height: ${msg.update_voting_parameters.confirmation_height}, voting_threshold: ${msg.update_voting_parameters.voting_threshold}`,
            );
            return {
                chainName,
                message: encodeMigrate(config, {
                    ...options,
                    contractName: config.getVotingVerifierContractForChainType(chainName),
                    chainName,
                    address: votingVerifierConfig.address,
                    codeId: votingVerifierConfig.codeId,
                    msg,
                }),
            };
        }),
    );

    const proposalOptions = {
        ...options,
        title,
        description,
    };

    if (prompt(`Proceed with migration of ${votingVerifierMessages.length} voting verifier(s)?`, yes)) {
        return;
    }

    for (const { chainName, message } of votingVerifierMessages) {
        const proposalId = await submitProposal(client, config, proposalOptions, message, fee);
        printInfo(`Migration proposal for chain ${chainName} submitted successfully: ${proposalId}`);
    }
}

async function updateSigningParametersForMultisig(
    client: ClientManager,
    config: ConfigManager,
    options: MigrationOptions,
    _args: string[],
    fee: string | StdFee,
): Promise<void> {
    const multisigConfig = config.getContractConfig('Multisig');
    config.validateRequired(multisigConfig.address, 'multisigConfig.address');

    const { block_expiry } = await client.queryContractSmart(multisigConfig.address, 'signing_parameters');
    printInfo(`Current signing parameters: block_expiry: ${block_expiry}`);

    const msg = {
        update_signing_parameters: {
            block_expiry: block_expiry * 5,
        },
    };

    printInfo(`New block expiry: ${msg.update_signing_parameters.block_expiry}`);

    const proposalOptions = {
        ...options,
        title: 'Update signing parameters for multisig',
        description: 'Update signing parameters for multisig',
        contractName: 'Multisig',
        msg,
    };

    if (prompt(`Proceed with migration of multisig?`, options.yes)) {
        return;
    }

    const migrationMessage = encodeMigrate(config, proposalOptions);
    const proposalId = await submitProposal(client, config, proposalOptions, migrationMessage, fee);
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

    const updateBlockTimeRelatedParametersCmd = program
        .command('update-voting-verifiers')
        .description('Update block time related parameters for all voting verifiers')
        .action((options) => {
            mainProcessor(updateBlockTimeRelatedParameters, options);
        });

    addAmplifierOptions(updateBlockTimeRelatedParametersCmd, {
        fetchCodeId: true,
        runAs: true,
    });

    const updateSigningParametersForMultisigCmd = program
        .command('update-signing-parameters-for-multisig')
        .description('Update signing parameters for multisig')
        .action((options) => {
            mainProcessor(updateSigningParametersForMultisig, options);
        });

    addAmplifierOptions(updateSigningParametersForMultisigCmd, {
        executeProposalOptions: true,
        runAs: true,
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
