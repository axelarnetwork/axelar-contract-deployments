'use strict';

import { CosmWasmClient } from '@cosmjs/cosmwasm-stargate';
import { StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';

import { addEnvOption } from '../../common/cli-utils';
import { FullConfig } from '../../common/config';
import { addAmplifierOptions, addAmplifierQueryContractOptions } from '../cli-utils';
import { ClientManager, mainProcessor, mainQueryProcessor } from '../processor';
import { getContractInfo } from '../query';
import { checkMigration as checkMigrationCoordinator, instantiatePermissions, migrate as migrateCoordinator } from './coordinator';
import { migrate as migrateMultisig } from './multisig';
import { InstantiatePermission, MigrationCheckOptions, MigrationOptions } from './types';

async function migrate(
    client: ClientManager,
    config: FullConfig,
    options: MigrationOptions,
    args: string[],
    fee: string | StdFee,
): Promise<void> {
    const senderAddress = client.accounts[0].address;
    const contractAddress = options.address ?? config.axelar.contracts[options.contractName]?.address;
    if (args.length === 0 || args[0] === undefined) {
        throw new Error('code_id argument is required');
    }
    const codeId = Number(args[0]);
    if (isNaN(codeId)) {
        throw new Error('code_id must be a valid number');
    }

    const contractInfo = await getContractInfo(client, contractAddress);
    switch (contractInfo.contract) {
        case 'coordinator':
            await migrateCoordinator(client, options, config, senderAddress, contractAddress, contractInfo.version, codeId, fee);
            break;
        case 'multisig':
            await migrateMultisig(client, options, config, senderAddress, contractAddress, contractInfo.version, codeId, fee);
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

async function coordinatorInstantiatePermissions(
    client: ClientManager,
    config: FullConfig,
    options: MigrationOptions,
    args: string[],
    fee: string | StdFee,
): Promise<void> {
    const senderAddress = client.accounts[0].address;
    const contractAddress = options.address ?? config.axelar.contracts['Coordinator']?.address;
    if (args.length < 2 || args[0] === undefined || args[1] === undefined) {
        throw new Error('code_id and current_permissions arguments are required');
    }
    const codeId = Number(args[0]);
    if (isNaN(codeId)) {
        throw new Error('code_id must be a valid number');
    }

    const permissions: InstantiatePermission = JSON.parse(args[1]);
    if (permissions.permission && permissions.permission === 'Everybody') {
        throw new Error(`coordinator is already allowed to instantiate code id ${codeId}`);
    }

    const permitted_addresses = permissions.addresses ?? [];
    if (permitted_addresses.includes(contractAddress)) {
        throw new Error(`coordinator is already allowed to instantiate code id ${codeId}`);
    }

    return instantiatePermissions(client, options, config, senderAddress, contractAddress, permitted_addresses, codeId, fee);
}

const programHandler = () => {
    const program = new Command();

    program.name('migrate').version('1.1.0').description('Automation for migrating Amplifier contracts');

    addAmplifierOptions(
        program
            .command('migrate')
            .argument('<code_id>', 'code id of new contract')
            .addOption(new Option('--ignoreChains [chains]', 'chains to ignore'))
            .addOption(new Option('--address <address>', 'contract address').makeOptionMandatory(true))
            .addOption(new Option('--deposit <deposit>', 'deposit amount').makeOptionMandatory(true))
            .option('--direct', 'make a direct migration rather than a proposal')
            .option('--dry', 'only generate migration msg')
            .description('Migrate contract')
            .action((codeId: string, options: MigrationOptions) => {
                mainProcessor(migrate, options, [codeId]);
            }),
        {},
    );

    addAmplifierQueryContractOptions(
        program
            .command('check')
            .addOption(new Option('--address <address>', 'address of contract to check'))
            .addOption(new Option('--coordinator <coordinator address>', 'coordinator address'))
            .addOption(new Option('--multisig <multisig address>', 'multisig address'))
            .description('Check migration succeeded')
            .action((options: MigrationCheckOptions) => {
                mainQueryProcessor(checkMigration, options, []);
            }),
    );

    addAmplifierOptions(
        program
            .command('coordinator-instantiate-permissions')
            .argument('<code_id>', 'coordinator will have instantiate permissions for this code id')
            .argument('<current_permissions>', 'current instantiate permissions for given contract')
            .addOption(new Option('--address <address>', 'contract address (overrides config)'))
            .option('--dry', 'only generate migration msg')
            .description('Give coordinator instantiate permissions for the given code id')
            .action((codeId: string, currentPermissions: string, options: MigrationOptions) => {
                mainProcessor(coordinatorInstantiatePermissions, options, [codeId, currentPermissions]);
            }),
        {
            proposalOptions: true,
        },
    );

    program.parse();
};

if (require.main === module) {
    programHandler();
}
