'use strict';

const { printInfo, printWarn, getChainConfig, itsHubContractAddress } = require('../common');
const { mainQueryProcessor } = require('./processor');
const { Command } = require('commander');
const { addAmplifierQueryOptions } = require('./cli-utils');

async function rewards(client, config, _options, args, _fee) {
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

async function tokenConfig(client, config, _options, args, _fee) {
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

async function customTokenMetadata(client, config, _options, args, _fee) {
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

async function tokenInstance(client, config, _options, args, _fee) {
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

async function itsChainConfig(client, config, _options, args, _fee) {
    const [chainName] = args;

    try {
        const result = await getItsChainConfig(client, config, chainName);
        printInfo(`ITS chain config for ${chainName}`, JSON.stringify(result, null, 2));
    } catch (error) {
        throw error;
    }
}

async function saveDeployedContracts(client, config, _options, args, _fee) {
    const [chainName] = args;

    const coordinatorAddress = config.axelar?.contracts?.Coordinator?.address;
    if (!coordinatorAddress) {
        return printWarn(`Coordinator contract address not found in config for ${chainName}`);
    }

    const deploymentName = config.axelar?.contracts?.Coordinator?.deployments?.[chainName]?.deploymentName;
    if (!deploymentName) {
        return printWarn(
            `No deployment found for chain ${chainName} in config.`,
            `Run 'ts-node cosmwasm/submit-proposal.js instantiate-chain-contracts -n ${chainName}'.`,
        );
    }

    let result;
    try {
        result = await client.queryContractSmart(coordinatorAddress, {
            deployed_contracts: {
                deployment_name: deploymentName,
            },
        });

        printInfo(`Fetched deployed contracts for ${chainName}`, JSON.stringify(result, null, 2));
    } catch (error) {
        return printWarn(`Failed to fetch deployed contracts for ${chainName}`, error?.message || String(error));
    }

    if (
        !result.verifier ||
        !config.axelar.contracts.VotingVerifier?.[chainName] ||
        !result.prover ||
        !config.axelar.contracts.MultisigProver?.[chainName] ||
        !result.gateway
    ) {
        return printWarn(
            `Missing config for ${chainName}.`,
            `Run 'ts-node cosmwasm/submit-proposal.js instantiate-chain-contracts -n ${chainName}'.`,
        );
    }

    config.axelar.contracts.VotingVerifier[chainName] = {
        ...config.axelar.contracts.VotingVerifier[chainName],
        address: result.verifier,
    };
    printInfo(`Updated VotingVerifier[${chainName}].address`, result.verifier);

    if (!config.axelar.contracts.Gateway) {
        config.axelar.contracts.Gateway = {};
    }
    if (!config.axelar.contracts.Gateway[chainName]) {
        config.axelar.contracts.Gateway[chainName] = {};
    }
    config.axelar.contracts.Gateway[chainName] = {
        ...config.axelar.contracts.Gateway[chainName],
        address: result.gateway,
    };
    printInfo(`Updated Gateway[${chainName}].address`, result.gateway);

    config.axelar.contracts.MultisigProver[chainName] = {
        ...config.axelar.contracts.MultisigProver[chainName],
        address: result.prover,
    };
    printInfo(`Updated MultisigProver[${chainName}].address`, result.prover);

    printInfo(`Config updated successfully for ${chainName}`);
}

const programHandler = () => {
    const program = new Command();

    program.name('query').description('Query contract state');

    const rewardsCmd = program
        .command('rewards <chainName>')
        .description('Query rewards pool state for multisig and voting_verifier contracts')
        .action((chainName, options) => {
            mainQueryProcessor(rewards, options, [chainName]);
        });

    const tokenConfigCmd = program
        .command('token-config <tokenId>')
        .description('Query token config from ITS Hub')
        .action((tokenId, options) => {
            mainQueryProcessor(tokenConfig, options, [tokenId]);
        });

    const customTokenMetadataCmd = program
        .command('custom-token-metadata <chainName> <tokenAddress>')
        .description('Query custom token metadata by chain name and token address')
        .action((chainName, tokenAddress, options) => {
            mainQueryProcessor(customTokenMetadata, options, [chainName, tokenAddress]);
        });

    const tokenInstanceCmd = program
        .command('token-instance <chainName> <tokenId>')
        .description('Query token instance by chain name and token ID')
        .action((chainName, tokenId, options) => {
            mainQueryProcessor(tokenInstance, options, [chainName, tokenId]);
        });

    const itsChainConfigCmd = program
        .command('its-chain-config <chainName>')
        .description('Query ITS chain configuration for a specific chain')
        .action((chainName, options) => {
            mainQueryProcessor(itsChainConfig, options, [chainName]);
        });

    const saveDeployedContractsCmd = program
        .command('save-deployed-contracts <chainName>')
        .description('Query and save deployed Gateway, VotingVerifier and MultisigProver contracts via Coordinator')
        .action((chainName, options) => {
            mainQueryProcessor(saveDeployedContracts, options, [chainName]);
        });

    addAmplifierQueryOptions(rewardsCmd);
    addAmplifierQueryOptions(tokenConfigCmd);
    addAmplifierQueryOptions(customTokenMetadataCmd);
    addAmplifierQueryOptions(tokenInstanceCmd);
    addAmplifierQueryOptions(itsChainConfigCmd);
    addAmplifierQueryOptions(saveDeployedContractsCmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}

module.exports = {
    getItsChainConfig,
};
