import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { encodeMigrateContractProposal, submitProposal } from '../utils';
import { MigrationOptions } from './types';

// eslint-disable-next-line @typescript-eslint/no-require-imports
export const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

interface ChainContracts {
    chain_name: string;
    prover_address?: string;
    gateway_address: string;
    verifier_address: string;
}

interface ChainProverPair {
    chain: string,
    prover: string,
}

export interface ChainEndpoint {
    name: string;
    gateway: {
        address: string;
    };
}

export async function queryChainsFromRouter(client: CosmWasmClient, router_address: string): Promise<ChainEndpoint[]> {
    try {
        const res: ChainEndpoint[] = await client.queryContractSmart(router_address, { chains: {} });
        return res;
    } catch (error) {
        throw error;
    }
}

function check_for_duplicates(chains: ChainContracts[]) {
    const provers: Map<string, string[]> = new Map();
    const verifiers: Map<string, string[]> = new Map();
    const gateways: Map<string, string[]> = new Map();

    chains.forEach((c) => {
        if (c.prover_address && !provers.has(c.prover_address)) {
            provers.set(c.prover_address, [c.chain_name]);
        } else if (provers.has(c.prover_address)) {
            provers.set(c.prover_address, provers.get(c.prover_address).concat([c.chain_name]));
        }

        if (!verifiers.has(c.verifier_address)) {
            verifiers.set(c.verifier_address, [c.chain_name]);
        } else if (verifiers.has(c.verifier_address)) {
            verifiers.set(c.verifier_address, verifiers.get(c.verifier_address).concat([c.chain_name]));
        }

        if (!gateways.has(c.gateway_address)) {
            gateways.set(c.gateway_address, [c.chain_name]);
        } else if (gateways.has(c.gateway_address)) {
            gateways.set(c.gateway_address, gateways.get(c.gateway_address).concat([c.chain_name]));
        }
    });

    let duplicates_found = false;

    provers.forEach((v, k) => {
        if (v.length > 1) {
            duplicates_found = true;
            console.log(`Prover ${k} duplicated between ${v}`);
        }
    });

    verifiers.forEach((v, k) => {
        if (v.length > 1) {
            duplicates_found = true;
            console.log(`Verifier ${k} duplicated between ${v}`);
        }
    });

    gateways.forEach((v, k) => {
        if (v.length > 1) {
            duplicates_found = true;
            console.log(`Gateway ${k} duplicated between ${v}`);
        }
    });

    if (duplicates_found) {
        throw new Error('uniqueness constraints not maintained for chain contracts');
    }
}

async function constructChainContracts(
    client: typeof SigningCosmWasmClient,
    multisig_address: string,
    chain_endpoints: ChainEndpoint[],
    ignore_chains: string[],
): Promise<ChainContracts[]> {
    try {
        interface GatewayConfig {
            verifier: string;
        }

        const chain_contracts: ChainContracts[] = [];

        for (let i = 0; i < chain_endpoints.length; i++) {
            try {
                const res = await client.queryContractRaw(chain_endpoints[i].gateway.address, Buffer.from('config'));
                const config: GatewayConfig = JSON.parse(Buffer.from(res).toString('ascii'));
                if (
                    chain_endpoints[i].name &&
                    !ignore_chains.includes(chain_endpoints[i].name) &&
                    chain_endpoints[i].gateway.address &&
                    config.verifier
                ) {
                    const authorized_provers = await client.queryContractSmart(multisig_address, {
                        authorized_caller: { chain_name: chain_endpoints[i].name },
                    });

                    chain_contracts.push({
                        chain_name: chain_endpoints[i].name,
                        gateway_address: chain_endpoints[i].gateway.address,
                        verifier_address: config.verifier,
                        prover_address: authorized_provers ?? '',
                    });
                }
            } catch (e) {
                console.log(`Warning: ${e}`);
            }
        }

        check_for_duplicates(chain_contracts);

        return chain_contracts;
    } catch (e) {
        throw e;
    }
}

async function constructCoordinatorChainProverPairs(
    client: CosmWasmClient,
    coordinator_address: string,
    router_address: string,
): Promise<ChainProverPair[]> {
    const all_chains = await queryChainsFromRouter(client, router_address);
    const chain_prover_pairs: ChainProverPair[] = [];

    for (let i = 0; i < all_chains.length; i++) {
        try {
            const chain_info: ChainContracts = await client.queryContractSmart(coordinator_address, {
                chain_contracts_info: { chain_name: all_chains[i].name},
            });

            if (!chain_info.prover_address) {
                throw new Error(`missing prover for chain ${all_chains[i].name}`);
            }

            chain_prover_pairs.push({
                chain: all_chains[i].name,
                prover: chain_info.prover_address,
            });
        } catch (e) {
            console.error(`Error querying contracts for chain ${all_chains[i].name}: ${e}`)
        }
    }

    return chain_prover_pairs;
}

async function coordinatorToVersion2_1_0(
    client: typeof SigningCosmWasmClient,
    options: MigrationOptions,
    config,
    sender_address: string,
    coordinator_address: string,
    code_id: number,
) {
    const router_address = config.axelar.contracts.Router.address;
    const multisig_address = config.axelar.contracts.Multisig.address;
    const ignore: string[] = options.ignoreChains ? JSON.parse(options.ignoreChains) : [];

    const chain_endpoints = await queryChainsFromRouter(client, router_address);
    const chain_contracts = await constructChainContracts(client, multisig_address, chain_endpoints, ignore);

    const migration_msg = {
        router: router_address,
        multisig: multisig_address,
        chain_contracts: chain_contracts,
    };

    console.log('Migration Msg:', migration_msg);

    const migrate_options = {
        contractName: 'Coordinator',
        msg: JSON.stringify(migration_msg),
        title: 'Migrate Coordinator v2.1.0',
        description: 'Migrate Coordinator v2.1.0',
        runAs: sender_address,
        codeId: code_id,
        deposit: options.deposit,
        fetchCodeId: false,
        address: coordinator_address,
    };

    const proposal = encodeMigrateContractProposal(config, migrate_options);

    if (!options.dry) {
        try {
            console.log('Executing migration...', migrate_options);
            if (options.proposal) {
                await submitProposal(client, config, migrate_options, proposal, options.fees);
                console.log('Migration proposal successfully submitted');
            } else {
                await client.migrate(sender_address, coordinator_address, Number(code_id), migration_msg, options.fees);
                console.log('Migration succeeded');
            }
        } catch (e) {
            console.log('Error:', e);
        }
    }
}

async function checkCoordinatorToVersion2_1_0(
    client: CosmWasmClient,
    config,
    coordinator_address?: string,
    multisig_address?: string,
) {
    coordinator_address = coordinator_address ?? config.axelar.contracts.Coordinator.address;
    multisig_address = multisig_address ?? config.axelar.contracts.Multisig.address;
    const router_address = config.axelar.contracts.Router.address;

    console.log(await constructCoordinatorChainProverPairs(client, coordinator_address, router_address));
}

export async function migrate(
    client: typeof SigningCosmWasmClient,
    options: MigrationOptions,
    config,
    sender_address: string,
    coordinator_address: string,
    version: string,
    code_id: number,
) {
    switch (version) {
        case '1.1.0':
            return coordinatorToVersion2_1_0(client, options, config, sender_address, coordinator_address, code_id);
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
