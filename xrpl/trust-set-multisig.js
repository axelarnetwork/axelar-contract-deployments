'use strict';

const { Command, Option } = require('commander');
const { addAmplifierOptions, addChainNameOption } = require('../cosmwasm/cli-utils');
const { printInfo, printError } = require('../common');
const { executeTransaction: executeCosmosTransaction } = require('../cosmwasm/utils');
const { mainCosmosProcessor } = require('./utils');

const trustSet = async (config, options, wallet, client, fee) => {
    const { chainName, tokenId } = options;
    const [account] = await wallet.getAccounts();

    const xrplMultisigProver = config.axelar.contracts.XrplMultisigProver[chainName];
    if (!xrplMultisigProver) {
        printError(`No XRPLMultisigProver contract found on chain ${chainName}`);
        process.exit(1);
    }

    const execMsg = {
        trust_set: {
            token_id: tokenId,
        },
    };

    const { transactionHash, events } = await executeCosmosTransaction(client, account, xrplMultisigProver.address, execMsg, fee);

    printInfo('Creating trust line between token and multisig', transactionHash);
    const multisigSessionId = events
        .find((e) => e.type === 'wasm-proof_under_construction')
        .attributes.find((a) => a.key === 'multisig_session_id').value;
    printInfo('Mutisig session ID', multisigSessionId);
};

const programHandler = () => {
    const program = new Command();

    program
        .name('trust-set-multisig')
        .description('Create a trust line between a token and the multisig.')
        .addOption(new Option('--tokenId <tokenId>', 'token ID of token to create trust line for').makeOptionMandatory(true));

    addChainNameOption(program);
    addAmplifierOptions(program, {
        contractOptions: false,
    });

    program.action((options) => {
        mainCosmosProcessor(trustSet, options);
    });

    program.parse();
};

if (require.main === module) {
    programHandler();
}
