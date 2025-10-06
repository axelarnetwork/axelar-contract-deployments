import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { StdFee } from '@cosmjs/stargate';

import { encodeMigrateContractProposal, submitProposal } from '../utils';
import { MigrationOptions, ProtocolContracts } from './types';

// eslint-disable-next-line @typescript-eslint/no-require-imports
export const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

interface ChainContracts {
    chainName: string;
    proverAddress?: string;
    gatewayAddress: string;
    verifierAddress: string;
}

export interface ChainEndpoint {
    name: string;
    gateway: {
        address: string;
    };
}

export async function queryChainsFromRouter(client: CosmWasmClient, router_address: string): Promise<ChainEndpoint[]> {
    try {
        const res: ChainEndpoint[] = await client.queryContractSmart(routerAddress, { chains: {} });
        return res;
    } catch (error) {
        throw error;
    }
}

function checkForDuplicates(chains: ChainContracts[]) {
    const provers: Map<string, string[]> = new Map();
    const verifiers: Map<string, string[]> = new Map();
    const gateways: Map<string, string[]> = new Map();

    chains.forEach((c) => {
        if (c.proverAddress && !provers.has(c.proverAddress)) {
            provers.set(c.proverAddress, [c.chainName]);
        } else if (provers.has(c.proverAddress)) {
            provers.set(c.proverAddress, provers.get(c.proverAddress).concat([c.chainName]));
        }

        if (!verifiers.has(c.verifierAddress)) {
            verifiers.set(c.verifierAddress, [c.chainName]);
        } else if (verifiers.has(c.verifierAddress)) {
            verifiers.set(c.verifierAddress, verifiers.get(c.verifierAddress).concat([c.chainName]));
        }

        if (!gateways.has(c.gatewayAddress)) {
            gateways.set(c.gatewayAddress, [c.chainName]);
        } else if (gateways.has(c.gatewayAddress)) {
            gateways.set(c.gatewayAddress, gateways.get(c.gatewayAddress).concat([c.chainName]));
        }
    });

    let duplicatesFound = false;

    provers.forEach((v, k) => {
        if (v.length > 1) {
            duplicatesFound = true;
            console.log(`Prover ${k} duplicated between ${v}`);
        }
    });

    verifiers.forEach((v, k) => {
        if (v.length > 1) {
            duplicatesFound = true;
            console.log(`Verifier ${k} duplicated between ${v}`);
        }
    });

    gateways.forEach((v, k) => {
        if (v.length > 1) {
            duplicatesFound = true;
            console.log(`Gateway ${k} duplicated between ${v}`);
        }
    });

    if (duplicatesFound) {
        throw new Error('uniqueness constraints not maintained for chain contracts');
    }
}

async function constructChainContracts(
    client: typeof SigningCosmWasmClient,
    multisigAddress: string,
    chainEndpoints: ChainEndpoint[],
    ignoreChains: string[],
): Promise<ChainContracts[]> {
    try {
        interface GatewayConfig {
            verifier: string;
        }

        const chainContracts: ChainContracts[] = [];

        for (const endpoint of chainEndpoints) {
            try {
                const res = await client.queryContractRaw(endpoint.gateway.address, Buffer.from('config'));
                const config: GatewayConfig = JSON.parse(Buffer.from(res).toString('ascii'));
                if (endpoint.name && !ignoreChains.includes(endpoint.name) && endpoint.gateway.address && config.verifier) {
                    const authorizedProvers = await client.queryContractSmart(multisigAddress, {
                        authorized_caller: { chain_name: endpoint.name },
                    });

                    chainContracts.push({
                        chainName: endpoint.name,
                        gatewayAddress: endpoint.gateway.address,
                        verifierAddress: config.verifier,
                        proverAddress: authorizedProvers ?? '',
                    });
                }
            } catch (e) {
                console.log(`Warning: ${e}`);
            }
        }

        checkForDuplicates(chainContracts);

        return chainContracts;
    } catch (e) {
        throw e;
    }
}

async function constructCoordinatorChainProverPairs(
    client: CosmWasmClient,
    coordinator_address: string,
    router_address: string,
): Promise<Map<string, string>> {
    const all_chains = await queryChainsFromRouter(client, router_address);
    const chain_prover_pairs: Map<string, string> = new Map();

    for (let i = 0; i < all_chains.length; i++) {
        let chain_info: ChainContracts;
        try {
            chain_info = await client.queryContractSmart(coordinator_address, {
                chain_contracts_info: { chain_name: all_chains[i].name },
            });
        } catch (e) {
            // Chain exists in router, but does not exist in the coordinator
            // This is not a critical/migration error, so continue
            continue;
        }

        if (!chain_info.prover_address) {
            throw new Error(`missing prover for chain ${all_chains[i].name}`);
        }

        chain_prover_pairs.set(all_chains[i].name, chain_info.prover_address);
    }

    return chain_prover_pairs;
}

