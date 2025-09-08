'use strict';

import { Command, Option } from 'commander';

import { loadConfig } from '../../common';
import { addAmplifierOptions } from '../cli-utils';
import { ContractInfo, getContractInfo } from '../contract';
import { prepareClient, prepareWallet } from '../utils';
import { migrate as migrateCoordinator } from './coordinator';

const programHandler = () => {
    const program = new Command();

    program.name('migrate').version('1.0.0').description('Automation for migrating Amplifier contracts');

    addAmplifierOptions(
        program
            .command('migrate')
            .argument('<code_id>', 'code id of new contract')
            .addOption(new Option('--fees <fees>', 'fees').default('auto'))
            .addOption(new Option('--address <address>', 'contract address').makeOptionMandatory(true))
            .addOption(new Option('--deposit <deposit>', 'deposit amount').makeOptionMandatory(true))
            .option('--dry', 'only generate migration msg')
            .description('Migrate contract')
            .action(async (code_id: string, options: { env: string; mnemonic: string; address: string; deposit: string, fees; dry?; dummy? }) => {
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

                const contract_info: ContractInfo = await getContractInfo(client, options.address);
                switch (contract_info.contract) {
                    case 'coordinator':
                        migrateCoordinator(
                            client,
                            wallet,
                            options,
                            config,
                            sender_address,
                            options.address,
                            contract_info.version,
                            Number(code_id),
                        );
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
