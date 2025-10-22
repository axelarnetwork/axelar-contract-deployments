'use strict';

import { StdFee } from '@cosmjs/stargate';

import { printError, printInfo } from '../../common';
import { encodeMigrateContractProposal, getCodeId, submitProposal } from '../utils';
import { MigrationOptions } from './types';

// eslint-disable-next-line @typescript-eslint/no-require-imports
export const { SigningCosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

export function extractDefaultProversFromConfig(config) {
    const pairs = {};

    const proverRe = /.*MultisigProver/;

    for (const key in config.axelar.contracts) {
        if (!config.axelar.contracts.hasOwnProperty(key)) {
            continue;
        }

        if (key.match(proverRe)) {
            for (const potentialChain in config.axelar.contracts[key]) {
                const potentialChainObject = config.axelar.contracts[key][potentialChain];

                if (potentialChainObject.hasOwnProperty('address')) {
                    pairs[potentialChain] = potentialChainObject.address;
                }
            }
        }
    }

    return pairs;
}

async function multisigToVersion2_3_1(
    client: typeof SigningCosmWasmClient,
    options: MigrationOptions,
    config,
    senderAddress: string,
    multisigAddress: string,
    fee: string | StdFee,
) {
    const coordinatorAddress = config.axelar.contracts.Coordinator.address;

    const migrationMsg = {
        coordinator: coordinatorAddress,
        default_authorized_provers: extractDefaultProversFromConfig(config),
    };

    const codeId = await getCodeId(client, config, { contractName: 'Multisig', fetchCodeId: true, codeId: options.codeId });

    printInfo(`Migration Msg: ${JSON.stringify(migrationMsg)}`);

    const migrateOptions = {
        contractName: 'Multisig',
        msg: JSON.stringify(migrationMsg),
        title: 'Migrate Multisig v2.3.1',
        description: 'Migrate Multisig v2.3.1',
        runAs: senderAddress,
        codeId: codeId,
        deposit: options.deposit,
        fetchCodeId: false,
        address: multisigAddress,
    };

    const proposal = encodeMigrateContractProposal(config, migrateOptions);

    if (!options.dry) {
        try {
            printInfo(`Executing migration...\n${JSON.stringify(migrateOptions)}`);
            if (options.direct) {
                await client.migrate(senderAddress, multisigAddress, Number(codeId), migrationMsg, fee);
                printInfo('Migration succeeded');
            } else {
                await submitProposal(client, config, migrateOptions, proposal, fee);
                printInfo('Migration proposal successfully submitted');
            }
        } catch (e) {
            printError(`Error: ${e}`);
        }
    }
}

export async function migrate(
    client: typeof SigningCosmWasmClient,
    options: MigrationOptions,
    config,
    senderAddress: string,
    multisigAddress: string,
    version: string,
    fee: string | StdFee,
) {
    switch (version) {
        case '2.1.0':
            return multisigToVersion2_3_1(client, options, config, senderAddress, multisigAddress, fee);
        default:
            printError(`no migration script found for multisig ${version}`);
    }
}
