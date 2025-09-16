'use strict';

const { Command, Option } = require('commander');
const { addAmplifierQueryOptions } = require('../cosmwasm/cli-utils');
const { prepareDummyWallet, prepareClient, initContractConfig } = require('../cosmwasm/utils');
const { loadConfig, printInfo, printWarn, printError } = require('../common');

async function xrplTokenId(client, config, options) {
    const { chainName, issuer, currency } = options;

    const xrplGateway = config.axelar.contracts.XrplGateway[chainName];
    if (!xrplGateway) {
        printError(`No XRPLGateway contract found on chain ${chainName}`);
        process.exit(1);
    }

    try {
        const result = await client.queryContractSmart(xrplGateway.address, {
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
    const config = loadConfig(env);

    initContractConfig(config, options);

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
