'use strict';

import { Command, Option } from 'commander';

import { loadConfig } from '../common';
import { addAmplifierQueryOptions } from './cli-utils';
import { prepareClient, prepareDummyWallet } from './utils';

// cosmwasm-stargate imports protobufjs which does not have a default export
// Consequently, import CosmWasmClient using CommonJS to avoid error TS1192
// eslint-disable-next-line @typescript-eslint/no-require-imports
const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

export interface ContractInfo {
    contract: string;
    version: string;
}

export enum Contract {
    ServiceRegistry,
    Router,
    Multisig,
    Coordinator,
    Rewards,
    AxelarnetGateway,
    InterchainTokenService,
}

export const ContractMap = new Map<Contract, string>([
    // [Contract.ServiceRegistry, 'ServiceRegistry'],
    [Contract.Router, 'router'],
    [Contract.Multisig, 'multisig'],
    [Contract.Coordinator, 'coordinator'],
    // [Contract.Rewards, 'Rewards'],
    // [Contract.AxelarnetGateway, 'AxelarnetGateway'],
    // [Contract.InterchainTokenService, 'InterchainTokenService'],
]);

export async function getContractInfo(client: typeof CosmWasmClient, contract_address: string): Promise<ContractInfo> {
    try {
        const result = await client.queryContractRaw(contract_address, Buffer.from('contract_info'));
        const contract_info: ContractInfo = JSON.parse(Buffer.from(result).toString('ascii'));
        return contract_info;
    } catch (error) {
        throw error;
    }
}

const programHandler = () => {
    const program = new Command();

    program.name('contract').version('1.0.0').description('Query contract info');

    addAmplifierQueryOptions(
        program
            .command('info')
            .description('Query contract info')
            .addOption(new Option('--contract <contract>', 'amplifier contract').choices(Array.from(ContractMap.values())))
            .option('--address <address>', 'contract address')
            .action(async (options: { env: string; contract?: string; address?: string }) => {
                const { env } = options;
                const config = loadConfig(env);

                const wallet = await prepareDummyWallet();
                const client = await prepareClient(config, wallet);

                if (options.contract && options.address) {
                    console.log('cannot specify both --contract and --address');
                    return;
                } else if (!options.contract && !options.address) {
                    console.log('must specify either --contract or --address');
                    return;
                }

                let address: string;
                if (options.contract) {
                    address = config.axelar.contracts[options.contract].address;
                } else {
                    address = options.address;
                }

                try {
                    const contract_info: ContractInfo = await getContractInfo(client, address);
                    console.log(contract_info);
                } catch (error) {
                    console.error(error);
                }
            }),
    );

    program.parse();
};

if (require.main === module) {
    programHandler();
}
