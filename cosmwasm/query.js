'use strict';

const { prepareDummyWallet, prepareClient, initContractConfig } = require('./utils');
const { loadConfig, printInfo, printWarn, getChainConfig, itsHubContractAddress, saveConfig } = require('../common');
const { Command } = require('commander');
const { addAmplifierQueryOptions } = require('./cli-utils');

async function rewards(client, config, _args, options) {
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

    return await client.queryContractSmart(itsHubContractAddress(config.axelar), {
        its_chain: {
            chain: chainConfig.axelarId,
        },
    });
}

async function itsChainConfig(client, config, options) {
    const { chainName } = options;

    try {
        const result = await getItsChainConfig(client, config, chainName);
        printInfo(`ITS chain config for ${chainName}`, JSON.stringify(result, null, 2));
    } catch (error) {
        throw error;
    }
}

async function tokenConfig(client, config, args, _options) {
    const [chainName, tokenId] = args;
    const itsHubAddress = config.axelar?.contracts?.InterchainTokenService?.address;

    if (!itsHubAddress) {
        printWarn('ITS Hub contract address not found in config');
        return;
    }

    try {
        const result = await client.queryContractSmart(itsHubAddress, {
            token_config: { chain: chainName, token_id: tokenId },
        });

        printInfo(`Custom token metadata for ${tokenId} on ${chainName}`, JSON.stringify(result, null, 2));
    } catch (error) {
        printWarn(`Failed to fetch custom token metadata for ${tokenId} on ${chainName}`, error?.message || String(error));
    }
}

async function deployedContracts(client, config, args, options) {
    const { chainName } = options;

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

    saveConfig(config, options.env);
    printInfo(`Config updated successfully for ${chainName}`);
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
        .command('rewards')
        .description('Query rewards pool state for multisig and voting_verifier contracts')
        .action((options) => {
            mainProcessor(rewards, [], options);
        });

    const tokenConfigCmd = program
        .command('token-config <chainName> <tokenId>')
        .description('Query custom token metadata from ITS Hub')
        .action((chainName, tokenId, options) => {
            mainProcessor(tokenConfig, [chainName, tokenId], options);
        });

    const saveDeployedContractsCmd = program
        .command('save-deployed-contracts')
        .description('Query and save deployed Gateway, VotingVerifier and MultisigProver contracts via Coordinator')
        .requiredOption('-n, --chainName <chainName>', 'chain name')
        .action((options) => {
            mainProcessor(deployedContracts, [], options);
        });

    addAmplifierQueryOptions(rewardCmd);
    addAmplifierQueryOptions(tokenConfigCmd);
    addAmplifierQueryOptions(saveDeployedContractsCmd);

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

module.exports = {
    getItsChainConfig,
};
