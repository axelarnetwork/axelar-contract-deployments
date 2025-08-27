import { query_chains_from_router, ChainEndpoint, SigningCosmWasmClient } from './utils';
import { ContractInfo } from '../contract';

interface Options {
    env: string; 
    mnemonic: string;
    address: string;
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

async function construct_chain_contracts(
    client: typeof SigningCosmWasmClient,
    chain_endpoints: ChainEndpoint[],
): Promise<ChainContracts[]> {
    return new Promise(async (resolve, reject) => {
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

            resolve(chain_contracts);
        } catch (e) {
            reject(e);
        }
    });
}

function add_missing_provers(
    client: typeof SigningCosmWasmClient,
    multisig_address: string,
    chain_contracts: ChainContracts[],
): Promise<ChainContracts[]> {
    return new Promise(async (resolve, _) => {
        for (let i = 0; i < chain_contracts.length; i++) {
            const authorized_provers = await client.queryContractSmart(multisig_address, {
                authorized_callers: { chain_name: chain_contracts[i].chain_name },
            });
            chain_contracts[i].prover_address = authorized_provers[0] ?? '';
        }

        resolve(chain_contracts);
    });
}

function missing_chain(error_message: string): string | null {
    const re = new RegExp('missing contracts to register for chain (?<chain>[a-z0-9]+):');
    const result = error_message.match(re);
    if (!result.groups.chain) {
        return null;
    }

    return result.groups.chain;
}

export async function migrate(client: typeof SigningCosmWasmClient, options: Options, config: any, sender_address: string, coordinator_address: string, version: string, code_id: number) {
    const router_address = config.axelar.contracts.Router.address;
    const multisig_address = config.axelar.contracts.Multisig.address;
    coordinator_address = options.address ?? config.axelar.contracts.Coordinator.address;

    const chain_endpoints = await query_chains_from_router(client, router_address);
    let chain_contracts = await construct_chain_contracts(client, chain_endpoints);
    chain_contracts = await add_missing_provers(client, multisig_address, chain_contracts);

    const migration_msg = {
        router: router_address,
        multisig: multisig_address,
        chain_contracts: chain_contracts,
    };

    console.log('Migration Msg:', migration_msg);

    if (!options.dry) {
        while (true) {
            try {
                console.log('Executing migration...');
                await client.migrate(sender_address, coordinator_address, Number(code_id), migration_msg, options.fees);
                console.log('Migration succeeded');
                break;
            } catch (e) {
                // Devnet has some incomplete registrations where a chain may have a prover, but
                // no gateway or verifier. We must supply dummy data in those cases. Those addresses
                // must be correctly encoded, so we can use the coordinator address by default.
                const chain_to_add = missing_chain(e.message);
                if (!options.dummy || !chain_to_add) {
                    console.log('Migration failed:', e.message);

                    if (chain_to_add) {
                        console.log("Set the '--dummy' flag to use dummy data for missing chains");
                    }
                    break;
                }

                console.log(`Missing information for chain ${chain_to_add}`);

                const dummy_data = {
                    chain_name: chain_to_add,
                    gateway_address: coordinator_address,
                    verifier_address: coordinator_address,
                };

                console.log(`Adding dummy data for ${JSON.stringify(dummy_data)}...`);

                migration_msg.chain_contracts.push(dummy_data);
            }
        }
    }
}
