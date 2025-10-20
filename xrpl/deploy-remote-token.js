'use strict';

const { Command, Option } = require('commander');
const { addAmplifierOptions, addChainNameOption } = require('../cosmwasm/cli-utils');
const { executeTransaction } = require('../cosmwasm/utils');
const { printInfo, printError, validateParameters } = require('../common');
const { mainProcessor } = require('../cosmwasm/processor');
const { Err } = require('@stellar/stellar-sdk/contract');

const CONTRACT_CALLED_EVENT_TYPE = 'wasm-contract_called';
const RESERVE_CURRENCY = 'XRP'

const deployRemoteToken = async (client, config, options, args, fee) => {
    // TODO: Add validation or retrieve token informaiton from on-chain
    const { chainName, issuer, currency, tokenName, tokenSymbol, destinationChain } = options;

    const xrplGateway = config.axelar.contracts.XrplGateway[chainName];
    if (!xrplGateway) {
        printError(`No XRPLGateway contract found on chain ${chainName}`);
        process.exit(1);
    }

    // For XRP, use the Xrp variant instead of issued
    const isXrp = currency === RESERVE_CURRENCY;
    
    const execMsg = {
        deploy_remote_token: {
            xrpl_token: isXrp ? 'xrp' : {
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

    const { transactionHash, events } = await executeTransaction(client, xrplGateway.address, execMsg, fee);

    printInfo('Initiated remote token deployment', transactionHash);

    const contractCalledEvent = events.find(e => e.type === CONTRACT_CALLED_EVENT_TYPE);
    if (!contractCalledEvent) {
        throw new Error(`${CONTRACT_CALLED_EVENT_TYPE} event not found`);
    }

    const messageId = contractCalledEvent.attributes.find(attr => attr.key === 'message_id')?.value;
    const payload = contractCalledEvent.attributes.find(attr => attr.key === 'payload')?.value;
    
    validateParameters({ isString: { messageId, payload } });

    printInfo('Message ID', messageId);
    printInfo('Payload', payload);
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
        mainProcessor(deployRemoteToken, options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
