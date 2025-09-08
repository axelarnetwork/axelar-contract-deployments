import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { encodeMigrateContractProposal, submitProposal } from '../utils';

// cosmwasm-stargate imports protobufjs which does not have a default export
// Therefore, import SigningCosmWasmClient using CommonJS to avoid error TS1192
// eslint-disable-next-line @typescript-eslint/no-require-imports
export const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

interface Options {
    env: string;
    mnemonic: string;
    address: string;
    deposit: string;
    fees;
    dry?;
    dummy?;
}

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

export async function queryChainsFromRouter(client: typeof SigningCosmWasmClient, router_address: string): Promise<ChainEndpoint[]> {
    try {
        const res: ChainEndpoint[] = await client.queryContractSmart(router_address, { chains: {} });
        return res;
    } catch (error) {
        throw error;
    }
}


async function constructChainContracts(client: typeof SigningCosmWasmClient, chain_endpoints: ChainEndpoint[]): Promise<ChainContracts[]> {
    interface GatewayConfig {
        verifier: string;
    }

    try {
        const chain_contracts: ChainContracts[] = [];

        for (let i = 0; i < chain_endpoints.length; i++) {
            const res = await client.queryContractRaw(chain_endpoints[i].gateway.address, Buffer.from('config'));
            const config: GatewayConfig = JSON.parse(Buffer.from(res).toString('ascii'));
            if (chain_endpoints[i].name && chain_endpoints[i].gateway.address && config.verifier) {
                chain_contracts.push({
                    chain_name: chain_endpoints[i].name,
                    gateway_address: chain_endpoints[i].gateway.address,
                    verifier_address: config.verifier,
                });
            }
        }

        return chain_contracts;
    } catch (e) {
        throw e;
    }
}

async function addMissingProvers(
    client: typeof SigningCosmWasmClient,
    multisig_address: string,
    chain_contracts: ChainContracts[],
): Promise<ChainContracts[]> {
    try {
        for (let i = 0; i < chain_contracts.length; i++) {
            const authorized_provers = await client.queryContractSmart(multisig_address, {
                authorized_callers: { chain_name: chain_contracts[i].chain_name },
            });
            chain_contracts[i].prover_address = authorized_provers[0] ?? '';
        }

        return chain_contracts;
    } catch (e) {
        throw e;
    }
}

async function coordinatorToVersion2_1_0(
    client: typeof SigningCosmWasmClient,
    wallet: DirectSecp256k1HdWallet,
    options: Options,
    config,
    sender_address: string,
    coordinator_address: string,
    code_id: number,
) {
    const router_address = config.axelar.contracts.Router.address;
    const multisig_address = config.axelar.contracts.Multisig.address;

    const chain_endpoints = await queryChainsFromRouter(client, router_address);
    let chain_contracts = await constructChainContracts(client, chain_endpoints);
    chain_contracts = await addMissingProvers(client, multisig_address, chain_contracts);

    const migration_msg = {
        router: router_address,
        multisig: multisig_address,
        chain_contracts: chain_contracts,
    };

    console.log('Migration Msg:', migration_msg);

    let migrate_options = {
        contractName: "Coordinator",
        msg: JSON.stringify(migration_msg),
        title: "Migrate Coordinator v2.1.0",
        description: "Migrate Coordinator v2.1.0",
        runAs: sender_address,
        codeId: code_id,
        deposit: options.deposit,
        fetchCodeId: false,
        address: coordinator_address
    }
    config.contractName = "coordinator";

    let proposal = encodeMigrateContractProposal(config, migrate_options);

    if (!options.dry) {
        try {
            console.log('Executing migration...');
            await submitProposal(client, wallet, config, migrate_options, proposal);
            console.log('Migration succeeded');
        } catch (e) {
            console.log("Error:", e);
        }
    }
}

export async function migrate(
    client: typeof SigningCosmWasmClient,
    wallet: DirectSecp256k1HdWallet,
    options: Options,
    config,
    sender_address: string,
    coordinator_address: string,
    version: string,
    code_id: number,
) {
    switch (version) {
        case '1.1.0':
            return coordinatorToVersion2_1_0(client, wallet, options, config, sender_address, coordinator_address, code_id);
        default:
            console.error(`no migration script found for coordinator ${version}`);
    }
}
