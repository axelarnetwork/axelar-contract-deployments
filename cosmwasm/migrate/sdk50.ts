'use strict';

import { StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';

import { addOptionsToCommands, getAmplifierChains, printInfo, printWarn, prompt } from '../../common';
import { ConfigManager } from '../../common/config';
import { addAmplifierOptions } from '../cli-utils';
import { ClientManager, Options, mainProcessor } from '../processor';
import { executeByGovernance, migrate, submitMessagesAsProposal } from '../proposal-utils';

// eslint-disable-next-line @typescript-eslint/no-require-imports
const { encodeMigrate, getCodeId } = require('../utils');

interface MigrationOptions extends Options {
    title?: string;
    description?: string;
    deposit?: string;
    yes?: boolean;
    fetchCodeId?: boolean;
    codeId?: number;
    dryRun?: boolean;
    newVersion?: string;
    [key: string]: unknown;
}

async function migrateAllVotingVerifiers(
    client: ClientManager,
    config: ConfigManager,
    options: MigrationOptions,
    _args: string[],
    fee: string | StdFee,
): Promise<void> {
    const chains = getAmplifierChains(config.chains);
    const votingVerifiers: Array<{ chainName: string; address: string; contractName: string; chainCodecAddress: string }> = [];

    for (const { name: chainName, config: chainConfig } of chains) {
        let votingVerifierConfig;
        let contractName;
        let chainCodecAddress;
        try {
            votingVerifierConfig = config.getVotingVerifierContract(chainName);
            contractName = config.getVotingVerifierContractForChainType(chainConfig.chainType);
            config.validateRequired(votingVerifierConfig.address, 'votingVerifierConfig.address');
        } catch (error) {
            printWarn(`Skipping ${chainName}: ${error instanceof Error ? error.message : error}`);
            continue;
        }

        if (contractName !== 'VotingVerifier') {
            printWarn(`Skipping ${chainName}: uses ${contractName}, which requires a dedicated migration flow`);
            continue;
        }

        try {
            chainCodecAddress = config.getChainCodecAddress(chainConfig.chainType);
        } catch (error) {
            printWarn(`Skipping ${chainName}: ${error instanceof Error ? error.message : error}`);
            continue;
        }

        votingVerifiers.push({
            chainName,
            address: votingVerifierConfig.address,
            contractName,
            chainCodecAddress,
        });
        printInfo(
            `Added ${chainName} voting verifier (address: ${votingVerifierConfig.address}, chain_codec_address: ${chainCodecAddress})`,
        );
    }

    printInfo(`Found ${votingVerifiers.length} voting verifier(s) to migrate`);

    for (const { chainName, address, contractName, chainCodecAddress } of votingVerifiers) {
        try {
            printInfo(`Proceeding with migration of voting verifier for chain ${chainName}...`);
            await migrate(
                client,
                config,
                {
                    ...options,
                    title: `Migrate Voting Verifier to v2.0.2 for chain ${chainName}`,
                    description: `Migrate Voting Verifier to v2.0.2 for chain ${chainName}`,
                    contractName,
                    address,
                    msg: JSON.stringify({ chain_codec_address: chainCodecAddress }),
                },
                undefined,
                fee,
            );
        } catch (error) {
            printWarn(`Error migrating voting verifier for chain ${chainName}: ${error}, skipping...`);
        }
    }
}

async function migrateAllVotingVerifiersBatched(
    client: ClientManager,
    config: ConfigManager,
    options: MigrationOptions,
    _args: string[],
    fee: string | StdFee,
): Promise<void> {
    const chains = getAmplifierChains(config.chains);
    const targets: Array<{ chainName: string; address: string; contractName: string; chainCodecAddress: string }> = [];

    if (typeof options.codeId !== 'number' && !options.fetchCodeId) {
        throw new Error(
            'migrate-voting-verifiers-batch requires either --codeId <N> or --fetchCodeId so the target code id is unambiguous',
        );
    }
    // Resolve the target code id ONCE up-front. With --fetchCodeId this also
    // populates VotingVerifier.lastUploadedCodeId as a side effect.
    const resolvedCodeId: number = await getCodeId(client, config, {
        contractName: 'VotingVerifier',
        codeId: options.codeId,
        fetchCodeId: options.fetchCodeId,
    });
    printInfo(`Target code id resolved`, String(resolvedCodeId));

    for (const { name: chainName, config: chainConfig } of chains) {
        let votingVerifierConfig;
        let contractName;
        let chainCodecAddress;
        try {
            votingVerifierConfig = config.getVotingVerifierContract(chainName);
            contractName = config.getVotingVerifierContractForChainType(chainConfig.chainType);
            config.validateRequired(votingVerifierConfig.address, 'votingVerifierConfig.address');
        } catch (error) {
            printWarn(`Skipping ${chainName}: ${error instanceof Error ? error.message : error}`);
            continue;
        }

        if (contractName !== 'VotingVerifier') {
            printWarn(`Skipping ${chainName}: uses ${contractName}, which requires a dedicated migration flow`);
            continue;
        }

        try {
            chainCodecAddress = config.getChainCodecAddress(chainConfig.chainType);
        } catch (error) {
            printWarn(`Skipping ${chainName}: ${error instanceof Error ? error.message : error}`);
            continue;
        }

        try {
            const { codeId: currentCodeId } = await client.getContract(votingVerifierConfig.address);
            if (currentCodeId === resolvedCodeId) {
                printWarn(`Skipping ${chainName}: already on code id ${resolvedCodeId}`);
                continue;
            }
        } catch (error) {
            printWarn(
                `Could not query current code id for ${chainName} (${error instanceof Error ? error.message : error}), including in batch anyway`,
            );
        }

        targets.push({
            chainName,
            address: votingVerifierConfig.address,
            contractName,
            chainCodecAddress,
        });
        printInfo(
            `Included ${chainName} voting verifier (address: ${votingVerifierConfig.address}, chain_codec_address: ${chainCodecAddress})`,
        );
    }

    if (targets.length === 0) {
        printInfo('No voting verifiers to migrate; exiting');
        return;
    }

    if (options.dryRun) {
        printInfo(`[DRY-RUN] Would submit ONE bundled proposal with ${targets.length} migration(s):`);
        targets.forEach((t, i) => {
            printInfo(`  ${i + 1}. ${t.chainName}: ${t.address} -> code ${resolvedCodeId} (codec ${t.chainCodecAddress})`);
        });
        const versionNote = options.newVersion ? `, version -> "${options.newVersion}"` : '';
        printInfo(
            `[DRY-RUN] On successful submission, would mutate the chains config: per-chain codeId -> ${resolvedCodeId}${versionNote} for those ${targets.length} chains, and VotingVerifier.lastUploadedCodeId -> ${resolvedCodeId}`,
        );
        return;
    }

    printInfo(`Bundling ${targets.length} VotingVerifier migration(s) into a single proposal`);

    const messages = targets.map(({ address, contractName, chainCodecAddress }) =>
        encodeMigrate(config, {
            ...options,
            contractName,
            address,
            codeId: resolvedCodeId,
            msg: JSON.stringify({ chain_codec_address: chainCodecAddress }),
        }),
    );

    const title = options.title || `Migrate VotingVerifier to code id ${resolvedCodeId} on ${targets.length} chains`;
    const description =
        options.description ||
        `Bundled MsgMigrateContract for ${targets.length} amplifier chains: ${targets.map((t) => t.chainName).join(', ')}`;

    const proposalId = await submitMessagesAsProposal(client, config, { ...options, title, description }, messages, fee);

    if (proposalId) {
        for (const { chainName } of targets) {
            const vv = config.getVotingVerifierContract(chainName);
            vv.codeId = resolvedCodeId;
            if (options.newVersion) {
                vv.version = options.newVersion;
            }
        }
        const baseVv = config.getContractConfig('VotingVerifier');
        baseVv.lastUploadedCodeId = resolvedCodeId;
        printInfo(
            `Updated in-memory config for ${targets.length} chains (codeId -> ${resolvedCodeId}${options.newVersion ? `, version -> "${options.newVersion}"` : ''}) and VotingVerifier.lastUploadedCodeId -> ${resolvedCodeId}; saveConfig will persist on exit. NOTE: this is optimistic — if the proposal fails at execution, revert manually.`,
        );
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

    const votingVerifierMessages = (
        await Promise.all(
            chains.map(async ({ name: chainName, config: chainConfig }) => {
                try {
                    const votingVerifierConfig = config.getVotingVerifierContract(chainName);
                    config.validateRequired(votingVerifierConfig.address, 'votingVerifierConfig.address');

                    const { block_expiry } = await client.queryContractSmart(votingVerifierConfig.address, 'voting_parameters');

                    const message = {
                        update_voting_parameters: {
                            block_expiry: String(votingVerifierConfig.blockExpiry),
                        },
                    };

                    if (String(votingVerifierConfig.blockExpiry) === block_expiry) {
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
            printInfo(`Proceeding with updating block-expiry parameter for chain ${chainName}...`);
            await executeByGovernance(
                client,
                config,
                {
                    ...options,
                    title: `Update block-expiry parameterfor chain ${chainName}`,
                    description: `Update block-expiry parameter for chain ${chainName}`,
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

    const { block_expiry } = await client.queryContractSmart(multisigConfig.address, 'signing_parameters');
    printInfo(`Current signing parameters: block_expiry: ${block_expiry}. New proposed block_expiry: ${multisigConfig.blockExpiry}`);

    const msg = {
        update_signing_parameters: {
            block_expiry: String(multisigConfig.blockExpiry),
        },
    };

    printInfo(`Proceeding with updating block-expiry parameter for Multisig...`);

    await executeByGovernance(
        client,
        config,
        {
            ...options,
            title: `Update block-expiry parameter for Multisig`,
            description: `Update block-expiry parameter for Multisig`,
            contractName: 'Multisig',
            address: multisigConfig.address,
            msg: JSON.stringify(msg),
        },
        undefined,
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

    const migrateVotingVerifiersBatchedCmd = program
        .command('migrate-voting-verifiers-batch')
        .description('Migrate all voting verifiers in a single bundled governance proposal')
        .action((options) => {
            mainProcessor(migrateAllVotingVerifiersBatched, options);
        });

    addAmplifierOptions(migrateVotingVerifiersBatchedCmd, {
        codeId: true,
        fetchCodeId: true,
    });
    migrateVotingVerifiersBatchedCmd.addOption(new Option('--dryRun', 'preview the bundled proposal without submitting').env('DRY_RUN'));
    migrateVotingVerifiersBatchedCmd.addOption(
        new Option(
            '--newVersion <ver>',
            'version string to write into the chains config for each migrated chain (e.g. "2.0.2")',
        ).makeOptionMandatory(true),
    );

    program
        .command('update-voting-verifiers')
        .description('Update block-expiry parameter for all voting verifiers')
        .action((options) => {
            mainProcessor(updateBlockTimeRelatedParameters, options);
        });

    program
        .command('update-signing-parameters-for-multisig')
        .description('Update block-expiry parameter for Multisig')
        .action((options) => {
            mainProcessor(updateSigningParametersForMultisig, options);
        });

    addOptionsToCommands(program, addAmplifierOptions, {});

    program.parse();
};

if (require.main === module) {
    programHandler();
}
