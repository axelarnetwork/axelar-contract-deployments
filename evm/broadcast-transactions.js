'use strict';

const chalk = require('chalk');
const { Command, Option } = require('commander');
const { ethers } = require('hardhat');
const {
    providers: { getDefaultProvider },
} = ethers;

const { printError, printInfo, printWarn, getConfigByChainId, prompt } = require('./utils');
const { loadConfig } = require('../common');
const { addBaseOptions } = require('../common');
const { sendTransaction, getSignedTx, storeSignedTx } = require('./sign-utils');

async function processCommand(config, _, options, file) {
    const { rpc } = options;

    const transaction = await getSignedTx(file);
    const parsedTx = ethers.utils.parseTransaction(transaction.signedTx);

    const chain = getConfigByChainId(parsedTx.chainId, config);

    const provider = getDefaultProvider(rpc || chain.rpc);

    if (parsedTx.chainId !== transaction.unsignedTx.chainId) {
        printError(
            `ChainId mismatch: signed tx chain id ${parsedTx.chainId} doesn't match unsigned tx chain id ${transaction.unsignedTx.chainId}`,
        );
        return;
    }

    if (
        prompt(
            `Proceed with the broadcasting of all pending signed transactions for file ${chalk.green(file)} on chain ${chalk.green(
                chain.name,
            )}`,
            options.yes,
        )
    ) {
        return;
    }

    if (transaction.status !== 'SUCCESS') {
        printInfo('Broadcasting transaction', JSON.stringify(transaction.unsignedTx, null, 2));

        // Send the signed transaction
        try {
            const { response, receipt } = await sendTransaction(transaction.signedTx, provider, chain.confirmations);
            transaction.status = 'SUCCESS';
            transaction.hash = response.hash;
            printInfo('Tx Receipt', JSON.stringify(receipt, null, 2));
        } catch (error) {
            transaction.status = 'FAILED';
            printError('Error broadcasting tx', error);
        }

        storeSignedTx(file, transaction);
    } else {
        printWarn('Skipping broadcast, transaction status is', transaction.status);
    }
}

async function main(options) {
    const config = loadConfig(options.env);
    const { files } = options;

    if (!files || files.length === 0) {
        throw new Error('FilePath is not provided in user info');
    }

    for (const file of files) {
        await processCommand(config, null, options, file);
    }
}

if (require.main === module) {
    const program = new Command();

    program.name('broadcast-transactions').description('Broadcast all the pending signed transactions of the signer');

    program.addOption(new Option('--files [files...]', 'The file where the signed tx are stored').makeOptionMandatory(true));
    addBaseOptions(program, { ignoreChainNames: true });
    program.addOption(new Option('-r, --rpc <rpc>', 'The chain rpc'));

    program.action((options) => {
        main(options);
    });

    program.parse();
}
