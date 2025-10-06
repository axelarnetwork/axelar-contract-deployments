import { StdFee } from '@cosmjs/stargate';

import { encodeMigrateContractProposal, submitProposal } from '../utils';
import { MigrationOptions } from './types';

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

export async function queryChainsFromRouter(client: typeof SigningCosmWasmClient, routerAddress: string): Promise<ChainEndpoint[]> {
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
