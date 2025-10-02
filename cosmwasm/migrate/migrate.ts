'use strict';

import { StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';

import { FullConfig } from '../../common/config';
import { addAmplifierOptions, addAmplifierQueryContractOptions } from '../cli-utils';
import { ClientManager, mainProcessor, mainQueryProcessor } from '../processor';
import { getContractInfo } from '../query';
import { migrate as migrateCoordinator, checkMigration as checkMigrationCoordinator} from './coordinator';
import { MigrationCheckOptions, MigrationOptions } from './types';
import { addEnvOption } from '../../common/cli-utils';
import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';

async function migrate(
    client: ClientManager,
    config: FullConfig,
    options: MigrationOptions,
    args: string[],
    _fee: string | StdFee,
): Promise<void> {
    const sender_address = client.accounts[0].address;
    const contract_address = options.address ?? config.axelar.contracts[options.contractName]?.address;
    if (args.length === 0 || args[0] === undefined) {
        throw new Error('code_id argument is required');
    }
    const code_id = Number(args[0]);
    if (isNaN(code_id)) {
        throw new Error('code_id must be a valid number');
    }

    const contract_info = await getContractInfo(client, contract_address);
    switch (contract_info.contract) {
        case 'coordinator':
            await migrateCoordinator(client, options, config, sender_address, contract_address, contract_info.version, code_id);
            break;
    }
}

async function checkMigration(
    client: CosmWasmClient,
    config: FullConfig,
    options: MigrationCheckOptions,
    args: string[],
    _fee: string | StdFee,
): Promise<void> {
    const contract_address = options.address ?? config.axelar.contracts[options.contractName]?.address;

    const contract_info = await getContractInfo(client, contract_address);
    switch (contract_info.contract) {
        case 'coordinator':
            await checkMigrationCoordinator(client, config, contract_info.version, options?.coordinator, options?.multisig);
            break;
    }
}

const programHandler = () => {
    const program = new Command();

    program.name('migrate').version('1.0.0').description('Automation for migrating Amplifier contracts');

    addAmplifierOptions(
        program
            .command('migrate')
            .argument('<code_id>', 'code id of new contract')
            .addOption(new Option('--fees <fees>', 'fees').default('auto'))
            .addOption(new Option('--ignoreChains [chains]', 'chains to ignore').default('auto'))
            .addOption(new Option('--address <address>', 'contract address').makeOptionMandatory(true))
            .addOption(new Option('--deposit <deposit>', 'deposit amount').makeOptionMandatory(true))
            .option('--proposal', 'make a proposal rather than a direct migration')
            .option('--dry', 'only generate migration msg')
            .description('Migrate contract')
            .action((code_id: string, options: MigrationOptions) => {
                mainProcessor(migrate, options, [code_id]);
            }),
        {},
    );

    addAmplifierQueryContractOptions(
        program
            .command('check')
            .addOption(new Option('--address <address>', 'address of contract to check'))
            .addOption(new Option('--coordinator <coordinator address>', 'coordinator address'))
            .addOption(new Option('--multisig <multisig address>', 'multisig address'))
            .description('check migration succeeded')
            .action((options: MigrationCheckOptions) => {
                mainQueryProcessor(checkMigration, options, []);
            }),
    )

    program.parse();
};

if (require.main === module) {
    programHandler();
}
