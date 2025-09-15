'use strict';

import { Command, Option } from 'commander';

import { loadConfig, saveConfig, printInfo, printError } from '../common';
import { addEnvOption } from '../common/cli-utils';
import { prepareClient, prepareDummyWallet } from './utils';
import { exit } from 'process';

const { CosmWasmClient } = require('@cosmjs/cosmwasm-stargate');

const programHandler = () => {
    const program = new Command();

    program.name('chain-codec').version('1.0.0').description('helpers for the ChainCodec migration of MultisigProver and VotingVerifier');

    addEnvOption(
        program
            .command('prepare')
            .description('Prepare the config for chain-codec instantiation')
            .action(async (options: { env: string }) => {
                const { env } = options;
                const config = loadConfig(env);

                try {
                    const codecMapping: Record<string, 'ChainCodecEvm' | 'ChainCodecSui' | 'ChainCodecStellar'> = {
                        evm: 'ChainCodecEvm',
                        sui: 'ChainCodecSui',
                        stellar: 'ChainCodecStellar',
                    };

                    const encoderMapping: Record<string, 'ChainCodecEvm' | 'ChainCodecSui' | 'ChainCodecStellar'> = {
                        abi: 'ChainCodecEvm',
                        bcs: 'ChainCodecSui',
                        stellar_xdr: 'ChainCodecStellar',
                    };
                    const addressFormatMapping: Record<string, 'ChainCodecEvm' | 'ChainCodecSui' | 'ChainCodecStellar'> = {
                        eip55: 'ChainCodecEvm',
                        sui: 'ChainCodecSui',
                        stellar: 'ChainCodecStellar',
                    };

                    const chains = config?.chains || {};
                    const multisigProverByChain = config?.axelar?.contracts?.MultisigProver || {};
                    const votingVerifierByChain = config?.axelar?.contracts?.VotingVerifier || {};

                    let updates = 0;

                    for (const [chainName, chainConfig] of Object.entries(chains)) {
                        const chainType: string | undefined = (chainConfig as any)?.chainType;
                        const codecContractName = chainType ? codecMapping[chainType] : undefined;
                        if (!codecContractName) {
                            // Unsupported or unspecified chain type; skip quietly
                            continue;
                        }

                        const multisigProver = multisigProverByChain?.[chainName];
                        const votingVerifier = votingVerifierByChain?.[chainName];

                        if (!multisigProver != !votingVerifier) {
                            printError(`Only one of MultisigProver and VotingVerifier found for ${chainName}; either both or none are required`);
                            exit(1);
                        }

                        if (!multisigProver) {
                            // nothing to migrate for this chain
                            continue;
                        }

                        // validate chain type against encoder and address format
                        if (encoderMapping[multisigProver.encoder] !== codecContractName) {
                            printError(`Encoder ${multisigProver.encoder} for ${chainName} does not match chain type ${chainType}`);
                            exit(1);
                        }
                        if (addressFormatMapping[votingVerifier.addressFormat] !== codecContractName) {
                            printError(`Address format ${votingVerifier.addressFormat} for ${chainName} does not match chain type ${chainType}`);
                            exit(1);
                        }

                        const domainSeparator: string | undefined = multisigProver.domainSeparator;
                        if (!domainSeparator) {
                            printError(`Missing domainSeparator in MultisigProver for ${chainName}; skipping codec entry`);
                            exit(1);
                        }

                        // add entry to the appropriate ChainCodec* section
                        if (!config.axelar.contracts[codecContractName]) {
                            config.axelar.contracts[codecContractName] = {};
                        }
                        if (domainSeparator) {
                            const codecSection = config.axelar.contracts[codecContractName];
                            codecSection[chainName] = {
                                ...codecSection[chainName],
                                domainSeparator,
                            };
                        }
                        updates += 1;
                        printInfo(`Prepared ${codecContractName}[${chainName}]`);

                        // clean up MultisigProver fields now handled by ChainCodec
                        if ('domainSeparator' in multisigProver) {
                            delete multisigProver.domainSeparator;
                        }
                        if ('encoder' in multisigProver) {
                            delete multisigProver.encoder;
                        }

                        // clean up VotingVerifier addressFormat (now handled by ChainCodec)
                        if ('addressFormat' in votingVerifier) {
                            delete votingVerifier.addressFormat;
                        }
                    }

                    saveConfig(config, env);
                    printInfo(`Chain codec preparation complete. Updated entries: ${updates}`);
                } catch (error) {
                    console.error(error);
                }
        })
    );

    program.parse();
};

if (require.main === module) {
    programHandler();
}
