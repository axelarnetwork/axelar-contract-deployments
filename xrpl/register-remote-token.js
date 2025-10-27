'use strict';

const { Command, Option } = require('commander');
const { addAmplifierOptions, addChainNameOption } = require('../cosmwasm/cli-utils');
const { executeTransaction } = require('../cosmwasm/utils');
const { printInfo, printError } = require('../common');
const { mainProcessor } = require('../cosmwasm/processor');

const registerRemoteToken = async (client, config, options, args, fee) => {
    const { chainName, tokenId, currency } = options;

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

    const { transactionHash } = await executeTransaction(client, xrplGateway.address, execMsg, fee);

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
        mainProcessor(registerRemoteToken, options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
