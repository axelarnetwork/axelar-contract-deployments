'use strict';

import { StdFee } from '@cosmjs/stargate';
import { Command } from 'commander';

import { printInfo, printWarn, prompt } from '../../common';
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
                fetchCodeId: true,
                contractName: config.getVotingVerifierContractForChainType(chainName),
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

    const migrationMessages = votingVerifiers.map(({ chainName, address, codeId }) =>
        encodeMigrate(config, {
            ...options,
            contractName: config.getVotingVerifierContractForChainType(chainName),
            chainName,
            address,
            codeId,
            msg: '{}',
        }),
    );

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

    for (const [index, migrationMessage] of migrationMessages.entries()) {
        const proposalId = await submitProposal(client, config, proposalOptions, migrationMessage, fee);
        printInfo(`Migration proposal for ${votingVerifiers[index].chainName} submitted successfully: ${proposalId}`);
    }
}

async function updateBlockTimeRelatedParameters(
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
                `New voting parameters for ${chainName}: block_expiry: ${block_expiry * 5}, confirmation_height: ${confirmation_height * 5}, voting_threshold: ${voting_threshold * 5}`,
            );
            return encodeMigrate(config, {
                ...options,
                contractName: config.getVotingVerifierContractForChainType(chainName),
                chainName,
                address: votingVerifierConfig.address,
                codeId: votingVerifierConfig.codeId,
                msg,
            });
        }),
    );

    printInfo(`Prepared ${votingVerifierMessages.length} migration message(s) for the proposal`);

    const proposalOptions = {
        ...options,
        title,
        description,
        deposit,
    };

    if (prompt(`Proceed with migration of ${votingVerifierMessages.length} voting verifier(s)?`, yes)) {
        return;
    }

    for (const [index, votingVerifierMessage] of votingVerifierMessages.entries()) {
        const proposalId = await submitProposal(client, config, proposalOptions, [votingVerifierMessage], fee);
        printInfo(`Migration proposal for ${chains[index]} submitted successfully: ${proposalId}`);
    }
}

async function updateBlockTimeRelatedParametersForMultisig(
    client: ClientManager,
    config: ConfigManager,
    options: MigrationOptions,
    _args: string[],
    fee: string | StdFee,
): Promise<void> {
    const multisigConfig = config.getContractConfig('Multisig');
    config.validateRequired(multisigConfig.address, 'multisigConfig.address');

    // TODO tkulik: align with the actual contract API once it is implemented
    const currentBlockExpiry = await client.queryContractSmart(multisigConfig.address, 'block_expiry');
    printInfo(`Current block expiry: ${currentBlockExpiry}`);

    // TODO tkulik: align with the actual contract API once it is implemented
    const msg = {
        update_block_expiry: {
            new_block_expiry: currentBlockExpiry * 5,
        },
    };

    printInfo(`New block expiry: ${currentBlockExpiry * 5}`);

    const proposalOptions = {
        ...options,
        title: 'Update block time related parameters for multisig',
        description: 'Update block time related parameters for multisig',
        deposit: options.deposit,
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

    const updateBlockTimeRelatedParametersForMultisigCmd = program
        .command('update-block-time-related-parameters-for-multisig')
        .description('Update block time related parameters for multisig')
        .action((options) => {
            mainProcessor(updateBlockTimeRelatedParametersForMultisig, options);
        });

    addAmplifierOptions(updateBlockTimeRelatedParametersForMultisigCmd, {
        executeProposalOptions: true,
        runAs: true,
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
