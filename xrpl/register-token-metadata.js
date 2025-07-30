'use strict';

const { Command, Option } = require('commander');
const { addAmplifierOptions, addChainNameOption } = require('../cosmwasm/cli-utils');
const { executeTransaction: executeCosmosTransaction } = require('../cosmwasm/utils');
const { printInfo, printError } = require('../common');
const { mainCosmosProcessor } = require('./utils');
const { contract } = require('@stellar/stellar-sdk');

const registerTokenMetadata = async (config, options, wallet, client, fee) => {
    const { chainName, issuer, currency } = options;
    const [account] = await wallet.getAccounts();

    const xrplGateway = config.axelar.contracts.XrplGateway[chainName];
    if (!xrplGateway) {
        printError(`No XRPLGateway contract found on chain ${chainName}`);
        process.exit(1);
    }

    const registerTokenMetadata = {
        register_token_metadata: {
            xrpl_token: {
                issued: {
                    issuer,
                    currency,
                },
            },
        },
    };

    const { transactionHash, events } = await executeCosmosTransaction(client, account, xrplGateway.address, registerTokenMetadata, fee);

    printInfo('Initiated token metadata registration', transactionHash);

    const contractCalledEvent = events.find((e) => e.type === 'wasm-contract_called');
    const messageId = contractCalledEvent.attributes.find((a) => a.key === 'message_id').value;
    const payload = contractCalledEvent.attributes.find((a) => a.key === 'payload').value;

    const tokenMetadataRegisteredEvent = events.find((e) => e.type === 'wasm-token_metadata_registered');
    const tokenAddress = tokenMetadataRegisteredEvent.attributes.find((a) => a.key === 'token_address').value;

    printInfo('Message ID', messageId);
    printInfo('Payload', payload);
    printInfo('Token address', tokenAddress);
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
        mainCosmosProcessor(registerTokenMetadata, options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
