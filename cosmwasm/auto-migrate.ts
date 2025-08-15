'use strict';

import { Command, Option } from 'commander';

import { loadConfig } from '../common';
import { addAmplifierOptions } from './cli-utils';
import { prepareClient, prepareWallet } from './utils';

// cosmwasm-stargate imports protobufjs which does not have a default export
// Therefore, import SigningCosmWasmClient using CommonJS to avoid error TS1192
// eslint-disable-next-line @typescript-eslint/no-require-imports
const { SigningCosmWasmClient} = require('@cosmjs/cosmwasm-stargate');

interface ChainEndpoint {
    name: string,
    gateway: {
        address: string,
    },
}

interface ChainContracts {
    chain_name: string,
    gateway_address: string,
    verifier_address: string,
}

async function query_chains_from_router(client: typeof SigningCosmWasmClient, router_address: string): Promise<ChainEndpoint[]> {
    return new Promise(async (resolve, reject) => {
        try {
            let res: ChainEndpoint[] = await client.queryContractSmart(router_address, {"chains": {}})
            resolve(res)
        } catch (e) {
            reject(e)
        }
    })
}

async function construct_chain_contracts(client: typeof SigningCosmWasmClient, chain_endpoints: ChainEndpoint[]): Promise<ChainContracts[]> {
    return new Promise(async (resolve, reject) => {
        interface GatewayConfig {
            verifier: string,
        }

        try {
            let chain_contracts: ChainContracts[] = []

            for (let i = 0; i < chain_endpoints.length; i++) {
                let res = await client.queryContractRaw(chain_endpoints[i].gateway.address, Buffer.from('config'))
                const config: GatewayConfig = JSON.parse(Buffer.from(res).toString('ascii'));
                chain_contracts.push({
                    chain_name: chain_endpoints[i].name ?? "_",
                    gateway_address: chain_endpoints[i].gateway.address  ?? "_",
                    verifier_address: config.verifier ?? "_",
                })
            }
            
            resolve(chain_contracts)
        } catch (e) {
            reject(e)
        }
    })
}

const programHandler = () => {
    const program = new Command();

    program.name('auto-migrate').version('1.0.0').description('Automation for migrating Amplifier contracts');

    addAmplifierOptions(
        program
            .command('migrate-coordinator')
            .argument('<code_id>', "code id of new contract")
            .addOption(new Option('--fees <fees>', 'fees').default("auto"))
            .option('--dry', 'only generate migration msg')
            .option('--coordinator <address>', 'coordinator address')
            .description('Migrate coordinator')
            .action(async (code_id: string, options: { env: string, mnemonic: string, fees: any, dry?: any, coordinator?: string}) => {
                const { env } = options;
                const config = loadConfig(env);

                const wallet = await prepareWallet(options);
                const client = await prepareClient(config, wallet);
                let accounts = await wallet.getAccounts()
                if (accounts.length < 1) {
                    console.log("invalid mnemonic")
                    return
                }

                let sender_address = accounts[0].address
                let router_address = config.axelar.contracts.Router.address
                let multisig_address = config.axelar.contracts.Multisig.address
                let coordinator_address = options.coordinator ?? config.axelar.contracts.Coordinator.address

                let chain_endpoints = await query_chains_from_router(client, router_address)
                let chain_contracts = await construct_chain_contracts(client, chain_endpoints)

                const migration_msg = {
                    router: router_address,
                    multisig: multisig_address,
                    chain_contracts: chain_contracts,
                }
                
                console.log("Migration Msg:", migration_msg)

                if (!options.dry) {
                    try {
                        console.log("Executing migration...")
                        let res = await client.migrate(sender_address, coordinator_address, Number(code_id), migration_msg, options.fees)
                        console.log("Migration succeeded")
                    } catch (e) {
                        console.log("Migration failed:", e)
                    }
                }
            }),
        {}
    );

    program.parse();
};

if (require.main === module) {
    programHandler();
}
