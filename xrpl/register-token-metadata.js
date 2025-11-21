'use strict';

const { Command, Option } = require('commander');
const { addAmplifierOptions, addChainNameOption } = require('../cosmwasm/cli-utils');
const { executeTransaction } = require('../cosmwasm/utils');
const { printInfo, printError } = require('../common');
const { getEvent, getEventAttr } = require('./utils');
const { mainProcessor } = require('../cosmwasm/processor');

const CONTRACT_CALLED_EVENT_TYPE = 'wasm-contract_called';
const TOKEN_METADATA_REGISTERED_EVENT_TYPE = 'wasm-token_metadata_registered';

const registerTokenMetadata = async (client, config, options, args, fee) => {
    const { chainName, issuer, currency } = options;


    const xrplGateway = config.axelar.contracts.XrplGateway[chainName];
    if (!xrplGateway) {
        printError(`No XRPLGateway contract found on chain ${chainName}`);
        process.exit(1);
    }

    const execMsg = {
        register_token_metadata: {
            xrpl_token: {
                issued: {
                    issuer,
                    currency,
                },
            },
        },
    };

    const { transactionHash, events } = await executeTransaction(client, xrplGateway.address, execMsg, fee);

    printInfo('Initiated token metadata registration', transactionHash);

    try {
        const contractCalledEvent = getEvent(events, CONTRACT_CALLED_EVENT_TYPE);
        const messageId = getEventAttr(contractCalledEvent, 'message_id');
        const payload = getEventAttr(contractCalledEvent, 'payload');

        const tokenMetadataRegisteredEvent = getEvent(events, TOKEN_METADATA_REGISTERED_EVENT_TYPE);
        const tokenAddress = getEventAttr(tokenMetadataRegisteredEvent, 'token_address');

        printInfo('Message ID', messageId);
        printInfo('Payload', payload);
        printInfo('Token address', tokenAddress);
    } catch (err) {
        printError(err.message);
        process.exit(1);
    }
};

const programHandler = () => {
    const program = new Command();

    program
        .name('register-token-metadata')
        .description("Register a token's metadata.")
        .addOption(new Option('--issuer <issuer>', 'XRPL address of the token issuer').makeOptionMandatory(true))
        .addOption(new Option('--currency <currency>', 'XRPL currency of the token').makeOptionMandatory(true));

    addChainNameOption(program);
    addAmplifierOptions(program, {
        contractOptions: false,
    });

    program.action((options) => {
        mainProcessor(registerTokenMetadata, options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
