'use strict';

import { CosmWasmClient, SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';

import { AxelarContractConfig, ConfigManager } from '../../common/config';
import { getAmplifierChains, printError, printHighlight, printInfo, printWarn, prompt, validateParameters } from '../../common/utils';
import { addAmplifierOptions } from '../cli-utils';
import { ClientManager, Options, mainProcessor, mainQueryProcessor } from '../processor';
import { RewardsPoolResponse, queryRewardsPool } from '../query';
import { confirmProposalSubmission } from '../submit-proposal';
import { GOVERNANCE_MODULE_ADDRESS, encodeExecuteContract, submitProposal } from '../utils';

interface PoolParams {
    chainName: string;
    contractAddress: string;
    contractType: 'Multisig' | 'VotingVerifier';
    epoch_duration: string;
    participation_threshold: [string, string];
    rewards_per_epoch: string;
}

interface UpdatePoolParamsMessage {
    update_pool_params: {
        params: {
            epoch_duration: string;
            rewards_per_epoch: string;
            participation_threshold: [string, string];
        };
        pool_id: {
            chain_name: string;
            contract: string;
        };
    };
}

async function queryAllRewardsPools(client: CosmWasmClient, configManager: ConfigManager): Promise<PoolParams[]> {
    const poolParams: PoolParams[] = [];
    const rewardsConfig = configManager.getContractConfig('Rewards');
    const rewardsAddress = configManager.validateRequired(rewardsConfig.address, 'Rewards.address');

    const amplifierChains = getAmplifierChains(configManager.chains);

    if (amplifierChains.length === 0) {
        throw new Error('No amplifier chains found');
    }

    const multisigConfig = configManager.getContractConfig('Multisig');
    const multisigAddress = configManager.validateRequired(multisigConfig.address, 'Multisig.address');

    for (const { name: chainName } of amplifierChains) {
        const chainPools: PoolParams[] = [];

        try {
            const result: RewardsPoolResponse = await queryRewardsPool(client, rewardsAddress, chainName, multisigAddress);
            chainPools.push({
                chainName,
                contractAddress: multisigAddress,
                contractType: 'Multisig',
                epoch_duration: result.epoch_duration,
                participation_threshold: result.participation_threshold,
                rewards_per_epoch: result.rewards_per_epoch,
            });
        } catch (error) {
            printError(`Failed to query Multisig pool for ${chainName}`, error instanceof Error ? error.message : String(error));
        }

        const votingVerifier = configManager.getVotingVerifierContract(chainName);
        if (!votingVerifier.address) {
            printError(`VotingVerifier address not found for ${chainName}`, '');
        } else {
            try {
                const result: RewardsPoolResponse = await queryRewardsPool(client, rewardsAddress, chainName, votingVerifier.address);
                chainPools.push({
                    chainName,
                    contractAddress: votingVerifier.address,
                    contractType: 'VotingVerifier',
                    epoch_duration: result.epoch_duration,
                    participation_threshold: result.participation_threshold,
                    rewards_per_epoch: result.rewards_per_epoch,
                });
            } catch (error) {
                printError(`Failed to query VotingVerifier pool for ${chainName}`, error instanceof Error ? error.message : String(error));
            }
        }

        poolParams.push(...chainPools);
    }

    return poolParams;
}

function printPoolParams(poolParams: PoolParams[], env: string): void {
    printHighlight(`REWARDS POOL PARAMETERS - ${env}`);
    printInfo('Found rewards pools', poolParams.length.toString());

    if (poolParams.length === 0) {
        printInfo('No rewards pools found', '');
        return;
    }

    const poolsByChain = poolParams.reduce(
        (acc, pool) => {
            if (!acc[pool.chainName]) {
                acc[pool.chainName] = [];
            }
            acc[pool.chainName].push(pool);
            return acc;
        },
        {} as Record<string, PoolParams[]>,
    );

    Object.entries(poolsByChain).forEach(([chainName, pools]) => {
        printInfo('Chain', chainName);
        pools.forEach((pool) => {
            printInfo(`  ${pool.contractType}`, pool.contractAddress);
            printInfo('    Epoch duration', pool.epoch_duration);
            printInfo('    Rewards per epoch', pool.rewards_per_epoch);
            printInfo('    Participation threshold', `[${pool.participation_threshold[0]}, ${pool.participation_threshold[1]}]`);
        });
        printInfo('', '');
    });
}

function buildUpdateMessages(poolParams: PoolParams[], newEpochDuration: string, newRewardsPerEpoch?: string): UpdatePoolParamsMessage[] {
    return poolParams.map((pool) => ({
        update_pool_params: {
            params: {
                epoch_duration: newEpochDuration,
                rewards_per_epoch: newRewardsPerEpoch ?? pool.rewards_per_epoch,
                participation_threshold: pool.participation_threshold,
            },
            pool_id: {
                chain_name: pool.chainName,
                contract: pool.contractAddress,
            },
        },
    }));
}

function isGovernanceRequired(configManager: ConfigManager): boolean {
    const rewardsGovernanceAddress = configManager.validateRequired(
        configManager.axelar.governanceAddress,
        'axelar.governanceAddress',
    );

    return rewardsGovernanceAddress === GOVERNANCE_MODULE_ADDRESS;
}

async function submitAsGovernanceProposal(
    client: ClientManager,
    config: ConfigManager,
    messages: UpdatePoolParamsMessage[],
    options: { title: string; description: string; deposit?: string; standardProposal?: boolean; yes?: boolean },
    fee: string | StdFee,
): Promise<string> {
    const [account] = client.accounts;
    printInfo('Proposer address', account.address);

    const encodedMessages = messages.map((msg) => {
        const msgOptions = {
            contractName: 'Rewards',
            msg: JSON.stringify(msg),
        };
        return encodeExecuteContract(config, msgOptions, undefined);
    });

    const proposalOptions = {
        title: options.title,
        description: options.description,
        deposit: options.deposit,
        standardProposal: options.standardProposal,
    };

    if (!confirmProposalSubmission(options, encodedMessages)) {
        throw new Error('Proposal submission cancelled by user');
    }

    const proposalId = await submitProposal(client, config, proposalOptions, encodedMessages, fee);
    printInfo('Proposal submitted', proposalId);
    return proposalId;
}

async function executeDirectly(
    client: ClientManager,
    rewardsAddress: string,
    messages: UpdatePoolParamsMessage[],
    options: { yes?: boolean },
    fee: string | StdFee,
): Promise<void> {
    const [account] = client.accounts;
    printInfo('Executor address', account.address);
    printInfo('Rewards contract', rewardsAddress);
    printInfo('Total messages to execute', messages.length.toString());

    printInfo('Messages to execute', '');
    messages.forEach((msg, index) => {
        printInfo(`Message ${index + 1}`, JSON.stringify(msg, null, 2));
    });

    if (prompt(`Proceed with direct execution?`, options.yes)) {
        throw new Error('Direct execution cancelled by user');
    }

    for (let i = 0; i < messages.length; i++) {
        const msg = messages[i];
        const poolInfo = `${msg.update_pool_params.pool_id.chain_name} - ${msg.update_pool_params.pool_id.contract}`;
        printInfo(`Executing message ${i + 1}/${messages.length}`, poolInfo);

        try {
            const executeFee: StdFee | 'auto' = fee === 'auto' ? 'auto' : (fee as StdFee);
            const result = await (client as SigningCosmWasmClient).execute(account.address, rewardsAddress, msg, executeFee, '');
            printInfo(`✅ Successfully executed message ${i + 1}`, result.transactionHash);
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : String(error);
            printError(`Failed to execute message ${i + 1} for ${poolInfo}`, errorMessage);
        }
    }

    printInfo('✅ Execution completed', '');
}

async function updateRewardsPoolEpochDuration(
    client: ClientManager,
    configManager: ConfigManager,
    options: Options & {
        epochDuration: string;
        rewardsPerEpoch?: string;
        title?: string;
        description?: string;
        yes?: boolean;
    },
    _args: string[],
    fee: string | StdFee,
): Promise<void> {
    validateParameters({ isPositiveInteger: { epochDuration: options.epochDuration } });

    if (options.rewardsPerEpoch !== undefined) {
        validateParameters({ isPositiveInteger: { rewardsPerEpoch: options.rewardsPerEpoch } });
    }

    const poolParams = await queryAllRewardsPools(client, configManager);
    validateParameters({ isPositiveInteger: { poolParamsLength: poolParams.length } });

    const amplifierChains = getAmplifierChains(configManager.chains);
    const expectedPoolCount = amplifierChains.length * 2;

    if (poolParams.length < expectedPoolCount) {
        printWarn(`Expected ${expectedPoolCount} pools but only found ${poolParams.length}. Some pools may be missing.`);
    }

    if (options.rewardsPerEpoch) {
        printInfo('Rewards per epoch', `Updating to ${options.rewardsPerEpoch} for all pools`);
    } else {
        printInfo('Rewards per epoch', 'Using existing values per pool');
    }

    const messages = buildUpdateMessages(poolParams, options.epochDuration, options.rewardsPerEpoch);

    const requiresGovernance = isGovernanceRequired(configManager);
    const executionMethod = requiresGovernance ? 'governance proposal' : 'direct execution (admin bypass)';
    printInfo('Execution method', executionMethod);

    if (requiresGovernance) {
        const title = options.title || 'Update rewards pools';
        const description = options.description || 'Update rewards pool parameters';

        await submitAsGovernanceProposal(
            client,
            configManager,
            messages,
            {
                title,
                description,
                deposit: options.deposit,
                yes: options.yes,
            },
            fee,
        );
    } else {
        await executeDirectly(client, configManager.getContractConfig('Rewards').address!, messages, { yes: options.yes }, fee);
    }
}

const program = new Command();

program.name('update-rewards-pool-epoch-duration').description('Query and update rewards pool epoch_duration for amplifier chains');

addAmplifierOptions(
    program
        .command('get-reward-pools')
        .description('Query and display current rewards pool parameters')
        .action(async (options) => {
            await mainQueryProcessor(
                async (client, configManager, options, _args) => {
                    const poolParams = await queryAllRewardsPools(client, configManager);
                    printPoolParams(poolParams, options.env);
                },
                {
                    ...options,
                    contractName: 'Rewards',
                    chainName: '',
                },
            );
        }),
    {},
);

addAmplifierOptions(
    program
        .command('update')
        .description('Update rewards pool parameters for amplifier chains')
        .addOption(new Option('--epoch-duration <epochDuration>', 'new epoch_duration value (in blocks)').makeOptionMandatory(true))
        .addOption(new Option('--rewards-per-epoch <rewardsPerEpoch>', 'update the rewards_per_epoch to new value'))
        .addOption(new Option('-t, --title <title>', 'governance proposal title (optional, auto-generated if not provided)'))
        .addOption(
            new Option('-d, --description <description>', 'governance proposal description (optional, auto-generated if not provided)'),
        )
        .addOption(new Option('--deposit <deposit>', 'governance proposal deposit amount'))
        .action(async (options) => {
            await mainProcessor(updateRewardsPoolEpochDuration, {
                ...options,
                contractName: 'Rewards',
                chainName: '',
            });
        }),
    {},
);

if (require.main === module) {
    program.parse();
}
