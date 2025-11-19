'use strict';

import { CosmWasmClient, SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { AccountData, DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { GasPrice, StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';

import { AxelarContractConfig, ConfigManager } from '../../common/config';
import { getAmplifierChains, loadConfig, printInfo, prompt } from '../../common/utils';
import { addAmplifierOptions } from '../cli-utils';
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

async function queryAllRewardsPools(env: string): Promise<PoolParams[]> {
    const poolParams: PoolParams[] = [];

    try {
        const configManager = new ConfigManager(env);
        const client = await CosmWasmClient.connect(configManager.axelar.rpc);
        const rewardsAddress = configManager.getContractConfig('Rewards').address;

        if (!rewardsAddress) {
            throw new Error(`Rewards contract address not found for ${env}`);
        }

        const config = loadConfig(env);
        const amplifierChains = getAmplifierChains(config.chains);

        if (amplifierChains.length === 0) {
            throw new Error(`No amplifier chains found in ${env}`);
        }

        for (const { name: chainName } of amplifierChains) {
            const multisigAddress = configManager.getContractConfig('Multisig').address;
            if (multisigAddress) {
                try {
                    const result: RewardsPoolResponse = await queryRewardsPool(client, rewardsAddress, chainName, multisigAddress);
                    poolParams.push({
                        chainName,
                        contractAddress: multisigAddress,
                        contractType: 'Multisig',
                        epoch_duration: result.epoch_duration,
                        participation_threshold: result.participation_threshold,
                        rewards_per_epoch: result.rewards_per_epoch,
                    });
                } catch (error) {
                    console.error(
                        `Failed to query Multisig pool for ${chainName}: ${error instanceof Error ? error.message : String(error)}`,
                    );
                }
            }

            const votingVerifier = configManager.getVotingVerifierContract(chainName);
            if (votingVerifier.address) {
                try {
                    const result: RewardsPoolResponse = await queryRewardsPool(client, rewardsAddress, chainName, votingVerifier.address);
                    poolParams.push({
                        chainName,
                        contractAddress: votingVerifier.address,
                        contractType: 'VotingVerifier',
                        epoch_duration: result.epoch_duration,
                        participation_threshold: result.participation_threshold,
                        rewards_per_epoch: result.rewards_per_epoch,
                    });
                } catch (error) {
                    console.error(
                        `Failed to query VotingVerifier pool for ${chainName}: ${error instanceof Error ? error.message : String(error)}`,
                    );
                }
            }
        }

        return poolParams;
    } catch (error) {
        throw new Error(`Error querying rewards pools for ${env}: ${error instanceof Error ? error.message : String(error)}`);
    }
}

function printPoolParams(poolParams: PoolParams[], env: string): void {
    console.log(`REWARDS POOL PARAMETERS - ${env}`);
    console.log(`Found ${poolParams.length} rewards pools\n`);

    if (poolParams.length === 0) {
        console.log('No rewards pools found');
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
        console.log(`${chainName}:`);
        pools.forEach((pool) => {
            console.log(`  ${pool.contractType}:`);
            console.log(`    Contract: ${pool.contractAddress}`);
            console.log(`    Epoch duration: ${pool.epoch_duration}`);
            console.log(`    Rewards per epoch: ${pool.rewards_per_epoch}`);
            console.log(`    Participation threshold: [${pool.participation_threshold[0]}, ${pool.participation_threshold[1]}]`);
        });
        console.log('');
    });
}

function buildUpdateMessages(poolParams: PoolParams[], newEpochDuration: string): UpdatePoolParamsMessage[] {
    return poolParams.map((pool) => ({
        update_pool_params: {
            params: {
                epoch_duration: newEpochDuration,
                rewards_per_epoch: pool.rewards_per_epoch,
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
    const rewardsConfig = configManager.getContractConfig('Rewards') as AxelarContractConfig;
    const rewardsGovernanceAddress = rewardsConfig.governanceAddress;

    if (!rewardsGovernanceAddress) {
        throw new Error('Rewards contract governanceAddress not found in config');
    }

    return rewardsGovernanceAddress === GOVERNANCE_MODULE_ADDRESS;
}

interface ClientManager extends SigningCosmWasmClient {
    accounts: readonly AccountData[];
}

async function submitAsGovernanceProposal(
    client: ClientManager,
    config: ConfigManager,
    messages: UpdatePoolParamsMessage[],
    options: { title: string; description: string; deposit?: string; yes?: boolean },
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
        deposit: options.deposit || config.getProposalDepositAmount(),
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

    console.log('\nMessages to execute:');
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
            throw new Error(
                `Failed to execute message ${i + 1} for ${poolInfo}: ${error instanceof Error ? error.message : String(error)}`,
            );
        }
    }

    printInfo('✅ All messages executed successfully', '');
}

const program = new Command();

program.name('update-rewards-pool-epoch-duration').description('Query and update rewards pool epoch_duration for amplifier chains');

const getRewardPoolsCmd = program
    .command('get-reward-pools')
    .description('Query and display current rewards pool parameters')
    .action(async (options) => {
        try {
            const poolParams = await queryAllRewardsPools(options.env);
            printPoolParams(poolParams, options.env);
        } catch (error) {
            console.error(`Error getting reward pools: ${error instanceof Error ? error.message : String(error)}`);
            process.exit(1);
        }
    });

addAmplifierOptions(getRewardPoolsCmd, {});

const updateCmd = program
    .command('update')
    .description('Update rewards pool epoch_duration for amplifier chains')
    .addOption(new Option('--epoch-duration <epochDuration>', 'new epoch_duration value (in blocks)').makeOptionMandatory(true))
    .addOption(new Option('-t, --title <title>', 'governance proposal title (optional, auto-generated if not provided)'))
    .addOption(new Option('-d, --description <description>', 'governance proposal description (optional, auto-generated if not provided)'))
    .addOption(new Option('--deposit <deposit>', 'governance proposal deposit amount'));

addAmplifierOptions(updateCmd, {});

updateCmd.action(async (options) => {
    const epochDurationNum = Number(options.epochDuration);
    if (isNaN(epochDurationNum) || epochDurationNum <= 0 || !Number.isInteger(epochDurationNum)) {
        throw new Error('--epoch-duration must be a positive integer');
    }

    const poolParams = await queryAllRewardsPools(options.env);

    if (poolParams.length === 0) {
        throw new Error('No rewards pools found. Cannot proceed with update.');
    }

    const configManager = new ConfigManager(options.env);
    const config = loadConfig(options.env);
    const amplifierChains = getAmplifierChains(config.chains);
    const expectedPoolCount = amplifierChains.length * 2;

    if (poolParams.length < expectedPoolCount) {
        console.warn(`Warning: Expected ${expectedPoolCount} pools but only found ${poolParams.length}. Some pools may be missing.`);
    }

    const messages = buildUpdateMessages(poolParams, options.epochDuration);

    const requiresGovernance = isGovernanceRequired(configManager);
    const executionMethod = requiresGovernance ? 'governance proposal' : 'direct execution (admin bypass)';
    printInfo('Execution method', executionMethod);

    const client = await prepareSigningClient(options.mnemonic, configManager);
    const fee = configManager.getFee();

    if (requiresGovernance) {
        const title = options.title || `Update rewards pool epoch_duration to ${options.epochDuration}`;
        const description =
            options.description ||
            `Update epoch_duration from current values to ${options.epochDuration} blocks for ${messages.length} rewards pools across ${new Set(poolParams.map((p) => p.chainName)).size} amplifier chains.`;

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
});

async function prepareSigningClient(mnemonic: string, configManager: ConfigManager): Promise<ClientManager> {
    const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, { prefix: 'axelar' });
    const client = (await SigningCosmWasmClient.connectWithSigner(configManager.axelar.rpc, wallet, {
        gasPrice: GasPrice.fromString(configManager.axelar.gasPrice),
    })) as ClientManager;
    client.accounts = await wallet.getAccounts();
    return client;
}

if (require.main === module) {
    program.parse();
}
