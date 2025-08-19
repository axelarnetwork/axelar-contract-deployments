'use strict';

const { prepareDummyWallet, prepareClient, initContractConfig } = require('./utils');
const { loadConfig, printInfo, printWarn, getChainConfig } = require('../common');
const { Command } = require('commander');
const { addAmplifierQueryOptions } = require('./cli-utils');

async function rewards(client, config, options) {
    const { chainName } = options;

    const rewardsContractAddresses = {
        multisig: config.axelar.contracts.Multisig.address,
        voting_verifier: config.axelar.contracts.VotingVerifier?.[chainName]?.address,
    };

    for (const [key, address] of Object.entries(rewardsContractAddresses)) {
        try {
            const result = await client.queryContractSmart(config.axelar.contracts.Rewards.address, {
                rewards_pool: {
                    pool_id: {
                        chain_name: chainName,
                        contract: address,
                    },
                },
            });

            printInfo(`Rewards pool for ${key} on ${chainName}`, JSON.stringify(result, null, 2));
        } catch (error) {
            printWarn(`Failed to fetch rewards pool for ${key} on ${chainName}`, `${error.message}`);
        }
    }
}

async function getItsChainConfig(client, config, chainName) {
    const chainConfig = getChainConfig(config.chains, chainName);
    if (!chainConfig) {
        throw new Error(`Chain '${chainName}' not found in config`);
    }

    const itsHubAddress = config.axelar.contracts.InterchainTokenService.address;

    return await client.queryContractSmart(itsHubAddress, {
        its_chain: {
            chain: chainName,
        },
    });
}

async function itsChainConfig(client, config, options) {
    const { chainName } = options;

    try {
        const result = await getItsChainConfig(client, config, chainName);
        printInfo(`ITS chain config for ${chainName}`, JSON.stringify(result, null, 2));
    } catch (error) {
        printWarn(`Failed to fetch chain config for ${chainName}`, `${error.message}`);
        throw error;
    }
}

const mainProcessor = async (processor, options) => {
    const { env } = options;
    const config = loadConfig(env);

    initContractConfig(config, options);

    const wallet = await prepareDummyWallet(options);
    const client = await prepareClient(config, wallet);

    await processor(client, config, options);
};

const programHandler = () => {
    const program = new Command();

    program.name('query').description('Query contract state');

    const rewardCmd = program
        .command('rewards')
        .description('Query rewards pool state for multisig and voting_verifier contracts')
        .action((options) => {
            mainProcessor(rewards, options);
        });

    addAmplifierQueryOptions(rewardCmd);

    const itsChainConfigCmd = program
        .command('its-chain-config')
        .description('Query ITS chain configuration for a specific chain')
        .argument('<chainName>', 'name of the chain to query')
        .action((chainName, options) => {
            options.chainName = chainName;
            mainProcessor(itsChainConfig, options);
        });

    addAmplifierQueryOptions(itsChainConfigCmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}
