'use strict';

import { Command } from 'commander';

import { loadConfig, saveConfig, printInfo, printWarn } from '../common';
import { addEnvOption } from '../common/cli-utils';

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

                    const chains = config?.chains || {};

                    const chainTypes = Object.values(chains).map(chainConfig => {
                        return (chainConfig as { chainType: string })?.chainType;
                    });

                    for (const [chainName, chainConfig] of Object.entries(chains)) {
                        const chainType = (chainConfig as { chainType: string })?.chainType;
                        if (!chainType) {
                            // Unsupported or unspecified chain type
                            printWarn(`Missing chain type for chain ${chainName}; skipping ChainCodec entry`);
                            continue;
                        }

                        const codecContractName = codecMapping[chainType];
                        if (!codecContractName) {
                            // Unsupported or unspecified chain type
                            printInfo(`Unsupported chain type: ${chainType}; skipping ChainCodec entry`);
                            continue;
                        }

                        // add ChainCodec for each chain type
                        config.axelar.contracts[codecContractName] = config.axelar.contracts[codecContractName] || {};

                        // remove addressFormat and encoder from MultisigProver and VotingVerifier config
                        const votingVerifier = config.axelar.contracts.VotingVerifier[chainName];
                        const multisigProver = config.axelar.contracts.MultisigProver[chainName];

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

                    saveConfig(config, env);
                    printInfo(`Chain codec preparation complete`);
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
