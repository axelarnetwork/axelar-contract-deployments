'use strict';

const { Command, Option } = require('commander');
const { addAmplifierQueryOptions } = require('../cosmwasm/cli-utils');
const { printInfo, printWarn, printError } = require('../common');
const { mainQueryProcessor } = require('../cosmwasm/processor');

async function xrplTokenId(client, config, options, args, fee) {
    const { chainName, issuer, currency } = options;
    const { address } = config.getContractConfigByChain('XrplGateway', chainName);
    if (!address) {
        throw new Error(`XrplGateway contract address not found in config for chain ${chainName}`);
    }

    try {
        const result = await client.queryContractSmart(address, {
            xrpl_token_id: {
                issuer,
                currency,
            },
        });

        printInfo(`Token ID for ${currency}.${issuer}`, result);
    } catch (error) {
        printWarn(`Failed to fetch token ID ${currency}.${issuer}`, `${error.message}`);
    }
}

const programHandler = () => {
    const program = new Command();

    program
        .name('xrpl-token-id')
        .description('Query token ID of XRPL token')
        .addOption(new Option('--issuer <issuer>', 'XRPL address of the token issuer').makeOptionMandatory(true))
        .addOption(new Option('--currency <currency>', 'XRPL currency of the token').makeOptionMandatory(true));

    addAmplifierQueryOptions(program);

    program.action((options) => {
        mainQueryProcessor(xrplTokenId, options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
