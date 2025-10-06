'use strict';

import { StdFee } from '@cosmjs/stargate';

import { encodeMigrateContractProposal, submitProposal } from '../utils';
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
    codeId: number,
    fee: string | StdFee,
) {
    const coordinatorAddress = config.axelar.contracts.Coordinator.address;

    const migrationMsg = {
        coordinator: coordinatorAddress,
        default_authorized_provers: extractDefaultProversFromConfig(config),
    };

    console.log('Migration Msg:', migrationMsg);

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
            console.log('Executing migration...', migrateOptions);
            if (options.direct) {
                await client.migrate(senderAddress, coordinatorAddress, Number(codeId), migrationMsg, fee);
                console.log('Migration succeeded');
            } else {
                await submitProposal(client, config, migrateOptions, proposal, fee);
                console.log('Migration proposal successfully submitted');
            }
        } catch (e) {
            console.log('Error:', e);
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
    codeId: number,
    fee: string | StdFee,
) {
    switch (version) {
        case '2.1.0':
            return multisigToVersion2_3_1(client, options, config, senderAddress, multisigAddress, codeId, fee);
        default:
            console.error(`no migration script found for coordinator ${version}`);
    }
}
