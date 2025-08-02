'use strict';

const { Command, Option } = require('commander');
const { addAmplifierOptions, addChainNameOption } = require('../cosmwasm/cli-utils');
const { executeTransaction: executeCosmosTransaction } = require('../cosmwasm/utils');
const { printInfo, printError } = require('../common');
const { mainCosmosProcessor } = require('./utils');

const registerLocalToken = async (config, options, wallet, client, fee) => {
    const { chainName, issuer, currency } = options;
    const [account] = await wallet.getAccounts();

    const xrplGateway = config.axelar.contracts.XrplGateway[chainName];
    if (!xrplGateway) {
        printError(`No XRPLGateway contract found on chain ${chainName}`);
        process.exit(1);
    }

    const registerLocalToken = {
        register_local_token: {
            xrpl_token: {
                issuer,
                currency,
            },
        },
    };

    const { transactionHash } = await executeCosmosTransaction(client, account, xrplGateway.address, registerLocalToken, fee);

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
        mainCosmosProcessor(registerLocalToken, options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
