'use strict';

const { prepareDummyWallet, prepareClient, initContractConfig } = require('./utils');
const { loadConfig, printInfo, printWarn, getChainConfig, itsHubContractAddress } = require('../common');
const { Command } = require('commander');
const { addAmplifierQueryOptions } = require('./cli-utils');

async function rewards(client, config, args, options) {
    const [chainName] = args;

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
        throw new Error(`Chain ${chainName} not found in config`);
    }

    return await client.queryContractSmart(itsHubContractAddress(config.axelar), {
        its_chain: {
            chain: chainConfig.axelarId,
        },
    });
}

async function tokenConfig(client, config, args, _options) {
    const [tokenId] = args;
    const itsHubAddress = itsHubContractAddress(config.axelar);

    if (!itsHubAddress) {
        printWarn('ITS Hub contract address not found in config');
        return;
    }

    try {
        const result = await client.queryContractSmart(itsHubAddress, {
            token_config: { token_id: tokenId },
        });

        printInfo(`Token config for ${tokenId}`, JSON.stringify(result, null, 2));
    } catch (error) {
        printWarn(`Failed to fetch token config for ${tokenId}`, error?.message || String(error));
    }
}

async function customTokenMetadata(client, config, args, options) {
    const [chainName, tokenAddress] = args;
    const itsHubAddress = itsHubContractAddress(config.axelar);

    if (!itsHubAddress) {
        printWarn('ITS Hub contract address not found in config');
        return;
    }

    const chainConfig = getChainConfig(config.chains, chainName);
    if (!chainConfig) {
        printWarn(`Chain ${chainName} not found in config`);
        return;
    }

    try {
        const result = await client.queryContractSmart(itsHubAddress, {
            custom_token_metadata: {
                chain: chainConfig.axelarId,
                token_address: tokenAddress,
            },
        });

        printInfo(`Custom token metadata for ${tokenAddress} on ${chainName}`, JSON.stringify(result, null, 2));
    } catch (error) {
        printWarn(`Failed to fetch custom token metadata for ${tokenAddress} on ${chainName}`, error?.message || String(error));
    }
}

async function tokenInstance(client, config, args, options) {
    const [chainName, tokenId] = args;
    const itsHubAddress = itsHubContractAddress(config.axelar);

    if (!itsHubAddress) {
        printWarn('ITS Hub contract address not found in config');
        return;
    }

    const chainConfig = getChainConfig(config.chains, chainName);
    if (!chainConfig) {
        printWarn(`Chain ${chainName} not found in config`);
        return;
    }

    try {
        const result = await client.queryContractSmart(itsHubAddress, {
            token_instance: {
                chain: chainConfig.axelarId,
                token_id: tokenId,
            },
        });

        printInfo(`Token instance for ${tokenId} on ${chainName}`, JSON.stringify(result, null, 2));
    } catch (error) {
        printWarn(`Failed to fetch token instance for ${tokenId} on ${chainName}`, error?.message || String(error));
    }
}

async function itsChainConfig(client, config, args, options) {
    const [chainName] = args;

    try {
        const result = await getItsChainConfig(client, config, chainName);
        printInfo(`ITS chain config for ${chainName}`, JSON.stringify(result, null, 2));
    } catch (error) {
        throw error;
    }
}

const mainProcessor = async (processor, args, options) => {
    const { env } = options;
    const config = loadConfig(env);

    initContractConfig(config, options);

    const wallet = await prepareDummyWallet(options);
    const client = await prepareClient(config, wallet);

    await processor(client, config, args, options);
};

const programHandler = () => {
    const program = new Command();

    program.name('query').description('Query contract state');

    const rewardCmd = program
        .command('rewards <chainName>')
        .description('Query rewards pool state for multisig and voting_verifier contracts')
        .action((chainName, options) => {
            mainProcessor(rewards, [chainName], options);
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

    const itsChainConfigCmd = program
        .command('its-chain-config <chainName>')
        .description('Query ITS chain configuration for a specific chain')
        .action((chainName, options) => {
            mainProcessor(itsChainConfig, [chainName], options);
        });

    addAmplifierQueryOptions(rewardCmd);
    addAmplifierQueryOptions(tokenConfigCmd);
    addAmplifierQueryOptions(customTokenMetadataCmd);
    addAmplifierQueryOptions(tokenInstanceCmd);
    addAmplifierQueryOptions(itsChainConfigCmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}

module.exports = {
    getItsChainConfig,
};
