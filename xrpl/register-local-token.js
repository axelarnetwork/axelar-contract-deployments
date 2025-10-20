'use strict';

const { Command, Option } = require('commander');
const { addAmplifierOptions, addChainNameOption } = require('../cosmwasm/cli-utils');
const { executeTransaction } = require('../cosmwasm/utils');
const { printInfo, printError } = require('../common');
const { mainProcessor } = require('../cosmwasm/processor');

const registerLocalToken = async (client, config, options, args, fee) => {
    const { chainName, issuer, currency } = options;

    const xrplGateway = config.axelar.contracts.XrplGateway[chainName];
    if (!xrplGateway) {
        printError(`No XRPLGateway contract found on chain ${chainName}`);
        process.exit(1);
    }

    const execMsg = {
        register_local_token: {
            xrpl_token: {
                issuer,
                currency,
            },
        },
    };

    const { transactionHash } = await executeTransaction(client, xrplGateway.address, execMsg, fee);

    printInfo('Registered local token', transactionHash);
};

const programHandler = () => {
    const program = new Command();

    program
        .name('register-local-token')
        .description('Register an XRPL IOU token.')
        .addOption(new Option('--issuer <issuer>', 'XRPL address of the token issuer').makeOptionMandatory(true))
        .addOption(new Option('--currency <currency>', 'XRPL currency of the token').makeOptionMandatory(true));

    addChainNameOption(program);
    addAmplifierOptions(program, {
        contractOptions: false,
    });

    program.action((options) => {
        mainProcessor(registerLocalToken, options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
