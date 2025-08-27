'use strict';

import { Command, Option } from 'commander';

import { loadConfig } from '../../common';
import { addAmplifierOptions } from '../cli-utils';
import { prepareClient, prepareWallet } from '../utils';
import { Contract, ContractMap, ContractInfo, get_contract_info} from '../contract';
import { migrate as migrate_coordinator } from './coordinator';

const programHandler = () => {
    const program = new Command();

    program.name('auto-migrate').version('1.0.0').description('Automation for migrating Amplifier contracts');

    addAmplifierOptions(
        program
            .command('migrate')
            .argument('<code_id>', 'code id of new contract')
            .addOption(new Option('--fees <fees>', 'fees').default('auto'))
            .addOption(new Option('--address <address>', 'coordinator address').makeOptionMandatory(true))
            .option('--dry', 'only generate migration msg')
            .option('--dummy', 'allow dummy data to be used when gateway or verifier is not present')
            .description('Migrate coordinator')
            .action(async (code_id: string, options: { env: string; mnemonic: string; address: string; fees; dry?; dummy?;}) => {
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

                let contract_info: ContractInfo = await get_contract_info(client, options.address);
                switch (contract_info.contract) {
                    case ContractMap[Contract.Coordinator]:
                        migrate_coordinator(client, options, config, sender_address, options.address, contract_info.version, Number(code_id));
                    break;
                }

                return;
            }),
        {},
    );

    program.parse();
};

if (require.main === module) {
    programHandler();
}
