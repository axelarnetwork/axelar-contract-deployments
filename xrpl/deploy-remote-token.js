'use strict';

const { Command, Option } = require('commander');
const { addAmplifierOptions, addChainNameOption } = require('../cosmwasm/cli-utils');
const { executeTransaction: executeCosmosTransaction } = require('../cosmwasm/utils');
const { printInfo, printError } = require('../common');
const { mainCosmosProcessor, getEvent, getEventAttr } = require('./utils');

const CONTRACT_CALLED_EVENT_TYPE = 'wasm-contract_called';

const deployRemoteToken = async (config, options, wallet, client, fee) => {
    const { chainName, issuer, currency, tokenName, tokenSymbol, destinationChain } = options;
    const [account] = await wallet.getAccounts();

    const xrplGateway = config.axelar.contracts.XrplGateway[chainName];
    if (!xrplGateway) {
        printError(`No XRPLGateway contract found on chain ${chainName}`);
        process.exit(1);
    }

    const execMsg = {
        deploy_remote_token: {
            xrpl_token: {
                issued: {
                    issuer,
                    currency,
                },
            },
            destination_chain: destinationChain,
            token_metadata: {
                name: tokenName,
                symbol: tokenSymbol,
            },
        },
    };

    const { transactionHash, events } = await executeCosmosTransaction(client, account, xrplGateway.address, execMsg, fee);

    printInfo('Initiated remote token deployment', transactionHash);

    try {
        const contractCalledEvent = getEvent(events, CONTRACT_CALLED_EVENT_TYPE);
        const messageId = getEventAttr(contractCalledEvent, 'message_id');
        const payload = getEventAttr(contractCalledEvent, 'payload');
        printInfo('Message ID', messageId);
        printInfo('Payload', payload);
    } catch (err) {
        printError(err.message);
        process.exit(1);
    }
};

const programHandler = () => {
    const program = new Command();

    program
        .name('deploy-remote-token')
        .description('Deploy XRPL IOU token on some remote chain.')
        .addOption(new Option('--issuer <issuer>', 'XRPL address of token issuer').makeOptionMandatory(true))
        .addOption(new Option('--currency <currency>', 'XRPL currency of token').makeOptionMandatory(true))
        .addOption(new Option('--destinationChain <destinationChain>', 'Chain to deploy the interchain token on').makeOptionMandatory(true))
        .addOption(new Option('--tokenName <tokenName>', 'Name of the new interchain token').makeOptionMandatory(true))
        .addOption(new Option('--tokenSymbol <tokenSymbol>', 'Symbol of the new interchain token').makeOptionMandatory(true));

    addChainNameOption(program);
    addAmplifierOptions(program, {
        contractOptions: false,
    });

    program.action((options) => {
        mainCosmosProcessor(deployRemoteToken, options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
