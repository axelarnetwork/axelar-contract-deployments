'use strict';

const { prepareDummyWallet, prepareClient, initContractConfig } = require('./utils');
const { loadConfig, printInfo, printWarn } = require('../common');
const { Command } = require('commander');
const { addAmplifierQueryOptions } = require('./cli-utils');

async function rewards(client, config, _itsHubAddress, _args, options) {
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

async function tokenConfig(client, _config, itsHubAddress, args, _options) {
    const [tokenId] = args;

    try {
        const result = await client.queryContractSmart(itsHubAddress, {
            token_config: { token_id: tokenId },
        });

        printInfo(`Token config for ${tokenId}`, JSON.stringify(result, null, 2));
    } catch (error) {
        printWarn(`Failed to fetch token config for ${tokenId}`, error?.message || String(error));
    }
}

async function customTokenMetadata(client, _config, itsHubAddress, args, _options) {
    const [chainName, tokenAddress] = args;

    try {
        const result = await client.queryContractSmart(itsHubAddress, {
            custom_token_metadata: {
                chain: chainName,
                token_address: tokenAddress,
            },
        });

        printInfo(`Custom token metadata for ${tokenAddress} on ${chainName}`, JSON.stringify(result, null, 2));
    } catch (error) {
        printWarn(`Failed to fetch custom token metadata for ${tokenAddress} on ${chainName}`, error?.message || String(error));
    }
}

async function tokenInstance(client, _config, itsHubAddress, args, _options) {
    const [chainName, tokenId] = args;

    try {
        const result = await client.queryContractSmart(itsHubAddress, {
            token_instance: {
                chain: chainName,
                token_id: tokenId,
            },
        });

        printInfo(`Token instance for ${tokenId} on ${chainName}`, JSON.stringify(result, null, 2));
    } catch (error) {
        printWarn(`Failed to fetch token instance for ${tokenId} on ${chainName}`, error?.message || String(error));
    }
}

const mainProcessor = async (processor, args, options, requiresItsHub = true) => {
    const { env } = options;
    const config = loadConfig(env);

    initContractConfig(config, options);

    const wallet = await prepareDummyWallet(options);
    const client = await prepareClient(config, wallet);

    const itsHubAddress = config.axelar?.contracts?.InterchainTokenService?.address;

    if (requiresItsHub && !itsHubAddress) {
        printWarn('ITS Hub contract address not found in config');
        return;
    }

    await processor(client, config, itsHubAddress, args, options);
};

const programHandler = () => {
    const program = new Command();

    program.name('query').description('Query contract state');

    const rewardCmd = program
        .command('rewards')
        .description('Query rewards pool state for multisig and voting_verifier contracts')
        .action((options) => {
            mainProcessor(rewards, [], options, false);
        });

    const tokenConfigCmd = program
        .command('token-config <tokenId>')
        .description('Query token config from ITS Hub')
        .action((tokenId, options) => {
            mainProcessor(tokenConfig, [tokenId], options);
        });

    const customTokenMetadataCmd = program
        .command('custom-token-metadata <chainName> <tokenAddress>')
        .description('Query custom token metadata by chain name and token address')
        .action((chainName, tokenAddress, options) => {
            mainProcessor(customTokenMetadata, [chainName, tokenAddress], options);
        });

    const tokenInstanceCmd = program
        .command('token-instance <chainName> <tokenId>')
        .description('Query token instance by chain name and token ID')
        .action((chainName, tokenId, options) => {
            mainProcessor(tokenInstance, [chainName, tokenId], options);
        });

    addAmplifierQueryOptions(rewardCmd);
    addAmplifierQueryOptions(tokenConfigCmd);
    addAmplifierQueryOptions(customTokenMetadataCmd);
    addAmplifierQueryOptions(tokenInstanceCmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}
