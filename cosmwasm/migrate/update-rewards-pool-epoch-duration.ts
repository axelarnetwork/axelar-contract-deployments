'use strict';

import { CosmWasmClient, SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { GasPrice, StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';
import { MsgExecuteContract } from 'cosmjs-types/cosmwasm/wasm/v1/tx';

import { ConfigManager } from '../../common/config';
import { getAmplifierChains, loadConfig, printInfo, prompt } from '../../common/utils';
import { addAmplifierOptions } from '../cli-utils';
import { RewardsPoolResponse, queryRewardsPool } from '../query';
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

async function queryAllRewardsPools(env: string, newEpochDuration: string): Promise<PoolParams[]> {
    console.log(`\n${'='.repeat(60)}`);
    console.log(`Environment: ${env.toUpperCase()}`);
    console.log(`New epoch_duration: ${newEpochDuration}`);
    console.log('='.repeat(60));

    const poolParams: PoolParams[] = [];

    try {
        const configManager = new ConfigManager(env);
        const client = await CosmWasmClient.connect(configManager.axelar.rpc);
        const rewardsAddress = configManager.getContractConfig('Rewards').address;

        if (!rewardsAddress) {
            console.error(`  ❌ Rewards contract address not found for ${env}`);
            return poolParams;
        }

        const config = loadConfig(env);
        const amplifierChains = getAmplifierChains(config.chains);
        console.log(`  Found ${amplifierChains.length} amplifier chain(s)\n`);

        if (amplifierChains.length === 0) {
            console.log(`  ⚠️  No amplifier chains found in ${env}`);
            return poolParams;
        }

        for (const { name: chainName } of amplifierChains) {
            // Query Multisig rewards pool
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
                        `  ❌ Failed to query Multisig pool for ${chainName}: ${error instanceof Error ? error.message : String(error)}`,
                    );
                }
            }

            // Query VotingVerifier rewards pool
            try {
                const votingVerifier = configManager.getVotingVerifierContract(chainName);
                if (votingVerifier.address) {
                    try {
                        const result: RewardsPoolResponse = await queryRewardsPool(
                            client,
                            rewardsAddress,
                            chainName,
                            votingVerifier.address,
                        );
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
                            `  ❌ Failed to query VotingVerifier pool for ${chainName}: ${error instanceof Error ? error.message : String(error)}`,
                        );
                    }
                }
            } catch (error) {
                // VotingVerifier might not exist for some chains (like xrpl uses XrplVotingVerifier)
                // This should be handled by getVotingVerifierContract, but just in case
                console.error(
                    `  ⚠️  Could not get VotingVerifier for ${chainName}: ${error instanceof Error ? error.message : String(error)}`,
                );
            }
        }

        console.log(`\n  ✅ Successfully queried ${poolParams.length} rewards pool(s)`);
        return poolParams;
    } catch (error) {
        console.error(`  ❌ Error processing ${env}:`, error instanceof Error ? error.message : String(error));
        return poolParams;
    }
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

function printUpdateSummary(poolParams: PoolParams[], newEpochDuration: string, messages: UpdatePoolParamsMessage[]): void {
    console.log(`\n${'='.repeat(60)}`);
    console.log('UPDATE SUMMARY');
    console.log('='.repeat(60));
    console.log(`Total pools to update: ${poolParams.length}`);
    console.log(`New epoch_duration: ${newEpochDuration}\n`);

    poolParams.forEach((pool, index) => {
        console.log(`${index + 1}. ${pool.chainName} - ${pool.contractType}`);
        console.log(`   Contract: ${pool.contractAddress}`);
        console.log(`   Current epoch_duration: ${pool.epoch_duration}`);
        console.log(`   New epoch_duration: ${newEpochDuration}`);
        console.log(`   Rewards per epoch: ${pool.rewards_per_epoch}`);
        console.log(`   Participation threshold: [${pool.participation_threshold[0]}, ${pool.participation_threshold[1]}]`);
        console.log('');
    });

    console.log('='.repeat(60));
    console.log('MESSAGES TO BE EXECUTED:');
    console.log('='.repeat(60));
    console.log(JSON.stringify(messages, null, 2));
    console.log('='.repeat(60));
}

function printProposalMessages(messages: any[]): void {
    // v0.50: array of messages
    messages.forEach((message, index) => {
        const decoded = MsgExecuteContract.decode(message.value);
        const msgJson = JSON.parse(Buffer.from(decoded.msg).toString());
        printInfo(
            `Message ${index + 1}`,
            JSON.stringify(
                {
                    typeUrl: message.typeUrl,
                    contract: decoded.contract,
                    msg: msgJson,
                },
                null,
                2,
            ),
        );
    });
}

function isGovernanceRequired(configManager: ConfigManager): boolean {
    const rewardsConfig = configManager.getContractConfig('Rewards');
    const rewardsGovernanceAddress = (rewardsConfig as any).governanceAddress;

    if (!rewardsGovernanceAddress) {
        throw new Error('Rewards contract governanceAddress not found in config');
    }

    // If governanceAddress equals GOVERNANCE_MODULE_ADDRESS, use governance proposal
    // Otherwise, use direct execution (admin bypass)
    return rewardsGovernanceAddress === GOVERNANCE_MODULE_ADDRESS;
}

