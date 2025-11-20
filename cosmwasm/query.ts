'use strict';

import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { Command } from 'commander';

import { addEnvOption, getChainConfig, itsHubContractAddress, printError, printInfo, printWarn } from '../common';
import { ConfigManager, ContractConfig } from '../common/config';
import { addAmplifierQueryContractOptions, addAmplifierQueryOptions } from './cli-utils';
import { Options, mainQueryProcessor } from './processor';

export interface ContractInfo {
    contract: string;
    version: string;
}

export interface RewardsPoolResponse {
    balance: string;
    epoch_duration: string;
    participation_threshold: [string, string];
    rewards_per_epoch: string;
    current_epoch_num: string;
    last_distribution_epoch: string | null;
}

export async function queryRewardsPool(
    client: CosmWasmClient,
    rewardsAddress: string,
    chainName: string,
    contractAddress: string,
): Promise<RewardsPoolResponse> {
    return await client.queryContractSmart(rewardsAddress, {
        rewards_pool: {
            pool_id: {
                chain_name: chainName,
                contract: contractAddress,
            },
        },
    });
}

async function rewards(client, config, _options, args, _fee) {
    const [chainName] = args;
    const rewardsAddress = config.getContractConfig('Rewards').address;

    const rewardsContractAddresses = {
        multisig: config.getContractConfig('Multisig').address,
        voting_verifier: config.getVotingVerifierContract(chainName).address,
    };

    for (const [key, address] of Object.entries(rewardsContractAddresses)) {
        try {
            const result = await queryRewardsPool(client, rewardsAddress, chainName, address);
            printInfo(`Rewards pool for ${key} on ${chainName}`, JSON.stringify(result, null, 2));
        } catch (error) {
            printWarn(`Failed to fetch rewards pool for ${key} on ${chainName}`, `${error.message}`);
        }
    }
}

export async function getItsChainConfig(client, config, chainName) {
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

    const coordinatorAddress = config.getContractConfig('Coordinator').address;
    if (!coordinatorAddress) {
        return printWarn(`Coordinator contract address not found in config for ${chainName}`);
    }

    const deploymentName = config.getContractConfig('Coordinator').deployments?.[chainName]?.deploymentName;
    if (!deploymentName) {
        return printWarn(
            `No deployment found for chain ${chainName} in config.`,
            `Run 'ts-node cosmwasm/submit-proposal.js instantiate-chain-contracts -n ${chainName}'.`,
        );
    }

    let result;
    try {
        result = await client.queryContractSmart(coordinatorAddress, {
            deployment: {
                deployment_name: deploymentName,
            },
        });

        printInfo(`Fetched deployed contracts for ${chainName}`, JSON.stringify(result, null, 2));
    } catch (error) {
        return printWarn(`Failed to fetch deployed contracts for ${chainName}`, error?.message || String(error));
    }

    if (!result.verifier_address || !result.prover_address || !result.gateway_address) {
        throw new Error(
            `Missing config for ${chainName}. Run 'ts-node cosmwasm/submit-proposal.js instantiate-chain-contracts -n ${chainName}' to instantiate the contracts.`,
        );
    }

    config.getVotingVerifierContract(chainName).address = result.verifier_address;
    config.getMultisigProverContract(chainName).address = result.prover_address;
    config.getGatewayContract(chainName).address = result.gateway_address;

    printInfo(`Updated VotingVerifier[${chainName}].address`, result.verifier_address);
    printInfo(`Updated MultisigProver[${chainName}].address`, result.prover_address);
    printInfo(`Updated Gateway[${chainName}].address`, result.gateway_address);
    printInfo(`Config updated successfully for ${chainName}`);
}

export async function getContractInfo(client: CosmWasmClient, contract_address: string): Promise<ContractInfo> {
    const result = await client.queryContractRaw(contract_address, Buffer.from('contract_info'));
    const contract_info: ContractInfo = JSON.parse(Buffer.from(result).toString('ascii'));
    return contract_info;
}

async function contractInfo(client: CosmWasmClient, config: ConfigManager, options: Options): Promise<void> {
    try {
        const address = config.getContractConfig(options.contractName).address;
        if (!address) {
            throw new Error(`No address configured for contract '${options.contractName}'`);
        }

        const contract_info: ContractInfo = await getContractInfo(client, address);
        console.log(contract_info);
    } catch (error) {
        console.error(error);
    }
}

async function queryAllContractVersions(
    client: CosmWasmClient,
    config: ConfigManager,
    _options: Options,
    _args?: string[],
    _fee?: unknown,
): Promise<void> {
    const axelarContracts = config.axelar.contracts;

    await Promise.all(
        Object.entries(axelarContracts).map(async ([contractName, contractConfig]: [string, ContractConfig]): Promise<void> => {
            if (contractConfig.address) {
                try {
                    const contractInfo = await getContractInfo(client, contractConfig.address);
                    contractConfig.version = contractInfo.version;
                } catch (error) {
                    printError(`Failed to get contract info for ${contractName}`, error);
                }
            }

            const chainNames = Object.entries(contractConfig).filter(([key, value]) => value.address);
            const versions = {} as Record<string, string[]>;
            await Promise.all(
                chainNames.map(async ([chainName, chainContractConfig]: [string, ContractConfig]): Promise<void> => {
                    try {
                        const contractInfo = await getContractInfo(client, chainContractConfig.address);
                        chainContractConfig.version = contractInfo.version;
                        if (!versions[contractInfo.version]) {
                            versions[contractInfo.version] = [];
                        }
                        versions[contractInfo.version].push(chainName);
                    } catch (error) {
                        printError(`Failed to get contract info for ${contractName} on ${chainName}`, error);
                    }
                }),
            );
            if (Object.keys(versions).length > 1) {
                printWarn(`${contractName} has different versions on different chains`, JSON.stringify(versions, null, 2));
            }
        }),
    );
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

    const contractInfoCmd = program
        .command('contract-info')
        .description('Query contract info')
        .action((options: Options) => {
            mainQueryProcessor(contractInfo, options, []);
        });

    const contractsVersions = program
        .command('contract-versions')
        .description('Query all cosmwasm axelar contract versions per environment')
        .action((options) => {
            mainQueryProcessor(queryAllContractVersions, options, []);
        });

    addEnvOption(contractsVersions);

    addAmplifierQueryOptions(rewardsCmd);
    addAmplifierQueryOptions(tokenConfigCmd);
    addAmplifierQueryOptions(customTokenMetadataCmd);
    addAmplifierQueryOptions(tokenInstanceCmd);
    addAmplifierQueryOptions(itsChainConfigCmd);
    addAmplifierQueryOptions(saveDeployedContractsCmd);
    addAmplifierQueryContractOptions(contractInfoCmd);

    program.parse();
};

if (require.main === module) {
    programHandler();
}
