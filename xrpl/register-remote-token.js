'use strict';

const { Command, Option } = require('commander');
const { addAmplifierOptions, addChainNameOption } = require('../cosmwasm/cli-utils');
const { executeTransaction: executeCosmosTransaction } = require('../cosmwasm/utils');
const { printInfo, printError } = require('../common');
const { mainCosmosProcessor } = require('./utils');

const registerRemoteToken = async (config, options, wallet, client, fee) => {
    const { chainName, tokenId, currency } = options;
    const [account] = await wallet.getAccounts();

    const xrplGateway = config.axelar.contracts.XrplGateway[chainName];
    if (!xrplGateway) {
        printError(`No XRPLGateway contract found on chain ${chainName}`);
        process.exit(1);
    }

    const execMsg = {
        register_remote_token: {
            token_id: tokenId,
            xrpl_currency: currency,
        },
    };

    const { transactionHash } = await executeCosmosTransaction(client, account, xrplGateway.address, execMsg, fee);

    printInfo('Registered remote token', transactionHash);
};

const programHandler = () => {
    const program = new Command();

    program
        .name('register-remote-token')
        .description('Register a token originating from a remote chain for XRPL support.')
        .addOption(new Option('--tokenId <tokenId>', 'token ID of token to register').makeOptionMandatory(true))
        .addOption(
            new Option('--currency <currency>', 'XRPL currency to use for the wrapped version of the token on XRPL').makeOptionMandatory(
                true,
            ),
        );

    addChainNameOption(program);
    addAmplifierOptions(program, {
        contractOptions: false,
    });

    program.action((options) => {
        mainCosmosProcessor(registerRemoteToken, options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
