'use strict';

const { Command, Option } = require('commander');
const { addAmplifierOptions, addChainNameOption } = require('../cosmwasm/cli-utils');
const { executeTransaction: executeCosmosTransaction } = require('../cosmwasm/utils');
const { printInfo, printError } = require('../common');
const { mainCosmosProcessor } = require('./utils');

const registerTokenInstance = async (config, options, wallet, client, fee) => {
    const { chainName, tokenId, sourceChain, decimals } = options;
    const [account] = await wallet.getAccounts();

    const xrplGateway = config.axelar.contracts.XrplGateway[chainName];
    if (!xrplGateway) {
        printError(`No XRPLGateway contract found on chain ${chainName}`);
        process.exit(1);
    }

    const execMsg = {
        register_token_instance: {
            token_id: tokenId,
            chain: sourceChain,
            decimals: parseInt(decimals, 10),
        },
    };

    const { transactionHash } = await executeCosmosTransaction(client, account, xrplGateway.address, execMsg, fee);

    printInfo('Registered token instance', transactionHash);
};

const programHandler = () => {
    const program = new Command();

    program
        .name('register-token-instance')
        .description('Register a token instance.')
        .addOption(new Option('--tokenId <tokenId>', 'token ID to register instance for').makeOptionMandatory(true))
        .addOption(new Option('--sourceChain <sourceChain>', 'chain to register token instance for').makeOptionMandatory(true))
        .addOption(new Option('--decimals <decimals>', 'token decimals on the given chain').makeOptionMandatory(true));

    addChainNameOption(program);
    addAmplifierOptions(program, {
        contractOptions: false,
    });

    program.action((options) => {
        mainCosmosProcessor(registerTokenInstance, options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
