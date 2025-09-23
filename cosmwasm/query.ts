import { SigningCosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { DirectSecp256k1HdWallet } from '@cosmjs/proto-signing';
import { Command } from 'commander';

import { FullConfig } from '../common/config';
import { addAmplifierQueryContractOptions } from './cli-utils';
import { Options, mainProcessor } from './processor';
import { ContractInfo } from './types';

export async function getContractInfo(client: SigningCosmWasmClient, contract_address: string): Promise<ContractInfo> {
    const result = await client.queryContractRaw(contract_address, Buffer.from('contract_info'));
    const contract_info: ContractInfo = JSON.parse(Buffer.from(result).toString('ascii'));
    return contract_info;
}

async function contractInfo(client: SigningCosmWasmClient, wallet: DirectSecp256k1HdWallet, config: FullConfig, options: Options) {
    try {
        const address = config?.axelar?.contracts[options.contractName]?.address;
        if (!address) {
            throw new Error(`No address configured for contract '${options.contractName}'`);
        }

        const contract_info: ContractInfo = await getContractInfo(client, address);
        console.log(contract_info);
    } catch (error) {
        console.error(error);
    }
}

const programHandler = () => {
    const program = new Command();

    const info = program
        .command('contract-info')
        .description('Query contract info')
        .action((options: Options) => {
            mainProcessor(contractInfo, options, []);
        });

    addAmplifierQueryContractOptions(info);

    program.parse();
};

if (require.main === module) {
    programHandler();
}
