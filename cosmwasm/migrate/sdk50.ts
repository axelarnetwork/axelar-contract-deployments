'use strict';

import { StdFee } from '@cosmjs/stargate';
import { Command } from 'commander';

import { printInfo, prompt } from '../../common';
import { ConfigManager } from '../../common/config';
import { addAmplifierOptions } from '../cli-utils';
import { ClientManager } from '../processor';
import { mainProcessor } from '../processor';
import { Options } from '../processor';
import { encodeExecuteContract, encodeMigrate, getCodeId, submitProposal } from '../utils';

interface MigrationOptions extends Options {
    title?: string;
    description?: string;
    deposit?: string;
    yes?: boolean;
    fetchCodeId?: boolean;
    codeId?: number;
    runAs?: string;
}

async function migrateAllVotingVerifiers(
    client: ClientManager,
    config: ConfigManager,
    options: MigrationOptions,
    _args: string[],
    fee: string | StdFee,
): Promise<void> {
    const { fetchCodeId } = options;
    const chains = Object.entries(config.chains)
        .filter(([, chainConfig]) => chainConfig.contracts?.AxelarGateway?.connectionType === 'amplifier')
        .map(([chainName]) => chainName);
    const votingVerifiers: Array<{ chainName: string; address: string; codeId: number }> = [];
    options.title = options.title || 'Migrate Voting Verifiers to update block time related parameters';
    options.description = options.description || 'Migrate all voting verifiers to update block time related parameters';

    for (const chainName of chains) {
        const chainConfig = config.getChainConfig(chainName);
        const votingVerifierConfig = config.getVotingVerifierContract(chainName);
        config.validateRequired(votingVerifierConfig.address, 'votingVerifierConfig.address');
        const codeId = await getCodeId(client, config, {
            ...options,
            contractName: config.getVotingVerifierContractForChainType(chainConfig.chainType),
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

    const migrationMessages = votingVerifiers.map(({ chainName, address, codeId }) => {
        const chainConfig = config.getChainConfig(chainName);
        return {
            chainName,
            message: encodeMigrate(config, {
                ...options,
                contractName: config.getVotingVerifierContractForChainType(chainConfig.chainType),
                chainName,
                address,
                codeId,
                msg: '{}',
            }),
        };
    });

    printInfo(`Prepared ${migrationMessages.length} migration message(s) for the proposal`);

    for (const { chainName, message } of migrationMessages) {
        if (prompt(`Proceed with migration of voting verifier for chain ${chainName}?`)) {
            continue;
        }
        const proposalId = await submitProposal(client, config, options, message, fee);
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
    const chains = Object.entries(config.chains)
        .filter(([, chainConfig]) => chainConfig.contracts?.AxelarGateway?.connectionType === 'amplifier')
        .map(([chainName]) => chainName);
    options.title = options.title || 'Update block time related parameters for all voting verifiers';
    options.description = options.description || 'Update block time related parameters for all voting verifiers';

    const votingVerifierMessages = await Promise.all(
        chains.map(async (chainName) => {
            const chainConfig = config.getChainConfig(chainName);
            const votingVerifierConfig = config.getVotingVerifierContract(chainName);
            config.validateRequired(votingVerifierConfig.address, 'votingVerifierConfig.address');

            const { block_expiry, confirmation_height } = await client.queryContractSmart(
                votingVerifierConfig.address,
                'voting_parameters',
            );

            const msg = {
                update_voting_parameters: {
                    block_expiry: votingVerifierConfig.blockExpiry,
                    confirmation_height: votingVerifierConfig.confirmationHeight,
                },
            };
            printInfo(
                `Current voting parameters for ${chainName}: block_expiry: ${block_expiry}, confirmation_height: ${confirmation_height}`,
            );
            printInfo(
                `New voting parameters for ${chainName}: block_expiry: ${msg.update_voting_parameters.block_expiry}, confirmation_height: ${msg.update_voting_parameters.confirmation_height}`,
            );
            return {
                chainName,
                message: encodeExecuteContract(
                    config,
                    {
                        ...options,
                        contractName: config.getVotingVerifierContractForChainType(chainConfig.chainType),
                        msg: JSON.stringify(msg),
                    },
                    chainName,
                ),
            };
        }),
    );

    for (const { chainName, message } of votingVerifierMessages) {
        if (prompt(`Proceed with updating block time related parameters for chain ${chainName}?`)) {
            continue;
        }
        const proposalId = await submitProposal(client, config, options, message, fee);
        printInfo(`Update block time parameters proposal for chain ${chainName} submitted successfully: ${proposalId}`);
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
    config.validateRequired(multisigConfig.blockExpiry, 'multisigConfig.blockExpiry', 'number');
    options.title = options.title || 'Update signing parameters for multisig';
    options.description = options.description || 'Update signing parameters for multisig';

    const { block_expiry } = await client.queryContractSmart(multisigConfig.address, 'signing_parameters');
    printInfo(`Current signing parameters: block_expiry: ${block_expiry}`);

    const msg = {
        update_signing_parameters: {
            block_expiry: multisigConfig.blockExpiry,
        },
    };

    printInfo(`New block expiry: ${msg.update_signing_parameters.block_expiry}`);

    const proposalOptions = {
        ...options,
        contractName: 'Multisig',
        msg: JSON.stringify(msg),
    };

    if (prompt(`Proceed with updating signing parameters for multisig?`)) {
        return;
    }

    const migrationMessage = encodeExecuteContract(config, proposalOptions, undefined);
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
