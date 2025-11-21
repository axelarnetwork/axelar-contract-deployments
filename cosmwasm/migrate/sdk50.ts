'use strict';

import { StdFee } from '@cosmjs/stargate';
import { Command } from 'commander';

import { addOptionsToCommands, getAmplifierChains, printInfo, printWarn, prompt } from '../../common';
import { ConfigManager } from '../../common/config';
import { addAmplifierOptions } from '../cli-utils';
import { ClientManager, Options, mainProcessor } from '../processor';
import { execute, migrate } from '../submit-proposal';

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
    const chains = getAmplifierChains(config.chains);
    const votingVerifiers: Array<{ chainName: string; address: string; contractName: string }> = [];
    options.title = options.title || 'Migrate Voting Verifiers to update block time related parameters';
    options.description = options.description || 'Migrate all voting verifiers to update block time related parameters';

    for (const { name: chainName, config: chainConfig } of chains) {
        const votingVerifierConfig = config.getVotingVerifierContract(chainName);
        const contractName = config.getVotingVerifierContractForChainType(chainConfig.chainType);
        config.validateRequired(votingVerifierConfig.address, 'votingVerifierConfig.address');

        votingVerifiers.push({
            chainName,
            address: votingVerifierConfig.address,
            contractName,
        });
        printInfo(`Added ${chainName} voting verifier (address: ${votingVerifierConfig.address})`);
    }

    printInfo(`Found ${votingVerifiers.length} voting verifier(s) to migrate`);

    for (const { chainName, address, contractName } of votingVerifiers) {
        try {
            printInfo(`Proceeding with migration of voting verifier for chain ${chainName}...`);
            await migrate(
                client,
                config,
                {
                    ...options,
                    contractName,
                    address,
                    msg: JSON.stringify({}),
                },
                undefined,
                fee,
            );
        } catch (error) {
            printWarn(`Error migrating voting verifier for chain ${chainName}: ${error}, skipping...`);
        }
    }
}

async function updateBlockTimeRelatedParameters(
    client: ClientManager,
    config: ConfigManager,
    options: MigrationOptions,
    _args: string[],
    fee: string | StdFee,
): Promise<void> {
    const chains = getAmplifierChains(config.chains);
    options.title = options.title || 'Update block time related parameters for all voting verifiers';
    options.description = options.description || 'Update block time related parameters for all voting verifiers';

    const votingVerifierMessages = (
        await Promise.all(
            chains.map(async ({ name: chainName, config: chainConfig }) => {
                try {
                    const votingVerifierConfig = config.getVotingVerifierContract(chainName);
                    config.validateRequired(votingVerifierConfig.address, 'votingVerifierConfig.address');

                    const { block_expiry } = await client.queryContractSmart(votingVerifierConfig.address, 'voting_parameters');

                    const message = {
                        update_voting_parameters: {
                            block_expiry: votingVerifierConfig.blockExpiry,
                            confirmation_height: null,
                            voting_threshold: null,
                        },
                    };

                    if (votingVerifierConfig.blockExpiry === block_expiry) {
                        printInfo(`Block expiry for ${chainName} is already up to date, skipping...`);
                        return undefined;
                    }

                    printInfo(
                        `Current voting parameters for ${chainName}: block_expiry: ${block_expiry}. New proposed block_expiry: ${message.update_voting_parameters.block_expiry}`,
                    );

                    const contractName = config.getVotingVerifierContractForChainType(chainConfig.chainType);

                    return {
                        chainName,
                        contractName,
                        address: votingVerifierConfig.address,
                        message,
                    };
                } catch (error) {
                    printWarn(`Error getting voting parameters for chain ${chainName}: ${error}, skipping...`);
                    return undefined;
                }
            }),
        )
    ).filter(Boolean);

    for (const { chainName, contractName, address, message } of votingVerifierMessages) {
        try {
            printInfo(`Proceeding with updating block time related parameters for chain ${chainName}...`);
            await execute(
                client,
                config,
                {
                    ...options,
                    contractName,
                    address,
                    chainName,
                    msg: JSON.stringify(message),
                },
                undefined,
                fee,
            );
        } catch (error) {
            printWarn(`Error updating block time related parameters for chain ${chainName}: ${error}, skipping...`);
        }
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
    config.validateRequired(multisigConfig.address, 'axelar.contracts.Multisig.address', 'string');
    config.validateRequired(multisigConfig.blockExpiry, 'axelar.contracts.Multisig.blockExpiry', 'number');
    options.title = options.title || 'Update signing parameters for multisig';
    options.description = options.description || 'Update signing parameters for multisig';

    const { block_expiry } = await client.queryContractSmart(multisigConfig.address, { signing_parameters: {} });
    printInfo(`Current signing parameters: block_expiry: ${block_expiry}. New proposed block_expiry: ${multisigConfig.blockExpiry}`);

    const msg = {
        update_signing_parameters: {
            block_expiry: multisigConfig.blockExpiry,
        },
    };

    printInfo(`Proceeding with updating signing parameters for multisig...`);

    await execute(
        client,
        config,
        {
            ...options,
            contractName: 'Multisig',
            msg: JSON.stringify(msg),
        },
        fee,
    );
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
    });

    program
        .command('update-voting-verifiers')
        .description('Update block time related parameters for all voting verifiers')
        .action((options) => {
            mainProcessor(updateBlockTimeRelatedParameters, options);
        });

    program
        .command('update-signing-parameters-for-multisig')
        .description('Update signing parameters for multisig')
        .action((options) => {
            mainProcessor(updateSigningParametersForMultisig, options);
        });

    addOptionsToCommands(program, addAmplifierOptions, { runAs: true });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