async function submitAsGovernanceProposal(
    client: SigningCosmWasmClient,
    config: ConfigManager,
    rewardsAddress: string,
    messages: UpdatePoolParamsMessage[],
    options: { title?: string; description?: string; deposit?: string; yes?: boolean },
    fee: string | StdFee,
): Promise<string> {
    const [account] = await client.signer.getAccounts();
    printInfo('Proposer address', account.address);

    // Encode messages for proposal
    // Rewards is a global contract (not chain-specific), so we pass undefined for chainName
    // This causes encodeExecuteContract to use the global contract address
    const encodedMessages = messages.map((msg) => {
        const msgOptions = {
            contractName: 'Rewards',
            msg: JSON.stringify(msg),
        };
        return encodeExecuteContract(config, msgOptions, undefined);
    });

    const proposalOptions = {
        title: options.title || `Update rewards pool epoch_duration to ${messages[0]?.update_pool_params?.params?.epoch_duration || 'N/A'}`,
        description: options.description || `Update epoch_duration for ${messages.length} rewards pool(s)`,
        deposit: options.deposit || config.getProposalDepositAmount(),
    };

    // Print proposal for review
    printProposalMessages(encodedMessages);
    if (prompt(`Proceed with governance proposal submission?`, options.yes)) {
        throw new Error('Proposal submission cancelled by user');
    }

    const proposalId = await submitProposal(client, config, proposalOptions, encodedMessages, fee);
    printInfo('Proposal submitted', proposalId);
    return proposalId;
}

async function executeDirectly(
    client: SigningCosmWasmClient,
    rewardsAddress: string,
    messages: UpdatePoolParamsMessage[],
    options: { yes?: boolean },
    fee: string | StdFee,
): Promise<void> {
    const [account] = await client.signer.getAccounts();
    printInfo('Executor address', account.address);
    printInfo('Rewards contract', rewardsAddress);
    printInfo('Total messages to execute', messages.length.toString());

    // Print messages for review
    console.log('\nMessages to execute:');
    messages.forEach((msg, index) => {
        printInfo(`Message ${index + 1}`, JSON.stringify(msg, null, 2));
    });

    if (prompt(`Proceed with direct execution?`, options.yes)) {
        throw new Error('Direct execution cancelled by user');
    }

    // Execute messages sequentially
    for (let i = 0; i < messages.length; i++) {
        const msg = messages[i];
        const poolInfo = `${msg.update_pool_params.pool_id.chain_name} - ${msg.update_pool_params.pool_id.contract}`;
        printInfo(`Executing message ${i + 1}/${messages.length}`, poolInfo);

        try {
            const result = await client.execute(account.address, rewardsAddress, msg, fee, '');
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

program
    .name('update-rewards-pool-epoch-duration')
    .description('Query and update rewards pool epoch_duration for amplifier chains')
    .addOption(
        new Option('--epoch-duration <epochDuration>', 'new epoch_duration value (in blocks)')
            .makeOptionMandatory(true)
            .env('EPOCH_DURATION'),
    )
    .addOption(new Option('-t, --title <title>', 'governance proposal title (required for governance proposals)'))
    .addOption(new Option('-d, --description <description>', 'governance proposal description (required for governance proposals)'))
    .addOption(new Option('--deposit <deposit>', 'governance proposal deposit amount'));

// Add standard amplifier options (env, mnemonic, yes)
addAmplifierOptions(program, {});

program.action(async (options) => {
    // Validate epoch_duration
    const epochDurationNum = Number(options.epochDuration);
    if (isNaN(epochDurationNum) || epochDurationNum <= 0 || !Number.isInteger(epochDurationNum)) {
        throw new Error('--epoch-duration must be a positive integer');
    }

    // Query all pools
    const poolParams = await queryAllRewardsPools(options.env, options.epochDuration);

    if (poolParams.length === 0) {
        throw new Error('No rewards pools found. Cannot proceed with update.');
    }

    // Check for any query failures - all amplifier chains should have pools
    const configManager = new ConfigManager(options.env);
    const config = loadConfig(options.env);
    const amplifierChains = getAmplifierChains(config.chains);
    const expectedPoolCount = amplifierChains.length * 2; // Multisig + VotingVerifier per chain

    if (poolParams.length < expectedPoolCount) {
        console.warn(`⚠️  Warning: Expected ${expectedPoolCount} pools but only found ${poolParams.length}. Some pools may be missing.`);
    }

    // Build update messages
    const messages = buildUpdateMessages(poolParams, options.epochDuration);

    // Print summary
    printUpdateSummary(poolParams, options.epochDuration, messages);

    // Determine execution method based on Rewards contract governanceAddress
    const requiresGovernance = isGovernanceRequired(configManager);

    if (requiresGovernance) {
        // Validate governance proposal options
        if (!options.title) {
            throw new Error('--title is required for governance proposals');
        }
        if (!options.description) {
            throw new Error('--description is required for governance proposals');
        }

        // Submit as governance proposal
        const client = await prepareSigningClient(options.mnemonic, configManager);
        const fee = configManager.getFee();
        await submitAsGovernanceProposal(
            client,
            configManager,
            configManager.getContractConfig('Rewards').address!,
            messages,
            {
                title: options.title,
                description: options.description,
                deposit: options.deposit,
                yes: options.yes,
            },
            fee,
        );
    } else {
        // Direct execution (admin bypass)
        const client = await prepareSigningClient(options.mnemonic, configManager);
        const fee = configManager.getFee();
        await executeDirectly(client, configManager.getContractConfig('Rewards').address!, messages, { yes: options.yes }, fee);
    }
});

async function prepareSigningClient(mnemonic: string, configManager: ConfigManager): Promise<SigningCosmWasmClient> {
    const wallet = await DirectSecp256k1HdWallet.fromMnemonic(mnemonic, { prefix: 'axelar' });
    return await SigningCosmWasmClient.connectWithSigner(configManager.axelar.rpc, wallet, {
        gasPrice: GasPrice.fromString(configManager.axelar.gasPrice),
    });
}

if (require.main === module) {
    program.parse();
}

export { queryAllRewardsPools };
