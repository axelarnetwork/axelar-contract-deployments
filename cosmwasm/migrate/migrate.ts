'use strict';

import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';

import { FullConfig } from '../../common/config';
import { addAmplifierOptions, addAmplifierQueryContractOptions } from '../cli-utils';
import { ClientManager, mainProcessor, mainQueryProcessor } from '../processor';
import { getContractInfo } from '../query';
import { checkMigration as checkMigrationCoordinator, migrate as migrateCoordinator } from './coordinator';
import { migrate as migrateMultisig } from './multisig';
import { MigrationCheckOptions, MigrationOptions } from './types';

async function migrate(
    client: ClientManager,
    config: FullConfig,
    options: MigrationOptions,
    _args: string[],
    fee: string | StdFee,
): Promise<void> {
    const senderAddress = client.accounts[0].address;
    const contractAddress = options.address ?? config.axelar.contracts[options.contractName]?.address;

    const contractInfo = await getContractInfo(client, contractAddress);
    switch (contractInfo.contract) {
        case 'coordinator':
            await migrateCoordinator(client, options, config, senderAddress, contractAddress, contractInfo.version, fee);
            break;
        case 'multisig':
            await migrateMultisig(client, options, config, senderAddress, contractAddress, contractInfo.version, fee);
            break;
    }
}

async function checkMigration(
    client: CosmWasmClient,
    config: FullConfig,
    options: MigrationCheckOptions,
    _args: string[],
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
            .addOption(new Option('--ignoreChains [chains]', 'chains to ignore'))
            .addOption(new Option('--address <address>', 'contract address').makeOptionMandatory(true))
            .addOption(new Option('--codeId <codeId>', 'code id of new contract (default fetch from the chain)'))
            .option('--direct', 'make a direct migration rather than a proposal')
            .option('--dry', 'only generate migration msg')
            .description('Migrate contract')
            .action((options: MigrationOptions) => {
                if (options.codeId !== undefined) {
                    const parsedCodeId = Number(options.codeId);

                    if (isNaN(parsedCodeId) || !Number.isInteger(parsedCodeId) || parsedCodeId <= 0) {
                        console.error(`Error: Invalid codeId '${options.codeId}'. CodeId must be a positive integer.`);
                        process.exit(1);
                    }

                    options.codeId = parsedCodeId;
                }
                mainProcessor(migrate, options, []);
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
    );

    program.parse();
};

if (require.main === module) {
    programHandler();
}
