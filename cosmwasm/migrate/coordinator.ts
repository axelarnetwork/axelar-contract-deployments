import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { StdFee } from '@cosmjs/stargate';
import { AccessType } from 'cosmjs-types/cosmwasm/wasm/v1/types';

import { printError, printInfo } from '../../common';
import { encodeMigrateContractProposal, encodeUpdateInstantiateConfigProposal, submitProposal } from '../utils';
import { MigrationOptions, ProtocolContracts } from './types';

// eslint-disable-next-line @typescript-eslint/no-require-imports
export const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

interface ChainContracts {
    chain_name: string;
    prover_address?: string;
    gateway_address: string;
    verifier_address: string;
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
                        chain_name: endpoint.name,
                        gateway_address: endpoint.gateway.address,
                        verifier_address: config.verifier,
                        prover_address: authorizedProvers ?? '',
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

        if (!chainInfo.prover_address) {
            throw new Error(`missing prover for chain ${endpoint.name}`);
        }

        chainProverPairs.set(endpoint.name, chainInfo.prover_address);
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

async function checkCoordinatorToVersion2_1(client: CosmWasmClient, config, coordinatorAddress?: string, multisigAddress?: string) {
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
    const v2_1_re = /2\.1\.\d+/;

    if (version.match(v2_1_re)) {
        return checkCoordinatorToVersion2_1(client, config, coordinatorAddress, multisigAddress);
    } else {
        printError(`no migration check script found for coordinator ${version}`);
    }
}

export async function instantiatePermissions(
    client: typeof SigningCosmWasmClient,
    options: MigrationOptions,
    config,
    senderAddress: string,
    coordinatorAddress: string,
    permittedAddresses: string[],
    codeId: number,
    fee: string | StdFee,
) {
    permittedAddresses.push(coordinatorAddress);

    const updateMsg: string = JSON.stringify([
        {
            codeId: codeId,
            instantiatePermission: {
                permission: AccessType.ACCESS_TYPE_ANY_OF_ADDRESSES,
                addresses: permittedAddresses,
            },
        },
    ]);

    printInfo(`Update Msg: ${JSON.stringify(updateMsg)}`);

    const updateOptions = {
        msg: updateMsg,
        title: options.title,
        description: options.description,
        runAs: senderAddress,
        deposit: options.deposit,
    };

    const proposal = encodeUpdateInstantiateConfigProposal(updateOptions);

    if (!options.dry) {
        try {
            printInfo(`Executing migration...\n${JSON.stringify(updateOptions)}`);
            await submitProposal(client, config, updateOptions, proposal, fee);
            printInfo('Migration proposal successfully submitted');
        } catch (e) {
            printError(`Error: ${e}`);
        }
    }
}
