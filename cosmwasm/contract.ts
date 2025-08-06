'use strict';

import { Command } from 'commander';

import { loadConfig } from '../common';
import { addAmplifierQueryOptions } from './cli-utils';
import { prepareClient, prepareDummyWallet } from './utils';

// cosmwasm-stargate imports protobufjs which does not have a default export
// Consequently, import CosmWasmClient using CommonJS to avoid error TS1192
// eslint-disable-next-line @typescript-eslint/no-require-imports
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

interface ContractInfo {
    contract: string;
    version: string;
}

function get_contract_info(client: typeof CosmWasmClient, contract_address: string): Promise<ContractInfo> {
    return new Promise(async (resolve, _) => {
        const result = await client.queryContractRaw(contract_address, Buffer.from('contract_info'));
        const contract_info: ContractInfo = JSON.parse(Buffer.from(result).toString('ascii'));
        resolve(contract_info);
    });
}

const programHandler = () => {
    const program = new Command();

    program.name('contract').version('1.0.0').description('Query contract info');

    addAmplifierQueryOptions(
        program
            .command('info')
            .argument('<contract_address>', 'The contract address')
            .description('Query contract info')
            .action(async (contract_address: string, options: { env: string }) => {
                const { env } = options;
                const config = loadConfig(env);

                const wallet = await prepareDummyWallet();
                const client = await prepareClient(config, wallet);

                console.log(await get_contract_info(client, contract_address));
            }),
    );

    program.parse();
};

if (require.main === module) {
    programHandler();
}
