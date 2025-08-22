'use strict';

const { prepareDummyWallet, prepareClient, initContractConfig } = require('./utils');
const { loadConfig, printInfo, printWarn } = require('../common');
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
    const { chainName, deploymentName } = options;
    const coordinatorAddress = config.axelar?.contracts?.Coordinator?.address;

    if (!coordinatorAddress) {
        printWarn('Coordinator contract address not found in config');
        return;
    }

    let deployment = deploymentName;
    if (!deployment && chainName) {
        deployment = config.axelar?.contracts?.Coordinator?.deployments?.[chainName]?.deploymentName;
        if (!deployment) {
            printWarn(`No deployment name found for chain ${chainName} in config`);
            return;
        }
    }

    if (!deployment) {
        printWarn('Deployment name is required. Use --deploymentName or --chainName with saved deployment');
        return;
    }

    try {
        const result = await client.queryContractSmart(coordinatorAddress, {
            deployed_contracts: {
                deployment_name: deployment,
            },
        });

        printInfo(`Deployed contracts for deployment "${deployment}"`, JSON.stringify(result, null, 2));

        if (chainName && result) {
            if (!config.axelar.contracts.VotingVerifier) {
                config.axelar.contracts.VotingVerifier = {};
            }
            if (!config.axelar.contracts.Gateway) {
                config.axelar.contracts.Gateway = {};
            }
            if (!config.axelar.contracts.MultisigProver) {
                config.axelar.contracts.MultisigProver = {};
            }

            if (result.verifier) {
                if (!config.axelar.contracts.VotingVerifier[chainName]) {
                    config.axelar.contracts.VotingVerifier[chainName] = {};
                }
                config.axelar.contracts.VotingVerifier[chainName].address = result.verifier;
                printInfo(`Updated VotingVerifier[${chainName}].address`, result.verifier);
            }

            if (result.gateway) {
                if (!config.axelar.contracts.Gateway[chainName]) {
                    config.axelar.contracts.Gateway[chainName] = {};
                }
                config.axelar.contracts.Gateway[chainName].address = result.gateway;
                printInfo(`Updated Gateway[${chainName}].address`, result.gateway);
            }

            if (result.prover) {
                if (!config.axelar.contracts.MultisigProver[chainName]) {
                    config.axelar.contracts.MultisigProver[chainName] = {};
                }
                config.axelar.contracts.MultisigProver[chainName].address = result.prover;
                printInfo(`Updated MultisigProver[${chainName}].address`, result.prover);
            }

            const { saveConfig } = require('../common');
            saveConfig(config, options.env);
            printInfo('Config updated successfully');
        }
    } catch (error) {
        printWarn(`Failed to fetch deployed contracts for deployment "${deployment}"`, error?.message || String(error));
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

    const deployedContractsCmd = program
        .command('deployed-contracts')
        .description('Query deployed Gateway, VotingVerifier and MultisigProver contracts via Coordinator')
        .option('-n, --chainName <chainName>', 'chain name of deployment')
        .option('--deploymentName <deploymentName>', 'deployment name to query')
        .action((options) => {
            mainProcessor(deployedContracts, [], options);
        });

    addAmplifierQueryOptions(rewardCmd);
    addAmplifierQueryOptions(tokenConfigCmd);
    addAmplifierQueryOptions(deployedContractsCmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}
