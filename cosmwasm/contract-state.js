'use strict';

require('dotenv').config();

const { prepareWallet, prepareClient, initContractConfig } = require('./utils');
const { loadConfig, printInfo, printWarn } = require('../common');
const { Command } = require('commander');
const { addAmplifierQueryOptions } = require('./cli-utils');

const CONTRACT_MAP = {
    multisig: (config) => config.axelar.contracts.Multisig.address,
    voting_verifier: (config, chainName) => config.axelar.contracts.VotingVerifier?.[chainName]?.address,
};

const rewards = async (client, config, options) => {
    const { contract, chainName } = options;
    const getContractAddress = CONTRACT_MAP[contract];

    if (!getContractAddress) {
        printWarn(`Query to ${contract} is not supported.`);
        return;
    }

    try {
        const result = await client.queryContractSmart(config.axelar.contracts.Rewards.address, {
            rewards_pool: {
                pool_id: {
                    chain_name: chainName,
                    contract: getContractAddress(config, chainName),
                },
            },
        });

        printInfo(`Rewards pool for ${contract} on ${chainName}`, JSON.stringify(result, null, 2));
    } catch (error) {
        printWarn(`Failed to fetch rewards pool for ${contract} on ${chainName}`, `${error.message}`);
    }
};

const mainProcessor = async (processor, options) => {
    const { env } = options;
    const config = loadConfig(env);

    initContractConfig(config, options);

    const wallet = await prepareWallet(options);
    const client = await prepareClient(config, wallet);

    await processor(client, config, options);
};

const programHandler = () => {
    const program = new Command();

    program.name('contract-state').description('Query contrct state');

    const rewardCmd = program
        .command('rewards')
        .description('Query rewards contract state')
        .argument('<contract>', 'Contract to query rewards')
        .action((contract, options) => {
            options.contract = contract;
            mainProcessor(rewards, options);
        });
    addAmplifierQueryOptions(rewardCmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}
