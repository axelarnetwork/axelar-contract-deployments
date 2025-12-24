'use strict';

import { CosmWasmClient, JsonObject } from '@cosmjs/cosmwasm-stargate';
import { StdFee } from '@cosmjs/stargate';
import { Command } from 'commander';

import { printInfo, printWarn } from '../../common';
import { addEnvOption } from '../../common/cli-utils';
import { ConfigManager } from '../../common/config';
import { isConsensusChain } from '../../evm/utils';
import { addAmplifierOptions } from '../cli-utils';
import { ClientManager, Options, mainProcessor, mainQueryProcessor } from '../processor';
import { confirmProposalSubmission } from '../submit-proposal';
import { encodeMigrate, getCodeId, submitProposal } from '../utils';
import { MigrationOptions } from './types';

const programHandler = () => {
    const program = new Command();

    program.name('chain-codec').version('1.0.0').description('helpers for the ChainCodec migration of MultisigProver and VotingVerifier');

    addEnvOption(
        program
            .command('prepare')
            .description('Prepare the config for chain-codec instantiation and migration of MultisigProver and VotingVerifier')
            .action((options) => mainQueryProcessor(prepare, options)),
    );

    addAmplifierOptions(
        program
            .command('migrate-mp-vv')
            .option('--direct', 'make a direct migration rather than a proposal')
            .description('Submit a proposal migrate the MultisigProver and VotingVerifier contracts')
            .action((options) => mainProcessor(migrate, options)),
        {
            proposalOptions: true,
        },
    );

    program.parse();
};

const CODEC_MAPPING: Record<string, 'ChainCodecEvm' | 'ChainCodecSui' | 'ChainCodecStellar' | 'ChainCodecSolana'> = {
    evm: 'ChainCodecEvm',
    sui: 'ChainCodecSui',
    stellar: 'ChainCodecStellar',
    svm: 'ChainCodecSolana',
};

async function prepare(_client: CosmWasmClient, config: ConfigManager, _: Options) {
    try {
        for (const [chainName, chainConfig] of Object.entries(config.chains)) {
            const chainType = chainConfig.chainType;
            if (!chainType) {
                // Unsupported or unspecified chain type
                printWarn(`Missing chain type for chain ${chainName}; skipping ChainCodec entry`);
                continue;
            }

            const codecContractName = CODEC_MAPPING[chainType];
            if (!codecContractName) {
                // Unsupported or unspecified chain type
                printInfo(`Unsupported chain type: ${chainType}; skipping ChainCodec entry`);
                continue;
            }

            // add ChainCodec for each chain type
            config.axelar.contracts[codecContractName] = config.axelar.contracts[codecContractName] || {};

            // remove addressFormat and encoder from MultisigProver and VotingVerifier config
            const votingVerifier: { addressFormat?: string } = config.axelar.contracts.VotingVerifier[chainName];
            const multisigProver: { encoder?: string } = config.axelar.contracts.MultisigProver[chainName];

            if (votingVerifier) {
                delete votingVerifier.addressFormat;
            } else {
                printInfo(`Missing VotingVerifier config for chain ${chainName}`);
            }

            if (multisigProver) {
                delete multisigProver.encoder;
            } else {
                printInfo(`Missing MultisigProver config for chain ${chainName}`);
            }
        }

        printInfo(`Chain codec preparation complete`);
    } catch (error) {
        console.error(error);
    }
}

async function migrate(client: ClientManager, config: ConfigManager, options: MigrationOptions, _args: string[], fee: 'auto' | StdFee) {
    try {
        const migrations: {
            proverAddress: string;
            proverCodeId: number;
            proverMsg: JsonObject;

            verifierAddress: string;
            verifierCodeId: number;
            verifierMsg: JsonObject;
        }[] = [];
        for (const [chainName, chainConfig] of Object.entries(config.chains).filter(([_, chainConfig]) => !isConsensusChain(chainConfig))) {
            let codecAddress: string;
            try {
                codecAddress = config.getChainCodecAddress(chainConfig.chainType);
            } catch (error) {
                printWarn(
                    `Missing ChainCodec address for chain ${chainName} with chain type ${chainConfig.chainType}; skipping migration for this chain`,
                );
                continue;
            }

            // migration data for MultisigProver contract
            const multisigProver = config.getMultisigProverContract(chainName);
            const multisigProverAddress = config.validateRequired(multisigProver.address, `MultisigProver[${chainName}].address`);
            const multisigProverContractName = config.getMultisigProverContractForChainType(chainConfig.chainType);
            const proverCodeId = await getCodeId(client, config, { contractName: multisigProverContractName, fetchCodeId: true });
            multisigProver.codeId = proverCodeId;

            // migration data for VotingVerifier contract
            const votingVerifier = config.getVotingVerifierContract(chainName);
            const votingVerifierAddress = config.validateRequired(votingVerifier.address, `VotingVerifier[${chainName}].address`);
            const votingVerifierContractName = config.getVotingVerifierContractForChainType(chainConfig.chainType);
            const verifierCodeId = await getCodeId(client, config, { contractName: votingVerifierContractName, fetchCodeId: true });
            votingVerifier.codeId = verifierCodeId;

            migrations.push({
                proverAddress: multisigProverAddress,
                proverCodeId: proverCodeId,
                proverMsg: {
                    chain_codec_address: codecAddress,
                },
                verifierAddress: votingVerifierAddress,
                verifierCodeId: verifierCodeId,
                verifierMsg: {
                    chain_codec_address: codecAddress,
                },
            });
        }

        if (options.direct) {
            const [account] = client.accounts;

            for (const migration of migrations) {
                await client.migrate(account.address, migration.proverAddress, migration.proverCodeId, migration.proverMsg, fee);
                printInfo('Migrated prover contract', migration.proverAddress);
                await client.migrate(account.address, migration.verifierAddress, migration.verifierCodeId, migration.verifierMsg, fee);
                printInfo('Migrated verifier contract', migration.verifierAddress);
            }
        } else {
            // Create one proposal with multiple migration messages
            const migrateMsgs = migrations.flatMap((migration) => {
                const migrateProverMsg = encodeMigrate(config, {
                    ...options,
                    address: migration.proverAddress,
                    codeId: migration.proverCodeId,
                    msg: JSON.stringify(migration.proverMsg),
                });
                const migrateVerifierMsg = encodeMigrate(config, {
                    ...options,
                    address: migration.verifierAddress,
                    codeId: migration.verifierCodeId,
                    msg: JSON.stringify(migration.verifierMsg),
                });
                return [migrateProverMsg, migrateVerifierMsg];
            });

            if (!confirmProposalSubmission(options, migrateMsgs)) {
                return;
            }

            const proposalId = await submitProposal(client, config, options, migrateMsgs, fee);
            printInfo('Migration proposal successfully submitted', proposalId);
        }
    } catch (error) {
        console.error(error);
    }
}

if (require.main === module) {
    programHandler();
}
