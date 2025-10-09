import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { StdFee } from '@cosmjs/stargate';

import { printError, printInfo } from '../../common';
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

export async function queryChainsFromRouter(client: CosmWasmClient, routerAddress: string): Promise<ChainEndpoint[]> {
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
            printInfo(`Prover ${k} duplicated between ${v}`);
        }
    });

    verifiers.forEach((v, k) => {
        if (v.length > 1) {
            duplicatesFound = true;
            printInfo(`Verifier ${k} duplicated between ${v}`);
        }
    });

    gateways.forEach((v, k) => {
        if (v.length > 1) {
            duplicatesFound = true;
            printInfo(`Gateway ${k} duplicated between ${v}`);
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
                printError(`Warning: ${e}`);
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
    coordinatorAddress: string,
    routerAddress: string,
): Promise<Map<string, string>> {
    const allChains = await queryChainsFromRouter(client, routerAddress);
    const chainProverPairs: Map<string, string> = new Map();

    for (const endpoint of allChains) {
        let chainInfo: ChainContracts;
        try {
            chainInfo = await client.queryContractSmart(coordinatorAddress, {
                chain_contracts_info: { chain_name: endpoint.name },
            });
        } catch (e) {
            // Chain exists in router, but does not exist in the coordinator
            // This is not a critical/migration error, so continue
            continue;
        }

        if (!chainInfo.proverAddress) {
            throw new Error(`missing prover for chain ${endpoint.name}`);
        }

        chainProverPairs.set(endpoint.name, chainInfo.proverAddress);
    }

    return chainProverPairs;
}

async function constructMultisigChainProverPairs(
    client: CosmWasmClient,
    multisigAddress: string,
    routerAddress: string,
): Promise<Map<string, string>> {
    const allChains = await queryChainsFromRouter(client, routerAddress);
    const chainProverPairs: Map<string, string> = new Map();

    for (const endpoint of allChains) {
        let proverAddr: string;

        try {
            proverAddr = await client.queryContractSmart(multisigAddress, {
                authorized_caller: { chain_name: endpoint.name },
            });
        } catch (e) {
            if (e.toString().includes('unknown variant')) {
                throw new Error('Multisig version must be >=2.3.0. please check multisig address');
            }

            // Chain exists in router, but does not exist in the multisig
            // This is not a critical/migration error, so continue
            continue;
        }

        chainProverPairs.set(endpoint.name, proverAddr);
    }

    return chainProverPairs;
}

async function coordinatorStoresMultisigAddress(
    client: CosmWasmClient,
    coordinatorAddress: string,
    multisigAddress: string,
): Promise<boolean> {
    const res = await client.queryContractRaw(coordinatorAddress, Buffer.from('protocol'));
    const protocolContracts: ProtocolContracts = JSON.parse(Buffer.from(res).toString('ascii'));
    if (protocolContracts.multisig !== multisigAddress) {
        printError(`Coordinator stores incorrect multisig address: expected ${multisigAddress}, saw ${protocolContracts.multisig}`);
        return false;
    }

    return true;
}

async function coordinatorToVersion2_1_1(
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

    printInfo(`Migration Msg: ${JSON.stringify(migrationMsg)}`);

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
            printInfo(`Executing migration...\n${JSON.stringify(migrateOptions)}`);
            if (options.direct) {
                await client.migrate(senderAddress, coordinatorAddress, Number(codeId), migrationMsg, fee);
                printInfo('Migration succeeded');
            } else {
                await submitProposal(client, config, migrateOptions, proposal, fee);
                printInfo('Migration proposal successfully submitted');
            }
        } catch (e) {
            printError(`Error: ${e}`);
        }
    }
}

async function checkCoordinatorToVersion2_1_0(client: CosmWasmClient, config, coordinatorAddress?: string, multisigAddress?: string) {
    coordinatorAddress = coordinatorAddress ?? config.axelar.contracts.Coordinator.address;
    multisigAddress = multisigAddress ?? config.axelar.contracts.Multisig.address;
    const routerAddress = config.axelar.contracts.Router.address;
    let stateIsConsistent = true;

    try {
        const coordinatorMapPromise = constructCoordinatorChainProverPairs(client, coordinatorAddress, routerAddress);
        const multisigMap = await constructMultisigChainProverPairs(client, multisigAddress, routerAddress);

        if (!(await coordinatorStoresMultisigAddress(client, coordinatorAddress, multisigAddress))) {
            stateIsConsistent = false;
        }

        const coordinatorMap = await coordinatorMapPromise;

        for (const [chain, prover] of coordinatorMap.entries()) {
            if (!multisigMap.has(chain)) {
                printInfo(`Multisig Missing chain ${chain}`);
                stateIsConsistent = false;
                continue;
            }

            const proverSeen = multisigMap.get(chain);
            if (proverSeen !== prover) {
                printInfo(
                    `Coordinator's prover does not match multisig's for chain ${chain}: prover in multisig ${proverSeen}, prover in coordinator ${prover}`,
                );
                stateIsConsistent = false;
                continue;
            }
        }

        if (!stateIsConsistent) {
            printError(`❌ State of coordinator v2 is not consistent with the rest of the protocol`);
        } else {
            printInfo(`✅ Migration succeeded!`);
        }
    } catch (e) {
        // These errors should never happen, as it would indicate a critical problem in the
        // Amplifier that would likely require manual intervention.
        printError(`Critical - ${e}`);
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
            return coordinatorToVersion2_1_1(client, options, config, senderAddress, coordinatorAddress, codeId, fee);
        default:
            printError(`no migration script found for coordinator ${version}`);
    }
}

export async function checkMigration(
    client: CosmWasmClient,
    config,
    version: string,
    coordinatorAddress?: string,
    multisigAddress?: string,
) {
    switch (version) {
        case '2.1.0':
            return checkCoordinatorToVersion2_1_0(client, config, coordinatorAddress, multisigAddress);
        default:
            printError(`no migration check script found for coordinator ${version}`);
    }
}
