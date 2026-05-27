'use strict';

const { Command, Option } = require('commander');
const { addAmplifierOptions, addChainNameOption } = require('../cosmwasm/cli-utils');
const { executeTransaction } = require('../cosmwasm/utils');
const { printInfo, printError } = require('../common');
const { getEvent, getEventAttr } = require('./utils');
const { mainProcessor } = require('../cosmwasm/processor');

const CONTRACT_CALLED_EVENT_TYPE = 'wasm-contract_called';
const LINK_TOKEN_STARTED_EVENT_TYPE = 'wasm-link_token_started';

function strip0x(value) {
    return value?.startsWith('0x') ? value.slice(2) : value;
}

function normalizeHex(value, name) {
    const hex = strip0x(value);

    if (!hex || hex.length % 2 !== 0 || !/^[0-9a-fA-F]+$/.test(hex)) {
        throw new Error(`${name} must be a non-empty hex string`);
    }

    return hex;
}

function normalizeOptionalBytes(value, name) {
    if (!value || value === '0x') {
        return null;
    }

    return normalizeHex(value, name);
}

const linkToken = async (client, config, options, _args, fee) => {
    const { chainName, tokenId, destinationChain, destinationTokenAddress, tokenManagerType, params, operator } = options;

    if (params && operator) {
        throw new Error('Use either --params or --operator, not both');
    }

    const xrplGateway = config.axelar.contracts.XrplGateway[chainName];
    if (!xrplGateway) {
        printError(`No XRPLGateway contract found on chain ${chainName}`);
        process.exit(1);
    }

    const linkParams = operator ? normalizeHex(operator, 'operator') : normalizeOptionalBytes(params, 'params');

    const execMsg = {
        link_token: {
            token_id: normalizeHex(tokenId, 'tokenId'),
            destination_chain: destinationChain,
            link_token: {
                token_manager_type: tokenManagerType,
                destination_token_address: normalizeHex(destinationTokenAddress, 'destinationTokenAddress'),
                params: linkParams,
            },
        },
    };

    const { transactionHash, events } = await executeTransaction(client, xrplGateway.address, execMsg, fee);

    printInfo('Initiated token link', transactionHash);

    try {
        const contractCalledEvent = getEvent(events, CONTRACT_CALLED_EVENT_TYPE);
        const messageId = getEventAttr(contractCalledEvent, 'message_id');
        const payload = getEventAttr(contractCalledEvent, 'payload');

        printInfo('Message ID', messageId);
        printInfo('Payload', payload);

        const linkTokenStartedEvent = getEvent(events, LINK_TOKEN_STARTED_EVENT_TYPE);
        printInfo('Token ID', getEventAttr(linkTokenStartedEvent, 'token_id'));
        printInfo('Destination chain', getEventAttr(linkTokenStartedEvent, 'destination_chain'));
        printInfo('Destination token address', getEventAttr(linkTokenStartedEvent, 'destination_token_address'));
    } catch (err) {
        printError(err.message);
        process.exit(1);
    }
};

const programHandler = () => {
    const program = new Command();

    program
        .name('link-token')
        .description('Link an XRPL-origin token ID to an existing token on a remote destination chain.')
        .addOption(new Option('--tokenId <tokenId>', 'XRPL-origin token ID').makeOptionMandatory(true))
        .addOption(new Option('--destinationChain <destinationChain>', 'Destination chain to link to').makeOptionMandatory(true))
        .addOption(
            new Option('--destinationTokenAddress <destinationTokenAddress>', 'Token address on the destination chain').makeOptionMandatory(
                true,
            ),
        )
        .addOption(
            new Option(
                '--tokenManagerType <tokenManagerType>',
                'Token manager type to deploy on the destination chain',
            ).makeOptionMandatory(true),
        )
        .addOption(new Option('--params <params>', 'Raw token manager params bytes. Empty by default.'))
        .addOption(new Option('--operator <operator>', 'EVM operator address encoded as link params. Mutually exclusive with --params.'));

    addChainNameOption(program);
    addAmplifierOptions(program, {
        contractOptions: false,
    });

    program.action((options) => {
        mainProcessor(linkToken, options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
