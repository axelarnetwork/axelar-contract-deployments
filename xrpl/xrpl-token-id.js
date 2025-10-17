'use strict';

const { Command, Option } = require('commander');
const { addAmplifierQueryOptions } = require('../cosmwasm/cli-utils');
const { prepareDummyWallet, prepareClient } = require('../cosmwasm/utils');
const { printInfo, printWarn, printError, ConfigManager } = require('../common');

async function xrplTokenId(client, config, options) {
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

const mainProcessor = async (processor, options) => {
    const { env } = options;
    const config = new ConfigManager(env);

    const wallet = await prepareDummyWallet(options);
    const client = await prepareClient(config, wallet);

    await processor(client, config, options);
};

const programHandler = () => {
    const program = new Command();

    program
        .name('xrpl-token-id')
        .description('Query token ID of XRPL token')
        .addOption(new Option('--issuer <issuer>', 'XRPL address of the token issuer').makeOptionMandatory(true))
        .addOption(new Option('--currency <currency>', 'XRPL currency of the token').makeOptionMandatory(true));

    addAmplifierQueryOptions(program);

    program.action((options) => {
        mainProcessor(xrplTokenId, options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
