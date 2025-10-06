'use strict';

import { StdFee } from '@cosmjs/stargate';
import { Command, Option } from 'commander';

import { FullConfig } from '../../common/config';
import { addAmplifierOptions } from '../cli-utils';
import { ClientManager, mainProcessor } from '../processor';
import { getContractInfo } from '../query';
import { migrate as migrateCoordinator } from './coordinator';
import { migrate as migrateMultisig } from './multisig';
import { MigrationOptions } from './types';

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

const programHandler = () => {
    const program = new Command();

    program.name('migrate').version('1.0.0').description('Automation for migrating Amplifier contracts');

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

    program.parse();
};

if (require.main === module) {
    programHandler();
}