async function constructMultisigChainProverPairs(
    client: CosmWasmClient,
    multisig_address: string,
    router_address: string,
): Promise<Map<string, string>> {
    const all_chains = await queryChainsFromRouter(client, router_address);
    const chain_prover_pairs: Map<string, string> = new Map();

    for (let i = 0; i < all_chains.length; i++) {
        let prover_addr: string;

        try {
            prover_addr = await client.queryContractSmart(multisig_address, {
                authorized_caller: { chain_name: all_chains[i].name },
            });
        } catch (e) {
            if (e.toString().includes('unknown variant')) {
                throw new Error('Multisig version must be >=2.3.0. please check multisig address');
            }

            // Chain exists in router, but does not exist in the multisig
            // This is not a critical/migration error, so continue
            continue;
        }

        chain_prover_pairs.set(all_chains[i].name, prover_addr);
    }

    return chain_prover_pairs;
}

async function coordinatorStoresMultisigAddress(
    client: CosmWasmClient,
    coordinator_address: string,
    multisig_address: string,
): Promise<boolean> {
    const res = await client.queryContractRaw(coordinator_address, Buffer.from('protocol'));
    const protocol_contracts: ProtocolContracts = JSON.parse(Buffer.from(res).toString('ascii'));
    if (protocol_contracts.multisig != multisig_address) {
        console.log(`Coordinator stores incorrect multisig address: expected ${multisig_address}, saw ${protocol_contracts.multisig}`);
        return false;
    }

    return true;
}

async function coordinatorToVersion2_1_0(
    client: typeof SigningCosmWasmClient,
    options: MigrationOptions,
    config,
    senderAddress: string,
    coordinatorAddress: string,
    codeId: number,
    fee: string | StdFee,
) {
    const routerAddress = config.axelar.contracts.Router.address;
    const multisigAddress = config.axelar.contracts.Multisig.address;
    const ignore: string[] = options.ignoreChains ? JSON.parse(options.ignoreChains) : [];

    const chainEndpoints = await queryChainsFromRouter(client, routerAddress);
    const chainContracts = await constructChainContracts(client, multisigAddress, chainEndpoints, ignore);

    const migrationMsg = {
        router: routerAddress,
        multisig: multisigAddress,
        chain_contracts: chainContracts,
    };

    console.log('Migration Msg:', migrationMsg);

    const migrateOptions = {
        contractName: 'Coordinator',
        msg: JSON.stringify(migrationMsg),
        title: 'Migrate Coordinator v2.1.1',
        description: 'Migrate Coordinator v2.1.1',
        runAs: senderAddress,
        codeId: codeId,
        deposit: options.deposit,
        fetchCodeId: false,
        address: coordinatorAddress,
    };

    const proposal = encodeMigrateContractProposal(config, migrateOptions);

    if (!options.dry) {
        try {
            console.log('Executing migration...', migrateOptions);
            if (options.direct) {
                await client.migrate(senderAddress, coordinatorAddress, Number(codeId), migrationMsg, fee);
                console.log('Migration succeeded');
            } else {
                await submitProposal(client, config, migrateOptions, proposal, fee);
                console.log('Migration proposal successfully submitted');
            }
        } catch (e) {
            console.log('Error:', e);
        }
    }
}

async function checkCoordinatorToVersion2_1_0(client: CosmWasmClient, config, coordinator_address?: string, multisig_address?: string) {
    coordinator_address = coordinator_address ?? config.axelar.contracts.Coordinator.address;
    multisig_address = multisig_address ?? config.axelar.contracts.Multisig.address;
    const router_address = config.axelar.contracts.Router.address;
    let state_is_consistent = true;

    try {
        const coordinator_map_promise = constructCoordinatorChainProverPairs(client, coordinator_address, router_address);
        const multisig_map = await constructMultisigChainProverPairs(client, multisig_address, router_address);

        if (!(await coordinatorStoresMultisigAddress(client, coordinator_address, multisig_address))) {
            state_is_consistent = false;
        }

        const coordinator_map = await coordinator_map_promise;

        for (const [chain, prover] of coordinator_map.entries()) {
            if (!multisig_map.has(chain)) {
                console.log(`Multisig Missing chain ${chain}`);
                state_is_consistent = false;
                continue;
            }

            const prover_seen = multisig_map.get(chain);
            if (prover_seen !== prover) {
                console.log(`Coordinator's prover does not match multisig's for chain ${chain}: expected ${prover_seen}, saw ${prover}`);
                state_is_consistent = false;
                continue;
            }
        }

        if (!state_is_consistent) {
            console.error(`❌ State of coordinator v2 is not consistent with the rest of the protocol`);
        } else {
            console.log(`✅ Migration succeeded!`);
        }
    } catch (e) {
        // These errors should never happen, as it would indicate a critical problem in the
        // Amplifier that would likely require manual intervention.
        console.log(`Critical - ${e}`);
    }
}

export async function migrate(
    client: typeof SigningCosmWasmClient,
    options: MigrationOptions,
    config,
    senderAddress: string,
    coordinatorAddress: string,
    version: string,
    codeId: number,
    fee: string | StdFee,
) {
    switch (version) {
        case '1.1.0':
            return coordinatorToVersion2_1_0(client, options, config, senderAddress, coordinatorAddress, codeId, fee);
        default:
            console.error(`no migration script found for coordinator ${version}`);
    }
}

export async function checkMigration(
    client: CosmWasmClient,
    config,
    version: string,
    coordinator_address?: string,
    multisig_address?: string,
) {
    switch (version) {
        case '2.1.0':
            return checkCoordinatorToVersion2_1_0(client, config, coordinator_address, multisig_address);
        default:
            console.error(`no migration check script found for coordinator ${version}`);
    }
}
