'use strict';

import { Command, Option } from 'commander';

import { loadConfig } from '../common';
import { addAmplifierOptions } from './cli-utils';
import { prepareClient, prepareWallet } from './utils';

// cosmwasm-stargate imports protobufjs which does not have a default export
// Therefore, import SigningCosmWasmClient using CommonJS to avoid error TS1192
// eslint-disable-next-line @typescript-eslint/no-require-imports
const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

interface ChainEndpoint {
    name: string;
    gateway: {
        address: string;
    };
}

interface ChainContracts {
    chain_name: string;
    gateway_address: string;
    verifier_address: string;
}

async function query_chains_from_router(client: typeof SigningCosmWasmClient, router_address: string): Promise<ChainEndpoint[]> {
    return new Promise(async (resolve, reject) => {
        try {
            const res: ChainEndpoint[] = await client.queryContractSmart(router_address, { chains: {} });
            resolve(res);
        } catch (e) {
            reject(e);
        }
    });
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

function missing_chain(error_message: string): string | null {
    const re = new RegExp('missing contracts to register for chain (?<chain>[a-z0-9]+):');
    const result = error_message.match(re);
    if (!result.groups.chain) {
        return null;
    }

    return result.groups.chain;
}

const programHandler = () => {
    const program = new Command();

    program.name('auto-migrate').version('1.0.0').description('Automation for migrating Amplifier contracts');

    addAmplifierOptions(
        program
            .command('migrate-coordinator')
            .argument('<code_id>', 'code id of new contract')
            .addOption(new Option('--fees <fees>', 'fees').default('auto'))
            .option('--dry', 'only generate migration msg')
            .option('--coordinator <address>', 'coordinator address')
            .option('--dummy', 'allow dummy data to be used when gateway or verifier is not present')
            .description('Migrate coordinator')
            .action(async (code_id: string, options: { env: string; mnemonic: string; fees; dry?; coordinator?: string; dummy? }) => {
                const { env } = options;
                const config = loadConfig(env);

                const wallet = await prepareWallet(options);
                const client = await prepareClient(config, wallet);
                const accounts = await wallet.getAccounts();
                if (accounts.length < 1) {
                    console.log('invalid mnemonic');
                    return;
                }

                const sender_address = accounts[0].address;
                const router_address = config.axelar.contracts.Router.address;
                const multisig_address = config.axelar.contracts.Multisig.address;
                const coordinator_address = options.coordinator ?? config.axelar.contracts.Coordinator.address;
                const dummy_address = coordinator_address;

                const chain_endpoints = await query_chains_from_router(client, router_address);
                const chain_contracts = await construct_chain_contracts(client, chain_endpoints);

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
                                gateway_address: dummy_address,
                                verifier_address: dummy_address,
                            };

                            console.log(`Adding dummy data for ${JSON.stringify(dummy_data)}...`);

                            migration_msg.chain_contracts.push(dummy_data);
                        }
                    }
                }
            }),
        {},
    );

    program.parse();
};

if (require.main === module) {
    programHandler();
}
