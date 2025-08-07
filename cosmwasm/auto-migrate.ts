'use strict';

import { Command } from 'commander';

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
            } catch(e) {
                reject(new Error(`execute error: ${e.message}`))
                return
            }
        }

        resolve(results)
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
            .action(async (contract_address: string, chain_prover_map: string, options: { env: string, mnemonic: string }) => {
                const { env } = options;
                const config = loadConfig(env);

                const wallet = await prepareWallet(options);
                const client = await prepareClient(config, wallet);
                let accounts = await wallet.getAccounts()
                if (accounts.length < 1) {
                    console.log("invalid mnemonic")
                    return
                }

                let address = accounts[0].address;
                let msgs: RegisterProverAddressMsg[] = JSON.parse(chain_prover_map)

                try {
                    console.log(await mock_register_prover_addresses(client, address, contract_address, msgs))
                } catch (e) {
                    console.log(e.message)
                }
            }),
        {}
    );

    program.parse();
};

if (require.main === module) {
    programHandler();
}
