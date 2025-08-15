'use strict';

import { Command, Option } from 'commander';

import { loadConfig } from '../common';
import { addAmplifierOptions } from './cli-utils';
import { prepareClient, prepareWallet } from './utils';
// import { SigningCosmWasmClient, ExecuteResult } from '@cosmjs/cosmwasm-stargate';

// cosmwasm-stargate imports protobufjs which does not have a default export
// Consequently, import CosmWasmClient using CommonJS to avoid error TS1192
// eslint-disable-next-line @typescript-eslint/no-require-imports
const { SigningCosmWasmClient, ExecuteResult} = require('@cosmjs/cosmwasm-stargate');

interface RegisterProverAddressMsg {
    chain_name: string,
    new_prover_addr: string,
}

interface RegisterProverAddress {
    register_prover_contract: RegisterProverAddressMsg,
}

function mock_register_prover_addresses(client: typeof SigningCosmWasmClient, sender_address: string, contract_address: string, msgs: RegisterProverAddressMsg[]): Promise<typeof ExecuteResult[]> {
    return new Promise(async (resolve, reject) => {
        let results: typeof ExecuteResult[] = []
        for (let i = 0; i < msgs.length; i++) {
            if (!msgs[i].chain_name || !msgs[i].new_prover_addr) {
                reject(new Error("invalid register prover address messages"))
                return
            }

            let execute_msg: RegisterProverAddress = {
                register_prover_contract: msgs[i],
            }

            try {
                results.push(await client.execute(sender_address, contract_address, execute_msg, "auto"))
                console.log("Added prover address: ", execute_msg)
            } catch(e) {
                reject(new Error(`execute error: ${e.message}`))
                return
            }
        }

        resolve(results)
    })
}

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
                // TODO: Hack to test that this script works. Properly ommit axelar in migration code
                if (chain_endpoints[i].name != "axelar") {
                    chain_contracts.push({
                        chain_name: chain_endpoints[i].name,
                        gateway_address: chain_endpoints[i].gateway.address,
                        verifier_address: config.verifier,
                    })
                }
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
            .command('mock-register-prover-address')
            .argument('<contract_address>', 'Contract address')
            .argument('<chain_prover_map>', 'Map from chain name to prover address')
            .description('Query contract info')
            .action(async (chain_prover_map: string, options: { env: string, mnemonic: string }) => {
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
                let coordinator_address = config.axelar.contracts.Coordinator.address
                let msgs: RegisterProverAddressMsg[] = JSON.parse(chain_prover_map)

                try {
                    console.log(await mock_register_prover_addresses(client, sender_address, coordinator_address, msgs))
                } catch (e) {
                    console.log(e.message)
                }
            }),
        {}
    );

    addAmplifierOptions(
        program
            .command('migrate-coordinator')
            .argument('<code_id>', "code id of new contract")
            .addOption(new Option('--fees <fees>', 'fees').default("auto"))
            .option('--generate', 'only generate migration msg')
            .option('--coordinator <address>', 'coordinator address')
            .description('Migrate coordinator')
            .action(async (code_id: string, options: { env: string, mnemonic: string, fees: any, generate?: any, coordinator?: string}) => {
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

                if (!options.generate) {
                    client.migrate(sender_address, coordinator_address, Number(code_id), migration_msg, options.fees)
                }
            }),
        {}
    );

    program.parse();
};

if (require.main === module) {
    programHandler();
}
